#!/usr/bin/env python3
"""
Parse SPARC plans and symposium results into an ECC-ready decisions graph.

Reads structured data from stdin (JSON) and emits the graph to stdout.
Called by ingest-decisions.sh which prepares the input data.
"""
import json
import sys
from datetime import datetime, timezone


def main():
    data = json.load(sys.stdin)
    nodes = []
    edges = []

    # ── Symposium Decisions ──────────────────────────────────────────
    for sym in data["symposiums"]:
        sym_id = sym["id"]  # e.g. "k2", "k3", "k5", "ecc"
        for d in sym["decisions"]:
            node = {
                "id": f"decision:{sym_id}:{d['id']}",
                "type": "decision",
                "symposium": sym_id,
                "title": d["title"],
                "rationale": d.get("rationale", ""),
                "panel": d.get("panel", ""),
                "status": d.get("status", "implemented"),
                "commitments": d.get("commitments", []),
            }
            nodes.append(node)

        # Decision → Decision edges (sequential within symposium)
        decision_ids = [f"decision:{sym_id}:{d['id']}" for d in sym["decisions"]]
        for i in range(len(decision_ids) - 1):
            edges.append({
                "from": decision_ids[i],
                "to": decision_ids[i + 1],
                "type": "Follows",
                "weight": 0.5,
            })

    # ── Commitments ──────────────────────────────────────────────────
    for sym in data["symposiums"]:
        sym_id = sym["id"]
        for c in sym.get("commitments", []):
            node = {
                "id": f"commitment:{sym_id}:{c['id']}",
                "type": "commitment",
                "symposium": sym_id,
                "description": c["description"],
                "status": c.get("status", "pending"),
                "phase": c.get("phase", ""),
            }
            nodes.append(node)

            # Commitment → Module edges
            for mod in c.get("modules", []):
                edges.append({
                    "from": f"commitment:{sym_id}:{c['id']}",
                    "to": f"module:{mod}",
                    "type": "Causes",
                    "weight": 0.9,
                })

            # Decision → Commitment edges
            if c.get("source_decision"):
                edges.append({
                    "from": f"decision:{sym_id}:{c['source_decision']}",
                    "to": f"commitment:{sym_id}:{c['id']}",
                    "type": "Causes",
                    "weight": 1.0,
                })

    # ── Phases ───────────────────────────────────────────────────────
    for p in data["phases"]:
        node = {
            "id": f"phase:{p['id']}",
            "type": "phase",
            "title": p["title"],
            "goal": p.get("goal", ""),
            "status": p.get("status", "planned"),
            "exit_criteria_total": p.get("exit_criteria_total", 0),
            "exit_criteria_checked": p.get("exit_criteria_checked", 0),
            "deliverables": p.get("deliverables", []),
        }
        nodes.append(node)

    # Phase → Phase dependency edges
    for p in data["phases"]:
        for dep in p.get("depends_on", []):
            edges.append({
                "from": f"phase:{dep}",
                "to": f"phase:{p['id']}",
                "type": "Enables",
                "weight": 1.0,
            })

    # ── Decision → Module implementation edges ───────────────────────
    for link in data.get("decision_module_links", []):
        edges.append({
            "from": link["from"],
            "to": f"module:{link['to']}",
            "type": "Causes",
            "weight": link.get("weight", 1.0),
        })

    # ── Symposium → Gap Analysis edges ───────────────────────────────
    for link in data.get("symposium_gap_links", []):
        edges.append({
            "from": link["from"],
            "to": link["to"],
            "type": "Causes",
            "weight": link.get("weight", 0.8),
        })

    # ── Compute stats ────────────────────────────────────────────────
    type_counts = {}
    status_counts = {"implemented": 0, "pending": 0, "deferred": 0, "blocked": 0, "partial": 0}
    for n in nodes:
        t = n["type"]
        type_counts[t] = type_counts.get(t, 0) + 1
        s = n.get("status", "")
        if s in status_counts:
            status_counts[s] += 1

    output = {
        "domain": "clawft-weftos",
        "source": "decisions-and-phases",
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "nodes": nodes,
        "edges": edges,
        "stats": {
            "total_decisions": type_counts.get("decision", 0),
            "total_phases": type_counts.get("phase", 0),
            "total_commitments": type_counts.get("commitment", 0),
            "total_nodes": len(nodes),
            "total_edges": len(edges),
            "implemented": status_counts["implemented"],
            "pending": status_counts["pending"],
            "deferred": status_counts["deferred"],
            "blocked": status_counts["blocked"],
            "partial": status_counts["partial"],
        },
    }

    json.dump(output, sys.stdout, indent=2)
    print()  # trailing newline


if __name__ == "__main__":
    main()
