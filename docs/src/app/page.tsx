import Link from 'next/link';

export default function HomePage() {
  return (
    <main className="flex min-h-screen flex-col items-center justify-center px-4">
      <div className="mx-auto max-w-4xl text-center">
        <h1 className="mb-4 text-5xl font-bold tracking-tight text-fd-foreground sm:text-6xl">
          clawft
        </h1>
        <p className="mb-12 text-lg text-fd-muted-foreground sm:text-xl">
          AI-native development framework with WeftOS kernel
        </p>

        <div className="grid gap-6 sm:grid-cols-2">
          <Link
            href="/docs/clawft"
            className="group rounded-xl border border-fd-border bg-fd-card p-8 text-left transition-colors hover:border-fd-primary hover:bg-fd-accent"
          >
            <div className="mb-3 text-sm font-medium uppercase tracking-wider text-fd-primary">
              Framework
            </div>
            <h2 className="mb-2 text-2xl font-semibold text-fd-card-foreground">
              clawft
            </h2>
            <p className="text-fd-muted-foreground">
              CLI, plugin system, LLM providers, tool sandbox, channels,
              skills, and deployment.
            </p>
          </Link>

          <Link
            href="/docs/weftos"
            className="group rounded-xl border border-fd-border bg-fd-card p-8 text-left transition-colors hover:border-fd-primary hover:bg-fd-accent"
          >
            <div className="mb-3 text-sm font-medium uppercase tracking-wider text-fd-primary">
              Kernel
            </div>
            <h2 className="mb-2 text-2xl font-semibold text-fd-card-foreground">
              WeftOS
            </h2>
            <p className="text-fd-muted-foreground">
              Process management, IPC, capabilities, ExoChain, governance,
              WASM sandbox, and mesh networking.
            </p>
          </Link>
        </div>

        <div className="mt-16 grid gap-4 sm:grid-cols-4">
          {[
            { label: 'CLI', desc: 'Ergonomic command interface' },
            { label: 'Plugins', desc: 'Extensible architecture' },
            { label: 'WASM', desc: 'Sandboxed tool execution' },
            { label: 'Mesh', desc: 'Distributed networking' },
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
