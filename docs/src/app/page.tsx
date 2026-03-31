import Link from 'next/link';

export default function HomePage() {
  return (
    <main className="flex min-h-screen flex-col items-center justify-center px-4">
      <div className="mx-auto max-w-4xl text-center">
        <h1 className="mb-4 text-5xl font-bold tracking-tight text-fd-foreground sm:text-6xl">
          WeftOS
        </h1>
        <p className="mb-6 text-xl text-fd-foreground/80 sm:text-2xl">
          Full-stack AI operating system built in Rust
        </p>
        <p className="mb-12 text-base text-fd-muted-foreground sm:text-lg max-w-2xl mx-auto">
          A complete AI framework from agent runtime to distributed kernel. 22 crates,
          181K lines of Rust, 3,300+ tests. Agents get persistent memory, verifiable
          reasoning, constitutional governance, and encrypted mesh networking.
        </p>

        {/* Three-layer architecture */}
        <div className="grid gap-4 sm:grid-cols-3 mb-12">
          <Link
            href="/docs/clawft"
            className="group rounded-xl border border-fd-border bg-fd-card p-6 text-left transition-colors hover:border-fd-primary hover:bg-fd-accent"
          >
            <div className="mb-2 text-xs font-medium uppercase tracking-wider text-fd-primary">
              Layer 1 — Agent Runtime
            </div>
            <h2 className="mb-2 text-xl font-semibold text-fd-card-foreground">
              clawft
            </h2>
            <p className="text-sm text-fd-muted-foreground">
              7-stage processing pipeline, 9 LLM providers, 11 messaging channels,
              tiered model routing, self-improving skills, and WASM-sandboxed tools.
            </p>
          </Link>

          <Link
            href="/docs/weftos"
            className="group rounded-xl border border-fd-border bg-fd-card p-6 text-left transition-colors hover:border-fd-primary hover:bg-fd-accent"
          >
            <div className="mb-2 text-xs font-medium uppercase tracking-wider text-fd-primary">
              Layer 2 — Kernel
            </div>
            <h2 className="mb-2 text-xl font-semibold text-fd-card-foreground">
              WeftOS
            </h2>
            <p className="text-sm text-fd-muted-foreground">
              Process management with PID tracking, ExoChain cryptographic provenance,
              three-branch governance, encrypted P2P mesh, and self-healing supervision.
            </p>
          </Link>

          <Link
            href="/docs/weftos/ecc"
            className="group rounded-xl border border-fd-border bg-fd-card p-6 text-left transition-colors hover:border-fd-primary hover:bg-fd-accent"
          >
            <div className="mb-2 text-xs font-medium uppercase tracking-wider text-fd-primary">
              Layer 3 — Cognitive
            </div>
            <h2 className="mb-2 text-xl font-semibold text-fd-card-foreground">
              ECC
            </h2>
            <p className="text-sm text-fd-muted-foreground">
              Causal knowledge graph, HNSW semantic search, spectral analysis,
              community detection, predictive change analysis, and the DEMOCRITUS
              continuous cognitive loop.
            </p>
          </Link>
        </div>

        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
          {[
            { label: 'Agent Loop', desc: '7-stage pipeline with tiered routing' },
            { label: 'Governance', desc: 'Constitutional AI with effect vectors' },
            { label: 'Provenance', desc: 'Cryptographic audit trail (ExoChain)' },
            { label: 'Mesh', desc: 'Encrypted P2P across heterogeneous nodes' },
          ].map((item) => (
            <div key={item.label} className="rounded-lg border border-fd-border p-4">
              <div className="mb-1 font-semibold text-fd-foreground">{item.label}</div>
              <div className="text-sm text-fd-muted-foreground">{item.desc}</div>
            </div>
          ))}
        </div>
      </div>
    </main>
  );
}
