/**
 * BudgetBlock — displays per-agent token usage, cost, and budget remaining.
 * Wired to StateStore key `/kernel/metrics/cost`.
 */

import { registerBlock } from '../engine';
import { useStateStore } from '../engine/StateStore';
import type { BlockComponentProps } from '../engine';

interface AgentCostEntry {
  agent_id: string;
  tokens_used: number;
  cost_usd: number;
  budget_remaining_usd: number;
}

interface CostData {
  agents: AgentCostEntry[];
  total_cost_usd: number;
  total_budget_usd: number;
}

function isCostData(v: unknown): v is CostData {
  if (typeof v !== 'object' || v === null) return false;
  const obj = v as Record<string, unknown>;
  return Array.isArray(obj.agents) && typeof obj.total_cost_usd === 'number';
}

function formatUsd(amount: number): string {
  return `$${amount.toFixed(4)}`;
}

function formatTokens(count: number): string {
  if (count >= 1_000_000) return `${(count / 1_000_000).toFixed(1)}M`;
  if (count >= 1_000) return `${(count / 1_000).toFixed(1)}k`;
  return String(count);
}

function budgetPercent(cost: number, budget: number): number {
  if (budget <= 0) return 0;
  return Math.min(100, (cost / budget) * 100);
}

function barColor(percent: number): string {
  if (percent >= 90) return 'bg-red-500';
  if (percent >= 70) return 'bg-amber-500';
  return 'bg-emerald-500';
}

function BudgetBlock({ resolvedProps }: BlockComponentProps) {
  const title = (resolvedProps.title as string) ?? 'Budget & Cost';
  const stateKey = (resolvedProps.stateKey as string) ?? '/kernel/metrics/cost';
  const rawData = useStateStore((s) => s.data[stateKey]);

  if (!rawData || !isCostData(rawData)) {
    return (
      <div className="bg-gray-800/60 rounded-lg p-4 border border-gray-700">
        <p className="text-sm font-medium text-gray-200 mb-2">{title}</p>
        <p className="text-xs text-gray-500">No cost data available. Connect to a running kernel to see budget metrics.</p>
      </div>
    );
  }

  const data = rawData;
  const totalPercent = budgetPercent(data.total_cost_usd, data.total_budget_usd);

  return (
    <div className="bg-gray-800/60 rounded-lg p-4 border border-gray-700">
      {/* Header */}
      <div className="flex items-center justify-between mb-3">
        <p className="text-sm font-medium text-gray-200">{title}</p>
        <span className="text-xs text-gray-400">
          {formatUsd(data.total_cost_usd)} / {formatUsd(data.total_budget_usd)}
        </span>
      </div>

      {/* Total budget bar */}
      <div className="mb-4">
        <div className="flex items-center justify-between text-xs text-gray-400 mb-1">
          <span>Total spend</span>
          <span>{totalPercent.toFixed(1)}%</span>
        </div>
        <div className="w-full h-2 bg-gray-700 rounded-full overflow-hidden">
          <div
            className={`h-full rounded-full transition-all duration-300 ${barColor(totalPercent)}`}
            style={{ width: `${totalPercent}%` }}
          />
        </div>
      </div>

      {/* Per-agent table */}
      {data.agents.length > 0 && (
        <div className="overflow-x-auto">
          <table className="w-full text-xs">
            <thead>
              <tr className="text-gray-400 border-b border-gray-700">
                <th className="text-left py-1 pr-3 font-medium">Agent</th>
                <th className="text-right py-1 px-2 font-medium">Tokens</th>
                <th className="text-right py-1 px-2 font-medium">Cost</th>
                <th className="text-right py-1 pl-2 font-medium">Remaining</th>
              </tr>
            </thead>
            <tbody>
              {data.agents.map((agent) => {
                const agentTotal = agent.cost_usd + agent.budget_remaining_usd;
                const agentPercent = budgetPercent(agent.cost_usd, agentTotal);
                return (
                  <tr key={agent.agent_id} className="border-b border-gray-800 last:border-0">
                    <td className="py-1.5 pr-3">
                      <div className="flex items-center gap-2">
                        <div className={`w-1.5 h-1.5 rounded-full ${barColor(agentPercent)}`} />
                        <span className="text-gray-200 font-mono">{agent.agent_id}</span>
                      </div>
                    </td>
                    <td className="text-right py-1.5 px-2 text-gray-300 font-mono">
                      {formatTokens(agent.tokens_used)}
                    </td>
                    <td className="text-right py-1.5 px-2 text-gray-300 font-mono">
                      {formatUsd(agent.cost_usd)}
                    </td>
                    <td className="text-right py-1.5 pl-2 text-gray-300 font-mono">
                      {formatUsd(agent.budget_remaining_usd)}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

registerBlock('Budget', BudgetBlock);
export default BudgetBlock;
