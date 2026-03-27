#!/usr/bin/env python3
"""
ECC Gap Analyzer — finds incomplete areas in clawft+WeftOS codebase
using the Weaver's three graph data sources.

Reads:
  .weftos/graph/module-deps.json   (214 nodes, 473 edges)
  .weftos/graph/git-history.json   (102 nodes, 965 edges)
  .weftos/graph/decisions.json     (92 nodes, 159 edges)

Produces:
  .weftos/analysis/gap-report.json
"""

import json
import os
import sys
from collections import defaultdict
from datetime import datetime, timezone

BASE = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
GRAPH_DIR = os.path.join(BASE, ".weftos", "graph")
OUT_DIR = os.path.join(BASE, ".weftos", "analysis")

# ---------------------------------------------------------------------------
# Load data
# ---------------------------------------------------------------------------

def load_graph(name):
    path = os.path.join(GRAPH_DIR, name)
    with open(path) as f:
        return json.load(f)


deps = load_graph("module-deps.json")
git = load_graph("git-history.json")
decisions = load_graph("decisions.json")

deps_nodes = {n["id"]: n for n in deps["nodes"]}
git_nodes = {n["id"]: n for n in git["nodes"]}
dec_nodes = {n["id"]: n for n in decisions["nodes"]}

# ---------------------------------------------------------------------------
# 1. Orphan Modules (no incoming Uses edges)
# ---------------------------------------------------------------------------

def find_orphan_modules():
    incoming_uses = set()
    for e in deps["edges"]:
        if e["type"] == "Uses":
            incoming_uses.add(e["to"])

    orphans = []
    for n in deps["nodes"]:
        if n["type"] == "module" and n["id"] not in incoming_uses:
            orphans.append({
                "id": n["id"],
                "crate": n.get("crate", "?"),
                "file": n.get("file", "?"),
                "lines": n.get("lines", 0),
                "test_count": n.get("test_count", 0),
                "feature_gate": n.get("feature_gate"),
                "category": n.get("category", "?"),
            })
    orphans.sort(key=lambda x: -x["lines"])
    return orphans

# ---------------------------------------------------------------------------
# 2. Untested Modules (zero EvidenceFor edges FROM test nodes)
# ---------------------------------------------------------------------------

def find_untested_modules():
    tested = set()
    for e in deps["edges"]:
        if e["type"] == "EvidenceFor":
            tested.add(e["to"])

    untested = []
    for n in deps["nodes"]:
        if n["type"] == "module" and n["id"] not in tested:
            untested.append({
                "id": n["id"],
                "crate": n.get("crate", "?"),
                "file": n.get("file", "?"),
                "lines": n.get("lines", 0),
                "test_count": n.get("test_count", 0),
                "feature_gate": n.get("feature_gate"),
                "category": n.get("category", "?"),
            })
    untested.sort(key=lambda x: -x["lines"])
    return untested

# ---------------------------------------------------------------------------
# 3. Decision Gaps (pending / blocked / deferred decisions)
# ---------------------------------------------------------------------------

def find_decision_gaps():
    pending = []
    for n in decisions["nodes"]:
        if n["type"] == "decision" and n.get("status") in ("pending", "blocked", "deferred"):
            # Find downstream edges (what does this decision Cause/Enable?)
            downstream = []
            for e in decisions["edges"]:
                if e["from"] == n["id"] and e["type"] in ("Causes", "Enables"):
                    downstream.append(e["to"])
            pending.append({
                "id": n["id"],
                "title": n.get("title", ""),
                "status": n["status"],
                "symposium": n.get("symposium", "?"),
                "panel": n.get("panel", "?"),
                "commitments": n.get("commitments", []),
                "downstream_count": len(downstream),
                "downstream": downstream,
            })

    # Also find pending/blocked commitments
    pending_commitments = []
    for n in decisions["nodes"]:
        if n["type"] == "commitment" and n.get("status") in ("pending", "blocked", "partial"):
            pending_commitments.append({
                "id": n["id"],
                "description": n.get("description", ""),
                "status": n["status"],
                "phase": n.get("phase", "?"),
            })

    # Sort by downstream impact (most blocking first)
    pending.sort(key=lambda x: -x["downstream_count"])
    return pending, pending_commitments

# ---------------------------------------------------------------------------
# 4. Git History Gaps (cold areas — tags with few commits)
# ---------------------------------------------------------------------------

def find_cold_areas():
    tag_commits = defaultdict(list)
    for n in git["nodes"]:
        tags = n.get("tags", [])
        date = n.get("date", "")
        for t in tags:
            tag_commits[t].append({
                "id": n["id"],
                "date": date,
                "subject": n.get("subject", ""),
            })

    # Sort tags by commit count (fewest first = coldest)
    cold = []
    for tag, commits in sorted(tag_commits.items(), key=lambda x: len(x[1])):
        dates = [c["date"] for c in commits if c["date"]]
        latest = max(dates) if dates else "unknown"
        cold.append({
            "tag": tag,
            "commit_count": len(commits),
            "latest_commit_date": latest,
            "latest_subject": commits[-1]["subject"] if commits else "",
        })

    # Commits with NO tags at all (unclassified work)
    untagged = []
    for n in git["nodes"]:
        if not n.get("tags"):
            untagged.append({
                "id": n["id"],
                "date": n.get("date", ""),
                "subject": n.get("subject", ""),
            })

    return cold, untagged

# ---------------------------------------------------------------------------
# 5. Feature Gate Islands (isolated feature clusters)
# ---------------------------------------------------------------------------

def find_feature_gate_islands():
    # Group modules by feature gate
    gate_modules = defaultdict(list)
    for n in deps["nodes"]:
        if n["type"] == "module":
            gate = n.get("feature_gate") or "default"
            gate_modules[gate].append(n["id"])

    # Check cross-gate connections via Uses edges
    gate_connections = defaultdict(set)  # gate -> set of other gates it connects to
    module_gate = {}
    for n in deps["nodes"]:
        if n["type"] == "module":
            module_gate[n["id"]] = n.get("feature_gate") or "default"

    for e in deps["edges"]:
        if e["type"] == "Uses":
            from_gate = module_gate.get(e["from"], "unknown")
            to_gate = module_gate.get(e["to"], "unknown")
            if from_gate != to_gate and from_gate != "unknown" and to_gate != "unknown":
                gate_connections[from_gate].add(to_gate)
                gate_connections[to_gate].add(from_gate)

    islands = []
    for gate, modules in gate_modules.items():
        connections = gate_connections.get(gate, set())
        total_lines = sum(deps_nodes[m].get("lines", 0) for m in modules if m in deps_nodes)
        islands.append({
            "feature_gate": gate,
            "module_count": len(modules),
            "total_lines": total_lines,
            "cross_gate_connections": len(connections),
            "connected_to": sorted(connections),
            "is_island": len(connections) == 0,
        })

    islands.sort(key=lambda x: (x["is_island"], -x["module_count"]), reverse=True)
    return islands

# ---------------------------------------------------------------------------
# 6. High Fan-Out / Single Point of Failure
# ---------------------------------------------------------------------------

def find_high_fanout():
    outgoing = defaultdict(list)
    incoming = defaultdict(list)
    for e in deps["edges"]:
        if e["type"] == "Uses":
            outgoing[e["from"]].append(e["to"])
            incoming[e["to"]].append(e["from"])

    results = []
    for n in deps["nodes"]:
        if n["type"] == "module":
            out_count = len(outgoing.get(n["id"], []))
            in_count = len(incoming.get(n["id"], []))
            if out_count >= 3 or in_count >= 3:
                results.append({
                    "id": n["id"],
                    "crate": n.get("crate", "?"),
                    "lines": n.get("lines", 0),
                    "test_count": n.get("test_count", 0),
                    "outgoing_uses": out_count,
                    "incoming_uses": in_count,
                    "fan_out_risk": out_count >= 5,
                    "single_point_of_failure": in_count >= 5,
                })

    results.sort(key=lambda x: -(x["outgoing_uses"] + x["incoming_uses"]))
    return results

# ---------------------------------------------------------------------------
# 7. Cross-Graph Correlations
# ---------------------------------------------------------------------------

def cross_graph_analysis():
    # Map files from git commits to crates/modules
    file_to_crate = {}
    for n in deps["nodes"]:
        if n["type"] == "module":
            f = n.get("file", "")
            if f:
                file_to_crate[f] = n.get("crate", "?")

    # Which crates have recent commits?
    crate_last_commit = {}
    crate_commit_count = defaultdict(int)
    for n in git["nodes"]:
        for f in n.get("files", []):
            for cf, crate in file_to_crate.items():
                if f.startswith(cf.rsplit("/", 1)[0] if "/" in cf else cf):
                    date = n.get("date", "")
                    if crate not in crate_last_commit or date > crate_last_commit[crate]:
                        crate_last_commit[crate] = date
                    crate_commit_count[crate] += 1

    # Phases with decisions vs commits
    phase_decisions = defaultdict(int)
    phase_decision_pending = defaultdict(int)
    for n in decisions["nodes"]:
        if n["type"] == "decision":
            sym = n.get("symposium", "?")
            phase_decisions[sym] += 1
            if n.get("status") in ("pending", "blocked", "deferred"):
                phase_decision_pending[sym] += 1

    phase_commits = defaultdict(int)
    tag_to_phase = {
        "process-mgmt": "K0-K2",
        "service-framework": "K2-K3",
        "wasm-sandbox": "K3",
        "container-runtime": "K4",
        "app-lifecycle": "K5",
        "mesh-networking": "K5-K6",
        "ecc": "K3c",
        "exochain": "K0-K6",
        "governance": "K0-K2",
        "security": "K0-K6",
        "agents": "K1-K5",
        "testing": "all",
        "documentation": "all",
        "planning": "all",
    }
    for n in git["nodes"]:
        for t in n.get("tags", []):
            phase = tag_to_phase.get(t, "unknown")
            phase_commits[phase] += 1

    # Decisions without implementation evidence
    decisions_without_code = []
    sym_impl_tags = {
        "k2": {"process-mgmt", "service-framework", "governance"},
        "k3": {"wasm-sandbox", "container-runtime"},
        "ecc": {"ecc"},
        "k5": {"mesh-networking", "app-lifecycle"},
    }
    for n in decisions["nodes"]:
        if n["type"] == "decision" and n.get("status") == "pending":
            sym = n.get("symposium", "")
            decisions_without_code.append({
                "decision": n["id"],
                "title": n.get("title", ""),
                "symposium": sym,
                "status": n["status"],
            })

    return {
        "crate_activity": {
            crate: {
                "commit_count": crate_commit_count.get(crate, 0),
                "last_commit": crate_last_commit.get(crate, "never"),
            }
            for crate in set(list(crate_commit_count.keys()) + list(crate_last_commit.keys()))
        },
        "phase_balance": {
            sym: {
                "total_decisions": phase_decisions[sym],
                "pending_decisions": phase_decision_pending[sym],
                "implementation_ratio": round(
                    1.0 - (phase_decision_pending[sym] / max(phase_decisions[sym], 1)), 2
                ),
            }
            for sym in phase_decisions
        },
        "unimplemented_decisions": decisions_without_code,
    }

# ---------------------------------------------------------------------------
# 8. Conversation Health Assessment
# ---------------------------------------------------------------------------

def assess_conversation_health():
    """Map each of the Weaver's 12 conversations to graph data."""

    # Conversation -> module/feature mapping
    conversation_map = {
        "C1: Kernel Architecture": {
            "crate": "clawft-kernel",
            "feature_gates": ["default", "native"],
            "symposiums": ["k2"],
        },
        "C2: Symposium Decisions": {
            "crate": None,  # decisions graph
            "feature_gates": [],
            "symposiums": ["k2", "k3", "ecc", "k5"],
        },
        "C3: SPARC Planning": {
            "crate": None,  # planning docs
            "feature_gates": [],
            "symposiums": [],
        },
        "C4: Mesh Networking": {
            "crate": "clawft-kernel",
            "feature_gates": ["mesh", "mesh-discovery", "mesh-full"],
            "symposiums": ["k5"],
        },
        "C5: ECC Cognitive Substrate": {
            "crate": "clawft-kernel",
            "feature_gates": ["ecc"],
            "symposiums": ["ecc"],
        },
        "C6: ExoChain Provenance": {
            "crate": "clawft-kernel",
            "feature_gates": ["exochain"],
            "symposiums": ["k2"],
        },
        "C7: Governance Constitution": {
            "crate": "clawft-kernel",
            "feature_gates": ["exochain"],  # governance uses exochain
            "symposiums": ["k2"],
        },
        "C8: Tool Lifecycle": {
            "crate": "clawft-kernel",
            "feature_gates": ["default"],
            "symposiums": ["k3"],
        },
        "C9: Agent Architecture": {
            "crate": "clawft-kernel",
            "feature_gates": ["default"],
            "symposiums": ["k2"],
        },
        "C10: OS Pattern Library": {
            "crate": "clawft-kernel",
            "feature_gates": ["os-patterns"],
            "symposiums": [],
        },
        "C11: Git Commit History": {
            "crate": None,  # git graph
            "feature_gates": [],
            "symposiums": [],
        },
        "C12: Three-Mode Engine Design": {
            "crate": None,  # theoretical
            "feature_gates": [],
            "symposiums": ["ecc"],
        },
    }

    health = {}
    for conv_name, mapping in conversation_map.items():
        # Count modules in this conversation
        module_count = 0
        total_lines = 0
        total_tests = 0
        untested_modules = 0

        if mapping["crate"]:
            for n in deps["nodes"]:
                if n["type"] == "module" and n.get("crate") == mapping["crate"]:
                    gate = n.get("feature_gate") or "default"
                    if not mapping["feature_gates"] or gate in mapping["feature_gates"]:
                        module_count += 1
                        total_lines += n.get("lines", 0)
                        total_tests += n.get("test_count", 0)
                        if n.get("test_count", 0) == 0:
                            untested_modules += 1

        # Count decisions for this conversation's symposiums
        decision_count = 0
        pending_count = 0
        for n in decisions["nodes"]:
            if n["type"] == "decision" and n.get("symposium") in mapping["symposiums"]:
                decision_count += 1
                if n.get("status") in ("pending", "blocked", "deferred"):
                    pending_count += 1

        # Count relevant git commits (by tags)
        tag_mapping = {
            "C1: Kernel Architecture": {"process-mgmt", "service-framework"},
            "C4: Mesh Networking": {"mesh-networking"},
            "C5: ECC Cognitive Substrate": {"ecc"},
            "C6: ExoChain Provenance": {"exochain"},
            "C7: Governance Constitution": {"governance"},
            "C8: Tool Lifecycle": {"wasm-sandbox"},
            "C9: Agent Architecture": {"agents"},
            "C10: OS Pattern Library": {"process-mgmt"},
            "C11: Git Commit History": set(),  # all commits
        }
        relevant_tags = tag_mapping.get(conv_name, set())
        commit_count = 0
        for n in git["nodes"]:
            node_tags = set(n.get("tags", []))
            if relevant_tags and node_tags & relevant_tags:
                commit_count += 1

        # Health score: modules with tests, decisions implemented, recent commits
        test_coverage = (
            round((module_count - untested_modules) / max(module_count, 1), 2)
            if module_count > 0
            else None
        )
        decision_completion = (
            round(1.0 - (pending_count / max(decision_count, 1)), 2)
            if decision_count > 0
            else None
        )

        # Composite health
        scores = [s for s in [test_coverage, decision_completion] if s is not None]
        composite = round(sum(scores) / len(scores), 2) if scores else None

        health[conv_name] = {
            "module_count": module_count,
            "total_lines": total_lines,
            "total_tests": total_tests,
            "untested_modules": untested_modules,
            "test_coverage_ratio": test_coverage,
            "decision_count": decision_count,
            "pending_decisions": pending_count,
            "decision_completion_ratio": decision_completion,
            "commit_count": commit_count,
            "composite_health": composite,
            "needs_attention": composite is not None and composite < 0.7,
        }

    return health

# ---------------------------------------------------------------------------
# 9. Phase Status Summary
# ---------------------------------------------------------------------------

def phase_status():
    phases = []
    for n in decisions["nodes"]:
        if n["type"] == "phase":
            phases.append({
                "id": n["id"],
                "title": n.get("title", ""),
                "status": n.get("status", "?"),
                "exit_criteria_total": n.get("exit_criteria_total", 0),
                "exit_criteria_checked": n.get("exit_criteria_checked", 0),
                "completion_pct": round(
                    n.get("exit_criteria_checked", 0)
                    / max(n.get("exit_criteria_total", 1), 1)
                    * 100,
                    1,
                ),
                "deliverables": n.get("deliverables", []),
            })
    phases.sort(key=lambda x: x["id"])
    return phases

# ---------------------------------------------------------------------------
# 10. Top Gaps (synthesized ranking)
# ---------------------------------------------------------------------------

def rank_top_gaps(orphans, untested, decision_gaps, commitments, cold, islands, fanout, cross, health, phases):
    gaps = []

    # Phase K8 has 117 exit criteria and 0 checked — massive gap
    for p in phases:
        if p["status"] == "planned" and p["exit_criteria_checked"] == 0:
            gaps.append({
                "rank": 0,
                "severity": "critical",
                "area": f"Phase {p['id']} not started",
                "detail": f"{p['title']}: {p['exit_criteria_total']} exit criteria, 0 checked",
                "impact": f"Blocks {len(p['deliverables'])} deliverables: {', '.join(p['deliverables'][:3])}...",
                "recommendation": f"Begin implementation of {p['id']} — largest remaining work package",
            })

    # Phases in-progress with incomplete criteria
    for p in phases:
        if p["status"] == "in-progress" and p["completion_pct"] < 100:
            remaining = p["exit_criteria_total"] - p["exit_criteria_checked"]
            gaps.append({
                "rank": 0,
                "severity": "high",
                "area": f"Phase {p['id']} incomplete",
                "detail": f"{p['title']}: {p['completion_pct']}% done, {remaining} criteria remaining",
                "impact": f"Blocks phase completion and downstream work",
                "recommendation": f"Complete remaining {remaining} exit criteria for {p['id']}",
            })

    # Blocked decisions
    for d in decision_gaps:
        if d["status"] == "blocked":
            gaps.append({
                "rank": 0,
                "severity": "high",
                "area": f"Blocked decision: {d['id']}",
                "detail": d["title"],
                "impact": f"Blocks {d['downstream_count']} downstream items",
                "recommendation": f"Unblock {d['id']} — resolve blocking dependency",
            })

    # Pending decisions with downstream impact
    for d in decision_gaps:
        if d["status"] == "pending" and d["downstream_count"] > 0:
            gaps.append({
                "rank": 0,
                "severity": "medium",
                "area": f"Pending decision with dependents: {d['id']}",
                "detail": d["title"],
                "impact": f"Blocks {d['downstream_count']} downstream items: {', '.join(d['downstream'][:2])}",
                "recommendation": "Convene panel to resolve decision",
            })

    # Blocked commitments
    for c in commitments:
        if c["status"] == "blocked":
            gaps.append({
                "rank": 0,
                "severity": "high",
                "area": f"Blocked commitment: {c['id']}",
                "detail": c["description"],
                "impact": f"Phase {c['phase']} cannot complete",
                "recommendation": "Resolve blocker for commitment",
            })

    # Large untested modules
    for u in untested[:5]:
        if u["lines"] > 200:
            gaps.append({
                "rank": 0,
                "severity": "medium",
                "area": f"Untested module: {u['id']}",
                "detail": f"{u['lines']} lines, 0 tests, category {u['category']}",
                "impact": "No regression safety net for significant code",
                "recommendation": f"Add tests to {u['file']}",
            })

    # Feature gate islands
    for island in islands:
        if island["is_island"] and island["module_count"] > 2:
            gaps.append({
                "rank": 0,
                "severity": "low",
                "area": f"Isolated feature gate: {island['feature_gate']}",
                "detail": f"{island['module_count']} modules, {island['total_lines']} lines, no cross-gate connections",
                "impact": "Integration risk — may not compose well with other features",
                "recommendation": "Verify integration points with other feature gates",
            })

    # High fan-out without adequate tests
    for f in fanout[:3]:
        if f["fan_out_risk"] and f["test_count"] < 5:
            gaps.append({
                "rank": 0,
                "severity": "high",
                "area": f"High fan-out, low test: {f['id']}",
                "detail": f"{f['outgoing_uses']} outgoing uses, {f['incoming_uses']} incoming, only {f['test_count']} tests",
                "impact": "Single point of failure with insufficient test coverage",
                "recommendation": f"Add comprehensive tests to {f['id']}",
            })

    # Conversations needing attention
    for conv, h in health.items():
        if h.get("needs_attention"):
            gaps.append({
                "rank": 0,
                "severity": "medium",
                "area": f"Conversation health: {conv}",
                "detail": f"Composite health {h['composite_health']}, {h['pending_decisions']} pending decisions, {h['untested_modules']} untested modules",
                "impact": "Conversation is underdeveloped relative to its importance",
                "recommendation": "Focus attention on this conversation's gaps",
            })

    # Assign ranks by severity
    severity_order = {"critical": 0, "high": 1, "medium": 2, "low": 3}
    gaps.sort(key=lambda x: severity_order.get(x["severity"], 4))
    for i, g in enumerate(gaps):
        g["rank"] = i + 1

    return gaps[:20]  # Top 20


# ---------------------------------------------------------------------------
# MAIN
# ---------------------------------------------------------------------------

def main():
    print("=" * 72)
    print("  ECC Gap Analysis — clawft + WeftOS Codebase")
    print(f"  Generated: {datetime.now(timezone.utc).isoformat()}")
    print("=" * 72)
    print()

    # Run all analyses
    orphans = find_orphan_modules()
    untested = find_untested_modules()
    decision_gaps, pending_commitments = find_decision_gaps()
    cold, untagged = find_cold_areas()
    islands = find_feature_gate_islands()
    fanout = find_high_fanout()
    cross = cross_graph_analysis()
    health = assess_conversation_health()
    phases = phase_status()
    top_gaps = rank_top_gaps(
        orphans, untested, decision_gaps, pending_commitments,
        cold, islands, fanout, cross, health, phases,
    )

    # -----------------------------------------------------------------------
    # Print results
    # -----------------------------------------------------------------------

    # 1. TOP GAPS
    print("=" * 72)
    print("  1. TOP GAPS (ranked by impact)")
    print("=" * 72)
    for g in top_gaps:
        print(f"\n  #{g['rank']} [{g['severity'].upper()}] {g['area']}")
        print(f"     Detail: {g['detail']}")
        print(f"     Impact: {g['impact']}")
        print(f"     Action: {g['recommendation']}")
    print()

    # 2. ORPHAN MODULES
    print("=" * 72)
    print("  2. ORPHAN MODULES (no incoming Uses edges)")
    print("=" * 72)
    print(f"\n  Total: {len(orphans)} modules with no dependents\n")
    for o in orphans[:20]:
        gate_str = f" [{o['feature_gate']}]" if o["feature_gate"] else ""
        test_str = f" ({o['test_count']} tests)" if o["test_count"] > 0 else " (NO TESTS)"
        print(f"  {o['id']:55s} {o['lines']:5d} lines{gate_str}{test_str}")
    if len(orphans) > 20:
        print(f"\n  ... and {len(orphans) - 20} more")
    print()

    # 3. UNTESTED MODULES
    print("=" * 72)
    print("  3. UNTESTED MODULES (zero EvidenceFor edges)")
    print("=" * 72)
    print(f"\n  Total: {len(untested)} modules with no test evidence\n")
    for u in untested[:15]:
        gate_str = f" [{u['feature_gate']}]" if u["feature_gate"] else ""
        print(f"  {u['id']:55s} {u['lines']:5d} lines  {u['category']}{gate_str}")
    if len(untested) > 15:
        print(f"\n  ... and {len(untested) - 15} more")
    print()

    # 4. DECISION BOTTLENECKS
    print("=" * 72)
    print("  4. DECISION BOTTLENECKS")
    print("=" * 72)
    print(f"\n  Pending/blocked decisions: {len(decision_gaps)}")
    print(f"  Pending/blocked commitments: {len(pending_commitments)}\n")
    for d in decision_gaps:
        ds = f" -> blocks {d['downstream_count']}" if d["downstream_count"] > 0 else ""
        print(f"  [{d['status'].upper():8s}] {d['id']:20s} {d['title'][:50]}{ds}")
    print()
    if pending_commitments:
        print("  Pending commitments:")
        for c in pending_commitments:
            print(f"  [{c['status'].upper():8s}] {c['id']:20s} {c['description'][:50]}  (phase {c['phase']})")
    print()

    # 5. COLD ZONES
    print("=" * 72)
    print("  5. COLD ZONES (development activity by tag)")
    print("=" * 72)
    print()
    for c in cold:
        bar = "#" * min(c["commit_count"], 40)
        print(f"  {c['tag']:20s} {c['commit_count']:3d} commits  {bar}")
    print(f"\n  Untagged commits: {len(untagged)}")
    for u in untagged[:5]:
        print(f"    {u['id']} ({u['date'][:10]}) {u['subject'][:60]}")
    print()

    # 6. INTEGRATION RISKS
    print("=" * 72)
    print("  6. INTEGRATION RISKS")
    print("=" * 72)
    print("\n  Feature Gate Islands:")
    for i in islands:
        status = "ISLAND" if i["is_island"] else f"connected to {len(i['connected_to'])} gates"
        print(f"  {i['feature_gate']:20s} {i['module_count']:3d} modules  {i['total_lines']:6d} lines  {status}")

    print("\n  High Fan-Out Modules (potential single points of failure):")
    for f in fanout[:10]:
        risk = " ** RISK **" if f["fan_out_risk"] or f["single_point_of_failure"] else ""
        print(f"  {f['id']:55s} out={f['outgoing_uses']} in={f['incoming_uses']} tests={f['test_count']}{risk}")
    print()

    # 7. CONVERSATION HEALTH
    print("=" * 72)
    print("  7. CONVERSATION HEALTH")
    print("=" * 72)
    print()
    for conv, h in sorted(health.items()):
        attention = " ** NEEDS ATTENTION **" if h.get("needs_attention") else ""
        tc = f"test_cov={h['test_coverage_ratio']}" if h["test_coverage_ratio"] is not None else "test_cov=N/A"
        dc = f"dec_comp={h['decision_completion_ratio']}" if h["decision_completion_ratio"] is not None else "dec_comp=N/A"
        ch = f"health={h['composite_health']}" if h["composite_health"] is not None else "health=N/A"
        print(f"  {conv}")
        print(f"    modules={h['module_count']} lines={h['total_lines']} tests={h['total_tests']} {tc} {dc} {ch}{attention}")

    # 8. PHASE STATUS
    print()
    print("=" * 72)
    print("  8. PHASE STATUS")
    print("=" * 72)
    print()
    for p in phases:
        bar_len = int(p["completion_pct"] / 5)
        bar = "#" * bar_len + "." * (20 - bar_len)
        print(f"  {p['id']:8s} [{bar}] {p['completion_pct']:5.1f}%  {p['status']:12s}  {p['title'][:40]}")
    print()

    # 9. CROSS-GRAPH: Phase balance
    print("=" * 72)
    print("  9. CROSS-GRAPH: Decision Implementation Balance")
    print("=" * 72)
    print()
    for sym, balance in sorted(cross["phase_balance"].items()):
        bar_len = int(balance["implementation_ratio"] * 20)
        bar = "#" * bar_len + "." * (20 - bar_len)
        print(f"  {sym:6s} [{bar}] {balance['implementation_ratio']*100:5.1f}%  ({balance['pending_decisions']}/{balance['total_decisions']} pending)")
    print()

    # -----------------------------------------------------------------------
    # Build JSON report
    # -----------------------------------------------------------------------
    report = {
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "summary": {
            "total_modules": len([n for n in deps["nodes"] if n["type"] == "module"]),
            "total_crates": len([n for n in deps["nodes"] if n["type"] == "crate"]),
            "total_commits": len(git["nodes"]),
            "total_decisions": len([n for n in decisions["nodes"] if n["type"] == "decision"]),
            "total_commitments": len([n for n in decisions["nodes"] if n["type"] == "commitment"]),
            "total_phases": len([n for n in decisions["nodes"] if n["type"] == "phase"]),
            "orphan_module_count": len(orphans),
            "untested_module_count": len(untested),
            "pending_decision_count": len(decision_gaps),
            "pending_commitment_count": len(pending_commitments),
            "feature_gate_island_count": len([i for i in islands if i["is_island"]]),
        },
        "top_gaps": top_gaps,
        "orphan_modules": orphans,
        "untested_modules": untested,
        "decision_bottlenecks": decision_gaps,
        "pending_commitments": pending_commitments,
        "cold_zones": cold,
        "untagged_commits": untagged,
        "feature_gate_islands": islands,
        "high_fanout_modules": fanout,
        "cross_graph": cross,
        "conversation_health": health,
        "phase_status": phases,
    }

    os.makedirs(OUT_DIR, exist_ok=True)
    out_path = os.path.join(OUT_DIR, "gap-report.json")
    with open(out_path, "w") as f:
        json.dump(report, f, indent=2)
    print(f"  Report saved to: {out_path}")
    print()

    return report


if __name__ == "__main__":
    main()
