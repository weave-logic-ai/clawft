#!/usr/bin/env python3
"""Build module-deps.json from Rust crate/module analysis.

Usage: python3 _build_module_graph.py <repo_root>

Outputs JSON to stdout.
"""

import json
import os
import re
import sys
from collections import defaultdict
from datetime import datetime, timezone
from pathlib import Path

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def parse_crate_name(cargo_toml: Path) -> str | None:
    """Extract the crate name from a Cargo.toml."""
    for line in cargo_toml.read_text().splitlines():
        m = re.match(r'^name\s*=\s*"([^"]+)"', line)
        if m:
            return m.group(1)
    return None


def parse_internal_deps(cargo_toml: Path, all_crate_names: set[str]) -> list[str]:
    """Extract workspace/path dependencies that refer to internal crates."""
    deps = []
    text = cargo_toml.read_text()
    # Match lines like:  clawft-core = { workspace = true ... }
    # or:                 exo-resource-tree = { workspace = true ... }
    for m in re.finditer(r'^([\w-]+)\s*=\s*\{[^}]*(?:workspace\s*=\s*true|path\s*=)', text, re.M):
        dep_name = m.group(1)
        if dep_name in all_crate_names:
            deps.append(dep_name)
    return sorted(set(deps))


def parse_features(cargo_toml: Path) -> list[str]:
    """Extract feature names from [features] section."""
    features = []
    in_features = False
    text = cargo_toml.read_text()
    for line in text.splitlines():
        if re.match(r'^\[features\]', line):
            in_features = True
            continue
        if in_features:
            if re.match(r'^\[', line):
                break
            m = re.match(r'^(\w[\w-]*)\s*=', line)
            if m:
                features.append(m.group(1))
    return features


def parse_module_feature_gate(lib_rs: Path) -> dict[str, str | None]:
    """Parse lib.rs to find cfg(feature) gates on pub mod declarations.

    Returns {module_name: feature_name_or_None}.
    """
    gates: dict[str, str | None] = {}
    lines = lib_rs.read_text().splitlines()
    pending_feature: str | None = None

    for line in lines:
        stripped = line.strip()
        # Check for #[cfg(feature = "...")]
        fm = re.match(r'#\[cfg\(feature\s*=\s*"([^"]+)"\)\]', stripped)
        if fm:
            pending_feature = fm.group(1)
            continue

        # Check for pub mod <name>;
        mm = re.match(r'pub\s+mod\s+(\w+)\s*;', stripped)
        if mm:
            mod_name = mm.group(1)
            gates[mod_name] = pending_feature
            pending_feature = None
            continue

        # Any non-attribute non-empty line resets the pending feature
        if stripped and not stripped.startswith('#[') and not stripped.startswith('//'):
            pending_feature = None

    return gates


def parse_use_crate_deps(rs_file: Path) -> list[str]:
    """Extract module names from `use crate::<module>` statements."""
    deps = set()
    for line in rs_file.read_text().splitlines():
        for m in re.finditer(r'use\s+crate::([a-z_]+)', line):
            deps.add(m.group(1))
    return sorted(deps)


def count_tests(rs_file: Path) -> int:
    """Count #[test] annotations in a file."""
    return rs_file.read_text().count('#[test]')


def count_lines(rs_file: Path) -> int:
    """Count lines in a file."""
    return len(rs_file.read_text().splitlines())


# K-level category heuristic based on module name and feature gate
K_CATEGORIES = {
    'ecc': 'K3c',
    'exochain': 'K5',
    'mesh': 'K6',
    'os-patterns': 'K8',
    'cluster': 'K7',
    'wasm-sandbox': 'K3',
    'containers': 'K4',
    'tilezero': 'K5t',
}


def classify_module(module_name: str, feature_gate: str | None) -> str:
    """Assign a k-level category based on feature gate or module name."""
    if feature_gate and feature_gate in K_CATEGORIES:
        return K_CATEGORIES[feature_gate]
    # Infer from prefix
    if module_name.startswith('mesh'):
        return 'K6'
    if module_name in ('chain', 'gate', 'tree_manager'):
        return 'K5'
    if module_name in ('artifact_store', 'calibration', 'causal', 'cognitive_tick',
                       'crossref', 'embedding', 'hnsw_service', 'impulse', 'weaver'):
        return 'K3c'
    if module_name in ('monitor', 'reconciler', 'dead_letter', 'reliable_queue',
                       'named_pipe', 'metrics', 'log_service', 'timer'):
        return 'K8'
    if module_name in ('container',):
        return 'K4'
    if module_name in ('wasm_runner',):
        return 'K3'
    if module_name in ('cluster',):
        return 'K7'
    return 'K1'  # core kernel


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    if len(sys.argv) < 2:
        print("Usage: _build_module_graph.py <repo_root>", file=sys.stderr)
        sys.exit(1)

    repo_root = Path(sys.argv[1])
    crates_dir = repo_root / "crates"

    # ── Step 1: Discover all crates ──────────────────────────────────
    crate_dirs: dict[str, Path] = {}  # name -> dir
    for d in sorted(crates_dir.iterdir()):
        cargo = d / "Cargo.toml"
        if cargo.exists():
            name = parse_crate_name(cargo)
            if name:
                crate_dirs[name] = d

    all_crate_names = set(crate_dirs.keys())

    # ── Step 2: Build crate nodes and inter-crate edges ──────────────
    nodes = []
    edges = []

    crate_deps_map: dict[str, list[str]] = {}

    for crate_name, crate_path in sorted(crate_dirs.items()):
        cargo = crate_path / "Cargo.toml"
        src_dir = crate_path / "src"

        rs_files = sorted(src_dir.glob("**/*.rs")) if src_dir.exists() else []
        file_count = len(rs_files)
        total_lines = sum(count_lines(f) for f in rs_files)

        internal_deps = parse_internal_deps(cargo, all_crate_names)
        crate_deps_map[crate_name] = internal_deps

        nodes.append({
            "id": f"crate:{crate_name}",
            "type": "crate",
            "files": file_count,
            "lines": total_lines,
            "dependencies": internal_deps,
        })

        # Crate dependency edges
        for dep in internal_deps:
            edges.append({
                "from": f"crate:{crate_name}",
                "to": f"crate:{dep}",
                "type": "DependsOn",
                "weight": 1.0,
            })

    # ── Step 3: Build module nodes (for all crates) ──────────────────
    # Track feature gates from lib.rs for each crate
    module_ids: dict[str, dict] = {}  # id -> node
    module_edge_counts: defaultdict[str, int] = defaultdict(int)

    for crate_name, crate_path in sorted(crate_dirs.items()):
        src_dir = crate_path / "src"
        if not src_dir.exists():
            continue

        lib_rs = src_dir / "lib.rs"
        feature_gates = parse_module_feature_gate(lib_rs) if lib_rs.exists() else {}

        for rs_file in sorted(src_dir.glob("*.rs")):
            module_name = rs_file.stem
            if module_name == "lib":
                continue  # skip lib.rs as a module (it's the crate root)

            lines = count_lines(rs_file)
            tests = count_tests(rs_file)
            feat_gate = feature_gates.get(module_name)
            rel_path = str(rs_file.relative_to(repo_root))
            category = classify_module(module_name, feat_gate)

            mod_id = f"module:{crate_name}/{module_name}"
            node = {
                "id": mod_id,
                "type": "module",
                "crate": crate_name,
                "file": rel_path,
                "lines": lines,
                "test_count": tests,
                "feature_gate": feat_gate,
                "category": category,
            }
            nodes.append(node)
            module_ids[mod_id] = node

            # Feature-gate edge
            if feat_gate:
                feat_node_id = f"feature:{feat_gate}"
                edges.append({
                    "from": mod_id,
                    "to": feat_node_id,
                    "type": "Enables",
                    "weight": 1.0,
                })
                module_edge_counts[mod_id] += 1
                module_edge_counts[feat_node_id] += 1

            # Test evidence edge
            if tests > 0:
                test_id = f"tests:{module_name}:{tests}"
                edges.append({
                    "from": mod_id,
                    "to": test_id,
                    "type": "EvidenceFor",
                    "weight": round(min(0.5 + tests * 0.03, 1.0), 2),
                })
                module_edge_counts[mod_id] += 1

    # ── Step 4: Build module `use crate::` edges ─────────────────────
    for crate_name, crate_path in sorted(crate_dirs.items()):
        src_dir = crate_path / "src"
        if not src_dir.exists():
            continue

        for rs_file in sorted(src_dir.glob("*.rs")):
            module_name = rs_file.stem
            if module_name == "lib":
                continue

            from_id = f"module:{crate_name}/{module_name}"
            use_deps = parse_use_crate_deps(rs_file)

            for dep_mod in use_deps:
                to_id = f"module:{crate_name}/{dep_mod}"
                if to_id in module_ids and to_id != from_id:
                    edges.append({
                        "from": from_id,
                        "to": to_id,
                        "type": "Uses",
                        "weight": 0.9,
                    })
                    module_edge_counts[from_id] += 1
                    module_edge_counts[to_id] += 1

    # ── Step 5: Collect unique feature nodes ─────────────────────────
    seen_features = set()
    for e in edges:
        if e["to"].startswith("feature:"):
            feat = e["to"]
            if feat not in seen_features:
                seen_features.add(feat)
                feat_name = feat.split(":", 1)[1]
                nodes.append({
                    "id": feat,
                    "type": "feature",
                    "name": feat_name,
                })

    # ── Step 6: Stats ────────────────────────────────────────────────
    total_crates = sum(1 for n in nodes if n["type"] == "crate")
    total_modules = sum(1 for n in nodes if n["type"] == "module")
    total_edges = len(edges)
    total_nodes = len(nodes)

    # Average degree: edges connect two endpoints each
    avg_degree = round((2 * total_edges) / max(total_nodes, 1), 1)

    # Most connected node
    if module_edge_counts:
        most_connected = max(module_edge_counts, key=module_edge_counts.get)
    else:
        most_connected = "none"

    output = {
        "domain": "clawft-weftos",
        "source": "module-dependencies",
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "nodes": nodes,
        "edges": edges,
        "stats": {
            "total_crates": total_crates,
            "total_modules": total_modules,
            "total_features": len(seen_features),
            "total_nodes": total_nodes,
            "total_edges": total_edges,
            "avg_degree": avg_degree,
            "most_connected": most_connected,
        },
    }

    json.dump(output, sys.stdout, indent=2)
    sys.stdout.write("\n")


if __name__ == "__main__":
    main()
