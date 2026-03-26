#!/usr/bin/env python3
"""Parse git log (piped from stdin) into ECC CausalGraph JSON."""

import json
import re
import sys
from collections import defaultdict
from datetime import datetime, timezone


# ---------------------------------------------------------------------------
# Tag rules
# ---------------------------------------------------------------------------

SUBJECT_TAG_RULES: list[tuple[re.Pattern, str]] = [
    (re.compile(r"(?i)\bK6\b|mesh", re.I), "mesh-networking"),
    (re.compile(r"(?i)\bK3\b|wasm|sandbox", re.I), "wasm-sandbox"),
    (re.compile(r"(?i)\bK2\b|container", re.I), "container-runtime"),
    (re.compile(r"(?i)\bK1\b|process|supervisor", re.I), "process-mgmt"),
    (re.compile(r"(?i)\bK0\b|exochain|chain", re.I), "exochain"),
    (re.compile(r"(?i)\bK4\b|service|daemon", re.I), "service-framework"),
    (re.compile(r"(?i)\bK5\b|app|lifecycle", re.I), "app-lifecycle"),
    (re.compile(r"(?i)\bECC\b|ecc|cognitive|weaver", re.I), "ecc"),
    (re.compile(r"(?i)\bdoc", re.I), "documentation"),
    (re.compile(r"(?i)governance|constitution|genesis.rule", re.I), "governance"),
    (re.compile(r"(?i)test|spec", re.I), "testing"),
    (re.compile(r"(?i)agent", re.I), "agents"),
    (re.compile(r"(?i)sparc|plan", re.I), "planning"),
    (re.compile(r"(?i)security|auth|capabilit", re.I), "security"),
]

FILE_TAG_RULES: list[tuple[re.Pattern, str]] = [
    (re.compile(r"mesh[_.]"), "mesh-networking"),
    (re.compile(r"chain\.rs|tree_manager\.rs|exochain"), "exochain"),
    (re.compile(r"wasm|sandbox"), "wasm-sandbox"),
    (re.compile(r"container"), "container-runtime"),
    (re.compile(r"supervisor|process"), "process-mgmt"),
    (re.compile(r"service\.rs|daemon"), "service-framework"),
    (re.compile(r"weaver|ecc"), "ecc"),
    (re.compile(r"governance|genesis"), "governance"),
    (re.compile(r"\.planning/|\.md$"), "documentation"),
    (re.compile(r"agents/"), "agents"),
    (re.compile(r"auth|capability"), "security"),
]

PR_ISSUE_RE = re.compile(r"#(\d+)")


def tag_commit(subject: str, files: list[str]) -> list[str]:
    tags: set[str] = set()
    for pattern, tag in SUBJECT_TAG_RULES:
        if pattern.search(subject):
            tags.add(tag)
    for f in files:
        for pattern, tag in FILE_TAG_RULES:
            if pattern.search(f):
                tags.add(tag)
    return sorted(tags)


def extract_pr_refs(subject: str) -> list[int]:
    return [int(m) for m in PR_ISSUE_RE.findall(subject)]


# ---------------------------------------------------------------------------
# Parse stdin
# ---------------------------------------------------------------------------

def parse_git_log(lines: list[str]) -> list[dict]:
    commits: list[dict] = []
    current = None

    for raw in lines:
        line = raw.rstrip("\n").rstrip("\r")
        if line.startswith("COMMIT|"):
            if current is not None:
                commits.append(current)
            parts = line.split("|", 5)
            current = {
                "hash": parts[1],
                "short": parts[2],
                "author": parts[3],
                "date": parts[4],
                "subject": parts[5] if len(parts) > 5 else "",
                "files": [],
            }
        elif line.strip() and current is not None:
            current["files"].append(line.strip())

    if current is not None:
        commits.append(current)

    return commits


# ---------------------------------------------------------------------------
# Build graph
# ---------------------------------------------------------------------------

def build_graph(commits: list[dict]) -> dict:
    nodes: list[dict] = []
    edges: list[dict] = []
    tag_counts: dict[str, int] = defaultdict(int)
    file_to_commits: dict[str, list[str]] = defaultdict(list)

    # Nodes ------------------------------------------------------------------
    for c in commits:
        tags = tag_commit(c["subject"], c["files"])
        for t in tags:
            tag_counts[t] += 1

        node_id = f"commit:{c['short']}"
        nodes.append({
            "id": node_id,
            "type": "commit",
            "hash": c["hash"],
            "author": c["author"],
            "date": c["date"],
            "subject": c["subject"],
            "files": c["files"],
            "tags": tags,
        })

        for f in c["files"]:
            file_to_commits[f].append(c["short"])

    # PR / issue reference nodes
    pr_nodes: set[str] = set()
    for c in commits:
        for num in extract_pr_refs(c["subject"]):
            pr_id = f"pr:{num}"
            if pr_id not in pr_nodes:
                pr_nodes.add(pr_id)
                nodes.append({
                    "id": pr_id,
                    "type": "pr_reference",
                    "number": num,
                    "tags": [],
                })

    # Edges ------------------------------------------------------------------

    # 1. Follows — consecutive commits (list is newest-first from git log)
    #    Reverse so oldest is first, then edge from earlier to later.
    ordered = list(reversed(commits))
    for i in range(len(ordered) - 1):
        edges.append({
            "from": f"commit:{ordered[i]['short']}",
            "to": f"commit:{ordered[i + 1]['short']}",
            "type": "Follows",
            "weight": 1.0,
        })

    # 2. Enables — commits touching the same file (limit fan-out)
    seen_enables: set[tuple[str, str]] = set()
    for _file, shorthashes in file_to_commits.items():
        # Only link consecutive touches of the same file (keeps edge count sane)
        for i in range(len(shorthashes) - 1):
            a, b = shorthashes[i], shorthashes[i + 1]
            pair = (min(a, b), max(a, b))
            if pair not in seen_enables:
                seen_enables.add(pair)
                edges.append({
                    "from": f"commit:{a}",
                    "to": f"commit:{b}",
                    "type": "Enables",
                    "weight": 0.7,
                })

    # 3. Correlates — commits sharing a tag (limit to nearest 3 per tag)
    tag_to_commits: dict[str, list[str]] = defaultdict(list)
    for c in ordered:
        tags = tag_commit(c["subject"], c["files"])
        for t in tags:
            tag_to_commits[t].append(c["short"])

    seen_correlates: set[tuple[str, str]] = set()
    for _tag, shorthashes in tag_to_commits.items():
        for i in range(len(shorthashes)):
            for j in range(i + 1, min(i + 4, len(shorthashes))):
                a, b = shorthashes[i], shorthashes[j]
                pair = (min(a, b), max(a, b))
                if pair not in seen_correlates:
                    seen_correlates.add(pair)
                    edges.append({
                        "from": f"commit:{a}",
                        "to": f"commit:{b}",
                        "type": "Correlates",
                        "weight": 0.5,
                    })

    # 4. EvidenceFor — commit references a PR/issue
    for c in commits:
        for num in extract_pr_refs(c["subject"]):
            edges.append({
                "from": f"commit:{c['short']}",
                "to": f"pr:{num}",
                "type": "EvidenceFor",
                "weight": 0.8,
            })

    # Stats ------------------------------------------------------------------
    dates = [c["date"] for c in commits if c["date"]]
    date_from = min(dates) if dates else ""
    date_to = max(dates) if dates else ""

    stats = {
        "total_commits": len(commits),
        "total_nodes": len(nodes),
        "total_edges": len(edges),
        "date_range": {"from": date_from, "to": date_to},
        "tags": dict(sorted(tag_counts.items(), key=lambda x: -x[1])),
    }

    return {
        "domain": "clawft-weftos",
        "source": "git-history",
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "nodes": nodes,
        "edges": edges,
        "stats": stats,
    }


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main() -> None:
    raw = sys.stdin.read()
    lines = raw.split("\n")
    commits = parse_git_log(lines)
    graph = build_graph(commits)
    json.dump(graph, sys.stdout, indent=2, ensure_ascii=False)
    sys.stdout.write("\n")


if __name__ == "__main__":
    main()
