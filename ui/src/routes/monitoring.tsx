import { useEffect } from "react";
import { useMonitoringStore } from "../stores/monitoring-store";
import { Card, CardHeader, CardTitle, CardContent } from "../components/ui/card";
import { Badge } from "../components/ui/badge";
import { Skeleton } from "../components/ui/skeleton";
import { formatRelativeTime } from "../lib/utils";

function formatNumber(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return String(n);
}

function SummaryCards() {
  const { tokenUsage, costs } = useMonitoringStore();

  if (!tokenUsage || !costs) {
    return (
      <div className="grid grid-cols-1 gap-4 md:grid-cols-4">
        {Array.from({ length: 4 }).map((_, i) => (
          <Card key={i}>
            <CardHeader>
              <Skeleton className="h-4 w-24" />
            </CardHeader>
            <CardContent>
              <Skeleton className="h-8 w-20" />
            </CardContent>
          </Card>
        ))}
      </div>
    );
  }

  const cards = [
    { label: "Total Input Tokens", value: formatNumber(tokenUsage.total_input) },
    { label: "Total Output Tokens", value: formatNumber(tokenUsage.total_output) },
    { label: "Total Requests", value: formatNumber(tokenUsage.total_requests) },
    { label: "Total Cost", value: `$${costs.total_cost_usd.toFixed(3)}` },
  ];

  return (
    <div className="grid grid-cols-1 gap-4 md:grid-cols-4">
      {cards.map((card) => (
        <Card key={card.label}>
          <CardHeader>
            <p className="text-sm font-medium text-gray-500 dark:text-gray-400">{card.label}</p>
          </CardHeader>
          <CardContent>
            <p className="text-2xl font-bold text-gray-900 dark:text-gray-100">{card.value}</p>
          </CardContent>
        </Card>
      ))}
    </div>
  );
}

function UsageByProviderTable() {
  const { tokenUsage, loading } = useMonitoringStore();

  return (
    <Card>
      <CardHeader>
        <CardTitle>Usage by Provider</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="overflow-hidden rounded-lg border border-gray-200 dark:border-gray-700">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-gray-200 bg-gray-50 dark:border-gray-700 dark:bg-gray-800/50">
                <th className="px-4 py-3 text-left font-medium text-gray-600 dark:text-gray-400">Provider</th>
                <th className="px-4 py-3 text-left font-medium text-gray-600 dark:text-gray-400">Model</th>
                <th className="px-4 py-3 text-right font-medium text-gray-600 dark:text-gray-400">Input</th>
                <th className="px-4 py-3 text-right font-medium text-gray-600 dark:text-gray-400">Output</th>
                <th className="px-4 py-3 text-right font-medium text-gray-600 dark:text-gray-400">Total</th>
                <th className="px-4 py-3 text-right font-medium text-gray-600 dark:text-gray-400">Requests</th>
              </tr>
            </thead>
            <tbody>
              {loading || !tokenUsage ? (
                Array.from({ length: 2 }).map((_, i) => (
                  <tr key={i} className="border-b border-gray-100 dark:border-gray-700">
                    {Array.from({ length: 6 }).map((_, j) => (
                      <td key={j} className="px-4 py-3"><Skeleton className="h-4 w-16" /></td>
                    ))}
                  </tr>
                ))
              ) : (
                tokenUsage.by_provider.map((p) => (
                  <tr key={`${p.provider}-${p.model}`} className="border-b border-gray-100 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-800/50">
                    <td className="px-4 py-3 font-medium text-gray-900 dark:text-gray-100">{p.provider}</td>
                    <td className="px-4 py-3">
                      <Badge variant="outline" className="text-xs">{p.model}</Badge>
                    </td>
                    <td className="px-4 py-3 text-right font-mono text-xs text-gray-500 dark:text-gray-400">
                      {formatNumber(p.input_tokens)}
                    </td>
                    <td className="px-4 py-3 text-right font-mono text-xs text-gray-500 dark:text-gray-400">
                      {formatNumber(p.output_tokens)}
                    </td>
                    <td className="px-4 py-3 text-right font-mono text-xs font-medium text-gray-900 dark:text-gray-100">
                      {formatNumber(p.total_tokens)}
                    </td>
                    <td className="px-4 py-3 text-right font-mono text-xs text-gray-500 dark:text-gray-400">
                      {p.request_count}
                    </td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </div>
      </CardContent>
    </Card>
  );
}

function UsageBySessionTable() {
  const { tokenUsage, loading } = useMonitoringStore();

  return (
    <Card>
      <CardHeader>
        <CardTitle>Usage by Session</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="overflow-hidden rounded-lg border border-gray-200 dark:border-gray-700">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-gray-200 bg-gray-50 dark:border-gray-700 dark:bg-gray-800/50">
                <th className="px-4 py-3 text-left font-medium text-gray-600 dark:text-gray-400">Session</th>
                <th className="px-4 py-3 text-right font-medium text-gray-600 dark:text-gray-400">Input Tokens</th>
                <th className="px-4 py-3 text-right font-medium text-gray-600 dark:text-gray-400">Output Tokens</th>
                <th className="px-4 py-3 text-right font-medium text-gray-600 dark:text-gray-400">Requests</th>
              </tr>
            </thead>
            <tbody>
              {loading || !tokenUsage ? (
                Array.from({ length: 3 }).map((_, i) => (
                  <tr key={i} className="border-b border-gray-100 dark:border-gray-700">
                    {Array.from({ length: 4 }).map((_, j) => (
                      <td key={j} className="px-4 py-3"><Skeleton className="h-4 w-16" /></td>
                    ))}
                  </tr>
                ))
              ) : (
                tokenUsage.by_session.map((s) => (
                  <tr key={s.session_key} className="border-b border-gray-100 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-800/50">
                    <td className="px-4 py-3 font-mono text-xs text-gray-900 dark:text-gray-100">{s.session_key}</td>
                    <td className="px-4 py-3 text-right font-mono text-xs text-gray-500 dark:text-gray-400">
                      {formatNumber(s.input_tokens)}
                    </td>
                    <td className="px-4 py-3 text-right font-mono text-xs text-gray-500 dark:text-gray-400">
                      {formatNumber(s.output_tokens)}
                    </td>
                    <td className="px-4 py-3 text-right font-mono text-xs text-gray-500 dark:text-gray-400">
                      {s.request_count}
                    </td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </div>
      </CardContent>
    </Card>
  );
}

function CostByTierTable() {
  const { costs, loading } = useMonitoringStore();

  return (
    <Card>
      <CardHeader>
        <CardTitle>Cost by Tier (ADR-026)</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="overflow-hidden rounded-lg border border-gray-200 dark:border-gray-700">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-gray-200 bg-gray-50 dark:border-gray-700 dark:bg-gray-800/50">
                <th className="px-4 py-3 text-left font-medium text-gray-600 dark:text-gray-400">Tier</th>
                <th className="px-4 py-3 text-left font-medium text-gray-600 dark:text-gray-400">Handler</th>
                <th className="px-4 py-3 text-right font-medium text-gray-600 dark:text-gray-400">Requests</th>
                <th className="px-4 py-3 text-right font-medium text-gray-600 dark:text-gray-400">Cost</th>
              </tr>
            </thead>
            <tbody>
              {loading || !costs ? (
                Array.from({ length: 3 }).map((_, i) => (
                  <tr key={i} className="border-b border-gray-100 dark:border-gray-700">
                    {Array.from({ length: 4 }).map((_, j) => (
                      <td key={j} className="px-4 py-3"><Skeleton className="h-4 w-16" /></td>
                    ))}
                  </tr>
                ))
              ) : (
                costs.by_tier.map((t) => (
                  <tr key={t.tier} className="border-b border-gray-100 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-800/50">
                    <td className="px-4 py-3 font-medium text-gray-900 dark:text-gray-100">Tier {t.tier}</td>
                    <td className="px-4 py-3 text-gray-700 dark:text-gray-300">{t.label}</td>
                    <td className="px-4 py-3 text-right font-mono text-xs text-gray-500 dark:text-gray-400">
                      {formatNumber(t.request_count)}
                    </td>
                    <td className="px-4 py-3 text-right font-mono text-xs font-medium text-gray-900 dark:text-gray-100">
                      {t.total_cost_usd === 0 ? "$0.00" : `$${t.total_cost_usd.toFixed(3)}`}
                    </td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </div>
      </CardContent>
    </Card>
  );
}

function PipelineRunsTable() {
  const { pipelineRuns, loading } = useMonitoringStore();

  return (
    <Card>
      <CardHeader>
        <CardTitle>Pipeline Runs</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="overflow-hidden rounded-lg border border-gray-200 dark:border-gray-700">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-gray-200 bg-gray-50 dark:border-gray-700 dark:bg-gray-800/50">
                <th className="px-4 py-3 text-left font-medium text-gray-600 dark:text-gray-400">Run ID</th>
                <th className="px-4 py-3 text-left font-medium text-gray-600 dark:text-gray-400">Session</th>
                <th className="px-4 py-3 text-left font-medium text-gray-600 dark:text-gray-400">Model</th>
                <th className="px-4 py-3 text-right font-medium text-gray-600 dark:text-gray-400">Complexity</th>
                <th className="px-4 py-3 text-right font-medium text-gray-600 dark:text-gray-400">Latency</th>
                <th className="px-4 py-3 text-left font-medium text-gray-600 dark:text-gray-400">Status</th>
                <th className="px-4 py-3 text-left font-medium text-gray-600 dark:text-gray-400">Time</th>
              </tr>
            </thead>
            <tbody>
              {loading ? (
                Array.from({ length: 5 }).map((_, i) => (
                  <tr key={i} className="border-b border-gray-100 dark:border-gray-700">
                    {Array.from({ length: 7 }).map((_, j) => (
                      <td key={j} className="px-4 py-3"><Skeleton className="h-4 w-16" /></td>
                    ))}
                  </tr>
                ))
              ) : pipelineRuns.length === 0 ? (
                <tr>
                  <td colSpan={7} className="px-4 py-12 text-center text-gray-500 dark:text-gray-400">
                    No pipeline runs recorded.
                  </td>
                </tr>
              ) : (
                pipelineRuns.map((run) => (
                  <tr key={run.id} className="border-b border-gray-100 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-800/50">
                    <td className="px-4 py-3 font-mono text-xs text-gray-900 dark:text-gray-100">{run.id}</td>
                    <td className="px-4 py-3 font-mono text-xs text-gray-500 dark:text-gray-400">{run.session_key}</td>
                    <td className="px-4 py-3">
                      <Badge variant="outline" className="text-xs">{run.model}</Badge>
                    </td>
                    <td className="px-4 py-3 text-right font-mono text-xs text-gray-500 dark:text-gray-400">
                      {(run.complexity * 100).toFixed(0)}%
                    </td>
                    <td className="px-4 py-3 text-right font-mono text-xs text-gray-500 dark:text-gray-400">
                      {run.latency_ms}ms
                    </td>
                    <td className="px-4 py-3">
                      <Badge variant={run.status === "success" ? "success" : "destructive"}>
                        {run.status}
                      </Badge>
                    </td>
                    <td className="px-4 py-3 text-gray-500 dark:text-gray-400">
                      {formatRelativeTime(run.timestamp)}
                    </td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </div>
      </CardContent>
    </Card>
  );
}

export function MonitoringPage() {
  const { fetchAll } = useMonitoringStore();

  useEffect(() => {
    fetchAll();
  }, [fetchAll]);

  return (
    <div className="flex-1 overflow-auto p-6 space-y-6">
      <div>
        <h1 className="text-2xl font-bold text-gray-900 dark:text-gray-100">
          Monitoring
        </h1>
        <p className="text-sm text-gray-500 dark:text-gray-400">
          Token usage, costs, and pipeline run telemetry
        </p>
      </div>

      <SummaryCards />

      <div className="grid grid-cols-1 gap-6 lg:grid-cols-2">
        <UsageByProviderTable />
        <UsageBySessionTable />
      </div>

      <CostByTierTable />
      <PipelineRunsTable />
    </div>
  );
}
