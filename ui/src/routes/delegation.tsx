import { useEffect, useState } from "react";
import { useDelegationStore } from "../stores/delegation-store";
import { Badge } from "../components/ui/badge";
import { Button } from "../components/ui/button";
import { Skeleton } from "../components/ui/skeleton";
import { Dialog, DialogHeader, DialogTitle, DialogFooter } from "../components/ui/dialog";
import { formatRelativeTime } from "../lib/utils";
import type { ActiveDelegation, DelegationRule } from "../lib/types";

function delegationStatusVariant(
  status: ActiveDelegation["status"],
): "success" | "secondary" | "destructive" | "default" {
  switch (status) {
    case "completed":
      return "success";
    case "running":
      return "default";
    case "pending":
      return "secondary";
    case "failed":
      return "destructive";
  }
}

function complexityColor(c: number): string {
  if (c < 0.2) return "text-green-600 dark:text-green-400";
  if (c < 0.5) return "text-yellow-600 dark:text-yellow-400";
  return "text-red-600 dark:text-red-400";
}

// ── Active Delegations Tab ─────────────────────────────────────

function ActiveTab() {
  const { activeDelegations, loading, fetchActive } = useDelegationStore();

  useEffect(() => {
    fetchActive();
  }, [fetchActive]);

  if (loading) {
    return (
      <div className="overflow-hidden rounded-lg border border-gray-200 dark:border-gray-700">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-gray-200 bg-gray-50 dark:border-gray-700 dark:bg-gray-800/50">
              <th className="px-4 py-3 text-left font-medium text-gray-600 dark:text-gray-400">Task ID</th>
              <th className="px-4 py-3 text-left font-medium text-gray-600 dark:text-gray-400">Session</th>
              <th className="px-4 py-3 text-left font-medium text-gray-600 dark:text-gray-400">Target</th>
              <th className="px-4 py-3 text-left font-medium text-gray-600 dark:text-gray-400">Tool</th>
              <th className="px-4 py-3 text-left font-medium text-gray-600 dark:text-gray-400">Status</th>
              <th className="px-4 py-3 text-left font-medium text-gray-600 dark:text-gray-400">Latency</th>
              <th className="px-4 py-3 text-left font-medium text-gray-600 dark:text-gray-400">Complexity</th>
            </tr>
          </thead>
          <tbody>
            {Array.from({ length: 3 }).map((_, i) => (
              <tr key={i} className="border-b border-gray-100 dark:border-gray-700">
                {Array.from({ length: 7 }).map((_, j) => (
                  <td key={j} className="px-4 py-3"><Skeleton className="h-4 w-20" /></td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    );
  }

  if (activeDelegations.length === 0) {
    return (
      <div className="py-12 text-center">
        <p className="text-gray-500 dark:text-gray-400">No active delegations.</p>
      </div>
    );
  }

  return (
    <div className="overflow-hidden rounded-lg border border-gray-200 dark:border-gray-700">
      <table className="w-full text-sm">
        <thead>
          <tr className="border-b border-gray-200 bg-gray-50 dark:border-gray-700 dark:bg-gray-800/50">
            <th className="px-4 py-3 text-left font-medium text-gray-600 dark:text-gray-400">Task ID</th>
            <th className="px-4 py-3 text-left font-medium text-gray-600 dark:text-gray-400">Session</th>
            <th className="px-4 py-3 text-left font-medium text-gray-600 dark:text-gray-400">Target</th>
            <th className="px-4 py-3 text-left font-medium text-gray-600 dark:text-gray-400">Tool</th>
            <th className="px-4 py-3 text-left font-medium text-gray-600 dark:text-gray-400">Status</th>
            <th className="px-4 py-3 text-left font-medium text-gray-600 dark:text-gray-400">Latency</th>
            <th className="px-4 py-3 text-left font-medium text-gray-600 dark:text-gray-400">Complexity</th>
          </tr>
        </thead>
        <tbody>
          {activeDelegations.map((d) => (
            <tr key={d.task_id} className="border-b border-gray-100 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-800/50">
              <td className="px-4 py-3 font-mono text-xs text-gray-900 dark:text-gray-100">{d.task_id}</td>
              <td className="px-4 py-3 font-mono text-xs text-gray-500 dark:text-gray-400">{d.session_key}</td>
              <td className="px-4 py-3">
                <Badge variant="outline" className="text-xs">{d.target}</Badge>
              </td>
              <td className="px-4 py-3 text-gray-700 dark:text-gray-300">{d.tool_name}</td>
              <td className="px-4 py-3">
                <Badge variant={delegationStatusVariant(d.status)}>{d.status}</Badge>
              </td>
              <td className="px-4 py-3 font-mono text-xs text-gray-500 dark:text-gray-400">
                {d.latency_ms != null ? `${d.latency_ms}ms` : "-"}
              </td>
              <td className={`px-4 py-3 font-mono text-xs ${complexityColor(d.complexity)}`}>
                {(d.complexity * 100).toFixed(0)}%
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

// ── Rules Tab ──────────────────────────────────────────────────

function RulesTab() {
  const { rules, loading, fetchRules, upsertRule, deleteRule } = useDelegationStore();
  const [dialogOpen, setDialogOpen] = useState(false);
  const [editingRule, setEditingRule] = useState<DelegationRule | null>(null);
  const [formName, setFormName] = useState("");
  const [formPattern, setFormPattern] = useState("");
  const [formTarget, setFormTarget] = useState("");
  const [formThreshold, setFormThreshold] = useState("0.3");
  const [formPriority, setFormPriority] = useState("1");

  useEffect(() => {
    fetchRules();
  }, [fetchRules]);

  const openCreateDialog = () => {
    setEditingRule(null);
    setFormName("");
    setFormPattern("");
    setFormTarget("");
    setFormThreshold("0.3");
    setFormPriority("1");
    setDialogOpen(true);
  };

  const openEditDialog = (rule: DelegationRule) => {
    setEditingRule(rule);
    setFormName(rule.name);
    setFormPattern(rule.pattern);
    setFormTarget(rule.target);
    setFormThreshold(String(rule.complexity_threshold));
    setFormPriority(String(rule.priority));
    setDialogOpen(true);
  };

  const handleSave = async () => {
    if (!formName.trim() || !formPattern.trim() || !formTarget.trim()) return;
    await upsertRule({
      name: formName,
      pattern: formPattern,
      target: formTarget,
      complexity_threshold: parseFloat(formThreshold) || 0.3,
      enabled: editingRule?.enabled ?? true,
      priority: parseInt(formPriority, 10) || 1,
    });
    setDialogOpen(false);
  };

  if (loading && rules.length === 0) {
    return (
      <div className="space-y-2">
        {Array.from({ length: 3 }).map((_, i) => (
          <Skeleton key={i} className="h-16 w-full" />
        ))}
      </div>
    );
  }

  return (
    <>
      <div className="mb-4 flex justify-end">
        <Button variant="default" size="sm" onClick={openCreateDialog}>
          Add Rule
        </Button>
      </div>

      <div className="overflow-hidden rounded-lg border border-gray-200 dark:border-gray-700">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-gray-200 bg-gray-50 dark:border-gray-700 dark:bg-gray-800/50">
              <th className="px-4 py-3 text-left font-medium text-gray-600 dark:text-gray-400">Name</th>
              <th className="px-4 py-3 text-left font-medium text-gray-600 dark:text-gray-400">Pattern</th>
              <th className="px-4 py-3 text-left font-medium text-gray-600 dark:text-gray-400">Target</th>
              <th className="px-4 py-3 text-left font-medium text-gray-600 dark:text-gray-400">Threshold</th>
              <th className="px-4 py-3 text-left font-medium text-gray-600 dark:text-gray-400">Priority</th>
              <th className="px-4 py-3 text-left font-medium text-gray-600 dark:text-gray-400">Status</th>
              <th className="px-4 py-3 text-right font-medium text-gray-600 dark:text-gray-400">Actions</th>
            </tr>
          </thead>
          <tbody>
            {rules.length === 0 ? (
              <tr>
                <td colSpan={7} className="px-4 py-12 text-center text-gray-500 dark:text-gray-400">
                  No delegation rules configured.
                </td>
              </tr>
            ) : (
              rules.map((rule) => (
                <tr key={rule.name} className="border-b border-gray-100 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-800/50">
                  <td className="px-4 py-3 font-medium text-gray-900 dark:text-gray-100">{rule.name}</td>
                  <td className="px-4 py-3 font-mono text-xs text-gray-500 dark:text-gray-400">{rule.pattern}</td>
                  <td className="px-4 py-3">
                    <Badge variant="outline" className="text-xs">{rule.target}</Badge>
                  </td>
                  <td className="px-4 py-3 font-mono text-xs text-gray-500 dark:text-gray-400">
                    {(rule.complexity_threshold * 100).toFixed(0)}%
                  </td>
                  <td className="px-4 py-3 text-gray-500 dark:text-gray-400">{rule.priority}</td>
                  <td className="px-4 py-3">
                    <Badge variant={rule.enabled ? "success" : "secondary"}>
                      {rule.enabled ? "enabled" : "disabled"}
                    </Badge>
                  </td>
                  <td className="px-4 py-3 text-right">
                    <div className="flex items-center justify-end gap-2">
                      <Button variant="ghost" size="sm" onClick={() => openEditDialog(rule)}>
                        Edit
                      </Button>
                      <Button variant="ghost" size="sm" onClick={() => deleteRule(rule.name)}>
                        Delete
                      </Button>
                    </div>
                  </td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>

      <Dialog open={dialogOpen} onClose={() => setDialogOpen(false)}>
        <DialogHeader>
          <DialogTitle>{editingRule ? "Edit Rule" : "Add Delegation Rule"}</DialogTitle>
        </DialogHeader>
        <div className="space-y-4">
          <div>
            <label className="mb-1 block text-sm font-medium text-gray-700 dark:text-gray-300">Name</label>
            <input
              type="text"
              value={formName}
              onChange={(e) => setFormName(e.target.value)}
              placeholder="e.g. simple-transforms"
              disabled={!!editingRule}
              className="w-full rounded-md border border-gray-300 bg-white px-3 py-2 text-sm text-gray-900 placeholder-gray-400 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500 disabled:opacity-50 dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100 dark:placeholder-gray-500"
            />
          </div>
          <div>
            <label className="mb-1 block text-sm font-medium text-gray-700 dark:text-gray-300">Pattern (glob)</label>
            <input
              type="text"
              value={formPattern}
              onChange={(e) => setFormPattern(e.target.value)}
              placeholder="format-*|lint-*"
              className="w-full rounded-md border border-gray-300 bg-white px-3 py-2 text-sm font-mono text-gray-900 placeholder-gray-400 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500 dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100 dark:placeholder-gray-500"
            />
          </div>
          <div>
            <label className="mb-1 block text-sm font-medium text-gray-700 dark:text-gray-300">Target</label>
            <input
              type="text"
              value={formTarget}
              onChange={(e) => setFormTarget(e.target.value)}
              placeholder="claude-haiku-3.5"
              className="w-full rounded-md border border-gray-300 bg-white px-3 py-2 text-sm text-gray-900 placeholder-gray-400 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500 dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100 dark:placeholder-gray-500"
            />
          </div>
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="mb-1 block text-sm font-medium text-gray-700 dark:text-gray-300">Complexity Threshold</label>
              <input
                type="number"
                value={formThreshold}
                onChange={(e) => setFormThreshold(e.target.value)}
                step="0.1"
                min="0"
                max="1"
                className="w-full rounded-md border border-gray-300 bg-white px-3 py-2 text-sm text-gray-900 placeholder-gray-400 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500 dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100 dark:placeholder-gray-500"
              />
            </div>
            <div>
              <label className="mb-1 block text-sm font-medium text-gray-700 dark:text-gray-300">Priority</label>
              <input
                type="number"
                value={formPriority}
                onChange={(e) => setFormPriority(e.target.value)}
                min="1"
                className="w-full rounded-md border border-gray-300 bg-white px-3 py-2 text-sm text-gray-900 placeholder-gray-400 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500 dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100 dark:placeholder-gray-500"
              />
            </div>
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" size="sm" onClick={() => setDialogOpen(false)}>Cancel</Button>
          <Button
            variant="default"
            size="sm"
            onClick={handleSave}
            disabled={!formName.trim() || !formPattern.trim() || !formTarget.trim()}
          >
            {editingRule ? "Update" : "Create"}
          </Button>
        </DialogFooter>
      </Dialog>
    </>
  );
}

// ── History Tab ────────────────────────────────────────────────

function HistoryTab() {
  const { history, historyTotal, loading, fetchHistory, historyFilter, setHistoryFilter } =
    useDelegationStore();
  const [sessionFilter, setSessionFilter] = useState(historyFilter.session ?? "");
  const [targetFilter, setTargetFilter] = useState(historyFilter.target ?? "");

  useEffect(() => {
    fetchHistory();
  }, [fetchHistory]);

  const applyFilters = () => {
    setHistoryFilter({
      session: sessionFilter || undefined,
      target: targetFilter || undefined,
    });
    fetchHistory();
  };

  return (
    <div className="space-y-4">
      {/* Filters */}
      <div className="flex items-end gap-3">
        <div>
          <label className="mb-1 block text-xs font-medium text-gray-600 dark:text-gray-400">Session</label>
          <input
            type="text"
            value={sessionFilter}
            onChange={(e) => setSessionFilter(e.target.value)}
            placeholder="Filter by session"
            className="rounded-md border border-gray-300 bg-white px-3 py-1.5 text-sm text-gray-900 placeholder-gray-400 focus:border-blue-500 focus:outline-none dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100 dark:placeholder-gray-500"
          />
        </div>
        <div>
          <label className="mb-1 block text-xs font-medium text-gray-600 dark:text-gray-400">Target</label>
          <input
            type="text"
            value={targetFilter}
            onChange={(e) => setTargetFilter(e.target.value)}
            placeholder="Filter by target"
            className="rounded-md border border-gray-300 bg-white px-3 py-1.5 text-sm text-gray-900 placeholder-gray-400 focus:border-blue-500 focus:outline-none dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100 dark:placeholder-gray-500"
          />
        </div>
        <Button variant="outline" size="sm" onClick={applyFilters}>
          Apply
        </Button>
      </div>

      {/* Table */}
      <div className="overflow-hidden rounded-lg border border-gray-200 dark:border-gray-700">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-gray-200 bg-gray-50 dark:border-gray-700 dark:bg-gray-800/50">
              <th className="px-4 py-3 text-left font-medium text-gray-600 dark:text-gray-400">Task ID</th>
              <th className="px-4 py-3 text-left font-medium text-gray-600 dark:text-gray-400">Session</th>
              <th className="px-4 py-3 text-left font-medium text-gray-600 dark:text-gray-400">Target</th>
              <th className="px-4 py-3 text-left font-medium text-gray-600 dark:text-gray-400">Tool</th>
              <th className="px-4 py-3 text-left font-medium text-gray-600 dark:text-gray-400">Status</th>
              <th className="px-4 py-3 text-left font-medium text-gray-600 dark:text-gray-400">Latency</th>
              <th className="px-4 py-3 text-left font-medium text-gray-600 dark:text-gray-400">Started</th>
            </tr>
          </thead>
          <tbody>
            {loading ? (
              Array.from({ length: 5 }).map((_, i) => (
                <tr key={i} className="border-b border-gray-100 dark:border-gray-700">
                  {Array.from({ length: 7 }).map((_, j) => (
                    <td key={j} className="px-4 py-3"><Skeleton className="h-4 w-20" /></td>
                  ))}
                </tr>
              ))
            ) : history.length === 0 ? (
              <tr>
                <td colSpan={7} className="px-4 py-12 text-center text-gray-500 dark:text-gray-400">
                  No delegation history found.
                </td>
              </tr>
            ) : (
              history.map((entry) => (
                <tr key={entry.task_id} className="border-b border-gray-100 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-800/50">
                  <td className="px-4 py-3 font-mono text-xs text-gray-900 dark:text-gray-100">{entry.task_id}</td>
                  <td className="px-4 py-3 font-mono text-xs text-gray-500 dark:text-gray-400">{entry.session_key}</td>
                  <td className="px-4 py-3">
                    <Badge variant="outline" className="text-xs">{entry.target}</Badge>
                  </td>
                  <td className="px-4 py-3 text-gray-700 dark:text-gray-300">{entry.tool_name}</td>
                  <td className="px-4 py-3">
                    <Badge variant={delegationStatusVariant(entry.status)}>{entry.status}</Badge>
                  </td>
                  <td className="px-4 py-3 font-mono text-xs text-gray-500 dark:text-gray-400">
                    {entry.latency_ms != null ? `${entry.latency_ms}ms` : "-"}
                  </td>
                  <td className="px-4 py-3 text-gray-500 dark:text-gray-400">
                    {formatRelativeTime(entry.started_at)}
                  </td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>

      {historyTotal > 0 && (
        <p className="text-right text-xs text-gray-500 dark:text-gray-400">
          Showing {history.length} of {historyTotal} entries
        </p>
      )}
    </div>
  );
}

// ── Main Page ──────────────────────────────────────────────────

const tabs = [
  { key: "active" as const, label: "Active" },
  { key: "rules" as const, label: "Rules" },
  { key: "history" as const, label: "History" },
];

export function DelegationPage() {
  const { activeTab, setActiveTab, activeDelegations } = useDelegationStore();

  return (
    <div className="flex-1 overflow-auto p-6 space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900 dark:text-gray-100">
            Delegation Monitor
          </h1>
          <p className="text-sm text-gray-500 dark:text-gray-400">
            View active delegations, manage rules, and browse history
          </p>
        </div>
        {activeDelegations.length > 0 && (
          <Badge variant="default">
            {activeDelegations.filter((d) => d.status === "running").length} running
          </Badge>
        )}
      </div>

      {/* Tab bar */}
      <div className="flex gap-1 rounded-lg border border-gray-200 bg-gray-50 p-1 dark:border-gray-700 dark:bg-gray-800">
        {tabs.map((tab) => (
          <button
            key={tab.key}
            onClick={() => setActiveTab(tab.key)}
            className={`rounded-md px-4 py-1.5 text-sm font-medium transition-colors ${
              activeTab === tab.key
                ? "bg-white text-gray-900 shadow-sm dark:bg-gray-700 dark:text-gray-100"
                : "text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300"
            }`}
          >
            {tab.label}
          </button>
        ))}
      </div>

      {/* Tab content */}
      {activeTab === "active" && <ActiveTab />}
      {activeTab === "rules" && <RulesTab />}
      {activeTab === "history" && <HistoryTab />}
    </div>
  );
}
