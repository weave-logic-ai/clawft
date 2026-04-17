# Addendum: Tree Calculus as Formal Foundation

**Source**: https://treecalcul.us/ | Barry Jay, 2021

## What It Is

Turing-complete computation from ONE symbol. Everything is a binary tree.
Three forms: **leaf**, **stem** (one child), **fork** (two children).

The key operation is **triage** — native structural pattern-matching:
- Arg is leaf → handler W
- Arg is stem → handler X
- Arg is fork → handler Y

Programs can inspect the structure of other programs. This is the
critical difference from lambda calculus.

## Why It Matters

**Triage IS geometry dispatch.** Our navigator pattern-matches: atom,
sequence, or branch? Triage does exactly this natively.

**Layout as reduction.** Layout transforms are tree→tree rewrites.
Mode composition (layout + heatmap + flow) is reduction composition.

**Self-interpretation.** Auto-detection heuristic = program that
examines topology-tree and returns layout-tree. First-class in
tree calculus.

## Rust + RVF Implementation Path

Tree calculus fits WeftOS natively:

```rust
enum Topology {
    Atom,                                    // Leaf
    Sequence(Box<Topology>),                 // Stem
    Branch(Box<Topology>, Box<Topology>),    // Fork
}
```

Serialized into RVF segments — each tree node is an RVF record with
child pointers:
- Cryptographic provenance via ExoChain (layout steps auditable)
- Zero-copy deserialization (RVF maps to Topology without allocation)
- Network transport (topology-trees as RVF wire frames between agents)
- WASM execution (user layout algorithms as sandboxed tree reductions)

Implementation: define `Topology` in `clawft-types`, add RVF
serialization, implement five reduction rules, build layout algorithms
as tree→tree transforms.

## Codebase Mapping

| Tree Calculus | WeftOS |
|---------------|--------|
| Leaf | Node with no `contains` |
| Stem | Node with `geometry: stream/timeline` |
| Fork | Node with `contains` children |
| Triage | Schema geometry dispatch |
| Reduction | Mode composition |
| Self-interpreter | Auto-detection heuristic |

## References

- Jay, B. "Reflective Programs in Tree Calculus." 2021
- Rust: github.com/Philogy/tree-calculus-rs
- JS: github.com/lambada-llc/tree-calculus
