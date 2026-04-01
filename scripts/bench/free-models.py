#!/usr/bin/env python3
"""
OpenRouter Free Model Benchmark
================================
Fetches all free models from OpenRouter, runs 8 test prompts per model,
and measures 5 metrics: TTFB, total latency, tokens/sec, accuracy, and
instruction compliance.

Usage:
    OPENROUTER_API_KEY=sk-or-... python3 scripts/bench/free-models.py
    OPENROUTER_API_KEY=sk-or-... python3 scripts/bench/free-models.py --models 5
    OPENROUTER_API_KEY=sk-or-... python3 scripts/bench/free-models.py --filter "qwen,llama"
    OPENROUTER_API_KEY=sk-or-... python3 scripts/bench/free-models.py --output results.json

Requires: requests  (pip install requests)
"""

import argparse
import json
import os
import re
import statistics
import sys
import time
from dataclasses import dataclass, field, asdict
from typing import Optional

try:
    import requests
except ImportError:
    print("ERROR: 'requests' not installed. Run: pip install requests", file=sys.stderr)
    sys.exit(1)

# ── Config ──────────────────────────────────────────────────────────────

API_BASE = "https://openrouter.ai/api/v1"
TIMEOUT_SECS = 60
MAX_TOKENS = 256
TEMPERATURE = 0.0  # deterministic for accuracy testing

# ── Test Suite ──────────────────────────────────────────────────────────

@dataclass
class TestCase:
    name: str
    category: str  # qa, reasoning, json, code, instruction, math, summary, knowledge
    system: Optional[str]
    prompt: str
    max_tokens: int
    # Accuracy checker: returns (correct: bool, detail: str)
    check: object  # callable

def check_contains(target: str, case_insensitive: bool = True):
    """Check if response contains a target string."""
    def checker(text: str) -> tuple:
        t = text.lower() if case_insensitive else text
        s = target.lower() if case_insensitive else target
        ok = s in t
        return (ok, f"{'found' if ok else 'missing'} '{target}'")
    return checker

def check_json_valid(text: str) -> tuple:
    """Check if response contains valid JSON."""
    # Try to find JSON in the response
    # Look for {} blocks
    match = re.search(r'\{[^{}]*\}', text, re.DOTALL)
    if not match:
        # Try multiline
        match = re.search(r'\{.*?\}', text, re.DOTALL)
    if match:
        try:
            obj = json.loads(match.group())
            has_keys = isinstance(obj, dict) and len(obj) >= 2
            return (has_keys, f"valid JSON with {len(obj)} keys")
        except json.JSONDecodeError:
            pass
    return (False, "no valid JSON object found")

def check_code_function(text: str) -> tuple:
    """Check if response contains a Python function definition."""
    has_def = "def " in text
    has_fib = any(w in text.lower() for w in ["fib", "fibonacci"])
    has_return = "return" in text
    ok = has_def and has_return
    detail = f"def={'y' if has_def else 'n'} fib={'y' if has_fib else 'n'} return={'y' if has_return else 'n'}"
    return (ok, detail)

def check_one_word_blue(text: str) -> tuple:
    """Check if response is approximately one word and mentions blue."""
    clean = text.strip().rstrip(".!,").strip()
    words = clean.split()
    is_short = len(words) <= 3
    has_blue = "blue" in clean.lower()
    ok = is_short and has_blue
    return (ok, f"words={len(words)} blue={'y' if has_blue else 'n'}")

def check_short_summary(text: str) -> tuple:
    """Check if response is a concise summary (under 200 chars)."""
    clean = text.strip()
    is_short = len(clean) < 200
    is_nonempty = len(clean) > 10
    ok = is_short and is_nonempty
    return (ok, f"len={len(clean)}")

def check_math_391(text: str) -> tuple:
    """Check if response contains 391."""
    ok = "391" in text
    return (ok, f"{'found' if ok else 'missing'} 391")

TESTS: list[TestCase] = [
    TestCase(
        name="simple_qa",
        category="qa",
        system="You are a helpful assistant. Be concise.",
        prompt="What is 2 + 2?",
        max_tokens=32,
        check=check_contains("4"),
    ),
    TestCase(
        name="reasoning",
        category="reasoning",
        system="You are a logic expert. Answer precisely.",
        prompt=(
            "If all roses are flowers, and some flowers fade quickly, "
            "can we definitively conclude that all roses fade quickly? "
            "Answer yes or no and explain in one sentence."
        ),
        max_tokens=100,
        check=check_contains("no"),
    ),
    TestCase(
        name="json_output",
        category="json",
        system="You are an API. Respond ONLY with valid JSON, no markdown.",
        prompt='Return a JSON object with exactly these keys: "name", "age", "city". Use any values.',
        max_tokens=100,
        check=check_json_valid,
    ),
    TestCase(
        name="code_gen",
        category="code",
        system="You are a Python expert. Write only code, no explanations.",
        prompt="Write a Python function that returns the nth Fibonacci number. Use 0-indexed.",
        max_tokens=200,
        check=check_code_function,
    ),
    TestCase(
        name="instruction_follow",
        category="instruction",
        system="Follow instructions exactly.",
        prompt="Reply with exactly one word: the color of a clear daytime sky.",
        max_tokens=16,
        check=check_one_word_blue,
    ),
    TestCase(
        name="math",
        category="math",
        system="You are a calculator. Give the numerical answer.",
        prompt="What is 17 multiplied by 23?",
        max_tokens=32,
        check=check_math_391,
    ),
    TestCase(
        name="summarize",
        category="summary",
        system="You summarize text concisely in one sentence.",
        prompt=(
            "Summarize this in one sentence: The mitochondria are membrane-bound "
            "organelles found in the cytoplasm of eukaryotic cells. They generate "
            "most of the cell's supply of adenosine triphosphate (ATP), which is "
            "used as a source of chemical energy. They were first discovered by "
            "Albert von Kolliker in 1857."
        ),
        max_tokens=80,
        check=check_short_summary,
    ),
    TestCase(
        name="knowledge",
        category="knowledge",
        system="You are a geography expert. Be concise.",
        prompt="What is the capital of Australia?",
        max_tokens=32,
        check=check_contains("canberra"),
    ),
]

# ── Data Types ──────────────────────────────────────────────────────────

@dataclass
class TestResult:
    test_name: str
    ttfb_ms: float          # time to first byte (streaming)
    total_ms: float         # total response time
    output_tokens: int
    tokens_per_sec: float
    correct: bool
    detail: str
    response_text: str
    error: Optional[str] = None

@dataclass
class ModelResult:
    model_id: str
    model_name: str
    context_length: int
    tests: list = field(default_factory=list)
    # Aggregates (computed after all tests)
    avg_ttfb_ms: float = 0.0
    avg_total_ms: float = 0.0
    avg_tokens_per_sec: float = 0.0
    accuracy_pct: float = 0.0
    instruction_compliance_pct: float = 0.0
    error_rate_pct: float = 0.0
    score: float = 0.0  # composite ranking score

# ── API Calls ───────────────────────────────────────────────────────────

def fetch_free_models(api_key: str) -> list[dict]:
    """Fetch all free models from OpenRouter."""
    resp = requests.get(
        f"{API_BASE}/models",
        headers={"Authorization": f"Bearer {api_key}"},
        timeout=30,
    )
    resp.raise_for_status()
    data = resp.json()
    free = []
    for m in data.get("data", []):
        pricing = m.get("pricing", {})
        if pricing.get("prompt") == "0" and pricing.get("completion") == "0":
            free.append({
                "id": m["id"],
                "name": m.get("name", m["id"]),
                "context_length": m.get("context_length", 0),
            })
    # Sort by context length descending
    free.sort(key=lambda x: x["context_length"], reverse=True)
    return free


def call_model_streaming(
    api_key: str,
    model_id: str,
    system_msg: Optional[str],
    user_msg: str,
    max_tokens: int,
) -> tuple:
    """
    Call a model via OpenRouter streaming API.
    Returns (response_text, ttfb_ms, total_ms, output_tokens, error).
    """
    messages = []
    if system_msg:
        messages.append({"role": "system", "content": system_msg})
    messages.append({"role": "user", "content": user_msg})

    payload = {
        "model": model_id,
        "messages": messages,
        "max_tokens": max_tokens,
        "temperature": TEMPERATURE,
        "stream": True,
    }

    headers = {
        "Authorization": f"Bearer {api_key}",
        "Content-Type": "application/json",
        "Accept": "text/event-stream",
        "HTTP-Referer": "https://github.com/ruvnet/clawft",
        "X-Title": "clawft-bench",
    }

    t_start = time.monotonic()
    ttfb = None
    text_parts = []
    output_tokens = 0
    error = None

    try:
        resp = requests.post(
            f"{API_BASE}/chat/completions",
            json=payload,
            headers=headers,
            stream=True,
            timeout=TIMEOUT_SECS,
        )

        if resp.status_code != 200:
            body = resp.text
            t_end = time.monotonic()
            return ("", 0, (t_end - t_start) * 1000, 0, f"HTTP {resp.status_code}: {body[:200]}")

        for line in resp.iter_lines(decode_unicode=True):
            if ttfb is None and line:
                ttfb = (time.monotonic() - t_start) * 1000

            if not line or not line.startswith("data: "):
                continue

            data_str = line[6:].strip()
            if data_str == "[DONE]":
                break

            try:
                chunk = json.loads(data_str)
            except json.JSONDecodeError:
                continue

            # Extract text content
            choices = chunk.get("choices", [])
            if choices:
                delta = choices[0].get("delta", {})
                content = delta.get("content")
                if content:
                    text_parts.append(content)

            # Extract usage from final chunk
            usage = chunk.get("usage")
            if usage:
                output_tokens = usage.get("completion_tokens", 0) or usage.get("output_tokens", 0)

    except requests.exceptions.Timeout:
        t_end = time.monotonic()
        return ("", 0, (t_end - t_start) * 1000, 0, "timeout")
    except requests.exceptions.ConnectionError as e:
        t_end = time.monotonic()
        return ("", 0, (t_end - t_start) * 1000, 0, f"connection error: {e}")
    except Exception as e:
        t_end = time.monotonic()
        return ("", 0, (t_end - t_start) * 1000, 0, str(e))

    t_end = time.monotonic()
    total_ms = (t_end - t_start) * 1000

    if ttfb is None:
        ttfb = total_ms

    full_text = "".join(text_parts)

    # Estimate tokens if usage not provided
    if output_tokens == 0 and full_text:
        output_tokens = max(1, len(full_text.split()) * 4 // 3)  # rough ~1.33 tokens/word

    return (full_text, ttfb, total_ms, output_tokens, error)


def call_model_non_streaming(
    api_key: str,
    model_id: str,
    system_msg: Optional[str],
    user_msg: str,
    max_tokens: int,
) -> tuple:
    """
    Fallback: call model without streaming.
    Returns (response_text, ttfb_ms, total_ms, output_tokens, error).
    """
    messages = []
    if system_msg:
        messages.append({"role": "system", "content": system_msg})
    messages.append({"role": "user", "content": user_msg})

    payload = {
        "model": model_id,
        "messages": messages,
        "max_tokens": max_tokens,
        "temperature": TEMPERATURE,
    }

    headers = {
        "Authorization": f"Bearer {api_key}",
        "Content-Type": "application/json",
        "HTTP-Referer": "https://github.com/ruvnet/clawft",
        "X-Title": "clawft-bench",
    }

    t_start = time.monotonic()
    try:
        resp = requests.post(
            f"{API_BASE}/chat/completions",
            json=payload,
            headers=headers,
            timeout=TIMEOUT_SECS,
        )
        t_end = time.monotonic()
        total_ms = (t_end - t_start) * 1000

        if resp.status_code != 200:
            return ("", total_ms, total_ms, 0, f"HTTP {resp.status_code}: {resp.text[:200]}")

        data = resp.json()
        choices = data.get("choices", [])
        text = ""
        if choices:
            text = choices[0].get("message", {}).get("content", "") or ""

        usage = data.get("usage", {})
        output_tokens = usage.get("completion_tokens", 0) or usage.get("output_tokens", 0)
        if output_tokens == 0 and text:
            output_tokens = max(1, len(text.split()) * 4 // 3)

        return (text, total_ms, total_ms, output_tokens, None)

    except requests.exceptions.Timeout:
        t_end = time.monotonic()
        return ("", 0, (t_end - t_start) * 1000, 0, "timeout")
    except Exception as e:
        t_end = time.monotonic()
        return ("", 0, (t_end - t_start) * 1000, 0, str(e))


# ── Runner ──────────────────────────────────────────────────────────────

def run_test(api_key: str, model_id: str, test: TestCase) -> TestResult:
    """Run a single test case against a model."""
    text, ttfb, total_ms, output_tokens, error = call_model_streaming(
        api_key, model_id, test.system, test.prompt, test.max_tokens,
    )

    # Fallback to non-streaming if streaming returned empty (some models don't stream well)
    if not text and not error:
        text, ttfb, total_ms, output_tokens, error = call_model_non_streaming(
            api_key, model_id, test.system, test.prompt, test.max_tokens,
        )

    tokens_per_sec = 0.0
    if total_ms > 0 and output_tokens > 0:
        tokens_per_sec = output_tokens / (total_ms / 1000.0)

    correct = False
    detail = ""
    if error:
        detail = f"error: {error}"
    elif text:
        correct, detail = test.check(text)
    else:
        detail = "empty response"

    return TestResult(
        test_name=test.name,
        ttfb_ms=round(ttfb, 1),
        total_ms=round(total_ms, 1),
        output_tokens=output_tokens,
        tokens_per_sec=round(tokens_per_sec, 1),
        correct=correct,
        detail=detail,
        response_text=text[:300],  # truncate for storage
        error=error,
    )


def bench_model(api_key: str, model: dict, tests: list[TestCase], quiet: bool = False) -> ModelResult:
    """Benchmark a single model across all tests."""
    result = ModelResult(
        model_id=model["id"],
        model_name=model["name"],
        context_length=model["context_length"],
    )

    for i, test in enumerate(tests):
        if not quiet:
            print(f"    [{i+1}/{len(tests)}] {test.name}...", end=" ", flush=True)

        tr = run_test(api_key, model["id"], test)
        result.tests.append(tr)

        if not quiet:
            status = "PASS" if tr.correct else ("ERR" if tr.error else "FAIL")
            print(f"{status} ({tr.total_ms:.0f}ms, {tr.tokens_per_sec:.0f} t/s) {tr.detail}")

        # Rate limit courtesy — 500ms between calls
        time.sleep(0.5)

    # Compute aggregates
    successful = [t for t in result.tests if not t.error]
    all_tests = result.tests

    if successful:
        result.avg_ttfb_ms = round(statistics.mean(t.ttfb_ms for t in successful), 1)
        result.avg_total_ms = round(statistics.mean(t.total_ms for t in successful), 1)
        tps_vals = [t.tokens_per_sec for t in successful if t.tokens_per_sec > 0]
        result.avg_tokens_per_sec = round(statistics.mean(tps_vals), 1) if tps_vals else 0.0

    result.accuracy_pct = round(
        100 * sum(1 for t in all_tests if t.correct) / len(all_tests), 1
    ) if all_tests else 0.0

    # Instruction compliance = correct on instruction-sensitive tests
    instruction_tests = [t for t in all_tests if t.test_name in ("instruction_follow", "json_output", "summarize")]
    if instruction_tests:
        result.instruction_compliance_pct = round(
            100 * sum(1 for t in instruction_tests if t.correct) / len(instruction_tests), 1
        )

    result.error_rate_pct = round(
        100 * sum(1 for t in all_tests if t.error) / len(all_tests), 1
    ) if all_tests else 100.0

    # Composite score: weighted combination (higher = better)
    # accuracy: 40%, speed: 25%, instruction: 20%, low errors: 15%
    speed_score = min(100, result.avg_tokens_per_sec / 1.0) if result.avg_tokens_per_sec > 0 else 0
    result.score = round(
        result.accuracy_pct * 0.40
        + speed_score * 0.25
        + result.instruction_compliance_pct * 0.20
        + (100 - result.error_rate_pct) * 0.15,
        1,
    )

    return result


# ── Display ─────────────────────────────────────────────────────────────

def print_leaderboard(results: list[ModelResult]):
    """Print a ranked leaderboard table."""
    results.sort(key=lambda r: r.score, reverse=True)

    print("\n" + "=" * 110)
    print(f"{'#':>3}  {'Model':<45} {'Acc%':>5} {'TTFB':>7} {'Lat':>7} {'T/s':>6} {'Instr%':>6} {'Err%':>5} {'Score':>6}")
    print("-" * 110)

    for i, r in enumerate(results, 1):
        name = r.model_name if len(r.model_name) <= 44 else r.model_name[:41] + "..."
        print(
            f"{i:>3}  {name:<45} {r.accuracy_pct:>5.1f} "
            f"{r.avg_ttfb_ms:>6.0f}ms {r.avg_total_ms:>6.0f}ms "
            f"{r.avg_tokens_per_sec:>5.0f} {r.instruction_compliance_pct:>5.1f} "
            f"{r.error_rate_pct:>5.1f} {r.score:>6.1f}"
        )

    print("=" * 110)
    print()
    print("Metrics:")
    print("  Acc%   = Accuracy (correct answers / total tests)")
    print("  TTFB   = Time to first byte (streaming latency)")
    print("  Lat    = Total response latency (avg)")
    print("  T/s    = Output tokens per second (throughput)")
    print("  Instr% = Instruction compliance (JSON/format/length tests)")
    print("  Err%   = Error rate (timeouts, HTTP errors)")
    print("  Score  = Composite (40% accuracy + 25% speed + 20% instruction + 15% reliability)")
    print()


def print_model_detail(r: ModelResult):
    """Print detailed per-test results for a model."""
    print(f"\n  Detail: {r.model_name}")
    print(f"  {'Test':<20} {'Status':>6} {'TTFB':>7} {'Total':>7} {'T/s':>6} {'Detail'}")
    print(f"  {'-'*80}")
    for t in r.tests:
        status = "PASS" if t.correct else ("ERR" if t.error else "FAIL")
        print(
            f"  {t.test_name:<20} {status:>6} {t.ttfb_ms:>6.0f}ms "
            f"{t.total_ms:>6.0f}ms {t.tokens_per_sec:>5.0f} {t.detail}"
        )


def print_task_leaderboards(results: list[ModelResult]):
    """Print per-task category leaderboards — best model for each task type."""
    # Collect all unique test names
    test_names = []
    for t in TESTS:
        if t.name not in test_names:
            test_names.append(t.name)

    # Map category labels
    cat_labels = {
        "simple_qa": "Simple Q&A",
        "reasoning": "Reasoning",
        "json_output": "JSON / Structured Output",
        "code_gen": "Code Generation",
        "instruction_follow": "Instruction Following",
        "math": "Math",
        "summarize": "Summarization",
        "knowledge": "Knowledge Recall",
    }

    print("\n" + "=" * 90)
    print("  PER-TASK LEADERBOARDS — Best model for each task category")
    print("=" * 90)

    for tname in test_names:
        label = cat_labels.get(tname, tname)

        # Gather (model_name, test_result) for this task
        entries = []
        for r in results:
            tr = next((t for t in r.tests if t.test_name == tname), None)
            if tr:
                entries.append((r.model_name, r.model_id, tr))

        # Sort: correct first, then by latency (fastest wins)
        entries.sort(key=lambda e: (not e[2].correct, e[2].total_ms))

        print(f"\n  {label} ({tname})")
        print(f"  {'#':>3}  {'Model':<40} {'Pass':>4} {'Latency':>8} {'T/s':>6} {'Detail'}")
        print(f"  {'-'*85}")

        for rank, (name, mid, tr) in enumerate(entries[:10], 1):
            short = name if len(name) <= 39 else name[:36] + "..."
            status = "Y" if tr.correct else ("ERR" if tr.error else "N")
            print(
                f"  {rank:>3}  {short:<40} {status:>4} "
                f"{tr.total_ms:>6.0f}ms {tr.tokens_per_sec:>5.0f} {tr.detail}"
            )

    # Summary: best-in-class for each category
    print(f"\n  {'─'*60}")
    print(f"  BEST-IN-CLASS SUMMARY")
    print(f"  {'Task':<25} {'Best Model':<40} {'Latency':>8}")
    print(f"  {'─'*60}")

    for tname in test_names:
        label = cat_labels.get(tname, tname)
        entries = []
        for r in results:
            tr = next((t for t in r.tests if t.test_name == tname), None)
            if tr and tr.correct:
                entries.append((r.model_name, tr))

        if entries:
            entries.sort(key=lambda e: e[1].total_ms)
            best_name = entries[0][0]
            if len(best_name) > 39:
                best_name = best_name[:36] + "..."
            print(f"  {label:<25} {best_name:<40} {entries[0][1].total_ms:>6.0f}ms")
        else:
            print(f"  {label:<25} {'(no model passed)':>40}")

    print()


# ── Main ────────────────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(description="Benchmark free OpenRouter models")
    parser.add_argument("--models", type=int, default=0, help="Max models to test (0=all)")
    parser.add_argument("--filter", type=str, default="", help="Comma-separated model name filters")
    parser.add_argument("--output", type=str, default="", help="Save results to JSON file")
    parser.add_argument("--detail", action="store_true", help="Show per-test detail for each model")
    parser.add_argument("--exclude-vl", action="store_true", help="Exclude vision/VL models")
    parser.add_argument("--skip-router", action="store_true", default=True,
                        help="Skip 'openrouter/free' meta-router (default: true)")
    parser.add_argument("--no-skip-router", action="store_false", dest="skip_router")
    parser.add_argument("--by-task", action="store_true",
                        help="Show per-task leaderboards (best model per category)")
    args = parser.parse_args()

    api_key = os.environ.get("OPENROUTER_API_KEY")
    if not api_key:
        print("ERROR: OPENROUTER_API_KEY not set", file=sys.stderr)
        sys.exit(1)

    print("Fetching free models from OpenRouter...")
    models = fetch_free_models(api_key)
    print(f"Found {len(models)} free models\n")

    # Apply filters
    if args.skip_router:
        models = [m for m in models if m["id"] != "openrouter/free"]

    if args.exclude_vl:
        models = [m for m in models if "-vl" not in m["id"].lower() and "vision" not in m["name"].lower()]

    if args.filter:
        filters = [f.strip().lower() for f in args.filter.split(",")]
        models = [m for m in models if any(f in m["id"].lower() or f in m["name"].lower() for f in filters)]

    if args.models > 0:
        models = models[:args.models]

    if not models:
        print("No models matched filters.", file=sys.stderr)
        sys.exit(1)

    print(f"Testing {len(models)} models x {len(TESTS)} tests = {len(models) * len(TESTS)} API calls")
    print(f"Estimated time: ~{len(models) * len(TESTS) * 5 // 60} - {len(models) * len(TESTS) * 10 // 60} minutes")
    print()

    results: list[ModelResult] = []

    for i, model in enumerate(models, 1):
        print(f"[{i}/{len(models)}] {model['name']} ({model['id']})")
        result = bench_model(api_key, model, TESTS)
        results.append(result)

        if args.detail:
            print_model_detail(result)

        print(f"  -> Accuracy: {result.accuracy_pct}% | TTFB: {result.avg_ttfb_ms:.0f}ms | "
              f"T/s: {result.avg_tokens_per_sec:.0f} | Score: {result.score}")
        print()

    # Leaderboard
    print_leaderboard(results)

    # Per-task leaderboards
    if args.by_task:
        print_task_leaderboards(results)

    # Save results
    if args.output:
        # Build per-task rankings for agent consumption
        task_rankings = {}
        for t in TESTS:
            entries = []
            for r in results:
                tr = next((x for x in r.tests if x.test_name == t.name), None)
                if tr:
                    entries.append({
                        "model_id": r.model_id,
                        "model_name": r.model_name,
                        "correct": tr.correct,
                        "latency_ms": tr.total_ms,
                        "tokens_per_sec": tr.tokens_per_sec,
                    })
            entries.sort(key=lambda e: (not e["correct"], e["latency_ms"]))
            task_rankings[t.name] = {
                "category": t.category,
                "rankings": entries,
                "best": entries[0]["model_id"] if entries and entries[0]["correct"] else None,
            }

        output_data = {
            "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
            "models_tested": len(results),
            "tests_per_model": len(TESTS),
            "results": [asdict(r) for r in results],
            "task_rankings": task_rankings,
        }
        # Sort by score for the JSON output too
        output_data["results"].sort(key=lambda r: r["score"], reverse=True)

        with open(args.output, "w") as f:
            json.dump(output_data, f, indent=2)
        print(f"Results saved to {args.output}")


if __name__ == "__main__":
    main()
