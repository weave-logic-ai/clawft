import { useEffect, useMemo } from "react";
import { useConfigStore } from "../stores/config-store";
import { Button } from "../components/ui/button";
import { Badge } from "../components/ui/badge";
import { Skeleton } from "../components/ui/skeleton";
import { Card, CardHeader, CardTitle, CardContent } from "../components/ui/card";
import { cn } from "../lib/utils";

const TABS = [
  { id: "general", label: "General" },
  { id: "agents", label: "Agents" },
  { id: "providers", label: "Providers" },
  { id: "channels", label: "Channels" },
  { id: "gateway", label: "Gateway" },
] as const;

function FormField({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex items-center justify-between py-3 border-b border-gray-100 dark:border-gray-700 last:border-0">
      <label className="text-sm font-medium text-gray-700 dark:text-gray-300">
        {label}
      </label>
      <div className="w-64">{children}</div>
    </div>
  );
}

function TextInput({
  value,
  onChange,
  type = "text",
}: {
  value: string | number;
  onChange: (val: string) => void;
  type?: string;
}) {
  return (
    <input
      type={type}
      value={value}
      onChange={(e) => onChange(e.target.value)}
      className="w-full rounded-md border border-gray-300 bg-white px-3 py-1.5 text-sm text-gray-900 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500 dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100"
    />
  );
}

function ToggleInput({
  checked,
  onChange,
}: {
  checked: boolean;
  onChange: (val: boolean) => void;
}) {
  return (
    <button
      type="button"
      onClick={() => onChange(!checked)}
      className={cn(
        "relative inline-flex h-6 w-11 items-center rounded-full transition-colors",
        checked ? "bg-blue-600" : "bg-gray-300 dark:bg-gray-600",
      )}
    >
      <span
        className={cn(
          "inline-block h-4 w-4 rounded-full bg-white transition-transform",
          checked ? "translate-x-6" : "translate-x-1",
        )}
      />
    </button>
  );
}

function AgentsTab({
  draft,
  onUpdate,
}: {
  draft: NonNullable<ReturnType<typeof useConfigStore.getState>["draft"]>;
  onUpdate: (path: string, value: unknown) => void;
}) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>Agent Defaults</CardTitle>
      </CardHeader>
      <CardContent>
        <FormField label="Model">
          <TextInput
            value={draft.agents.defaults.model}
            onChange={(v) => onUpdate("agents.defaults.model", v)}
          />
        </FormField>
        <FormField label="Max Tokens">
          <TextInput
            value={draft.agents.defaults.max_tokens}
            onChange={(v) =>
              onUpdate("agents.defaults.max_tokens", parseInt(v, 10) || 0)
            }
            type="number"
          />
        </FormField>
        <FormField label="Temperature">
          <TextInput
            value={draft.agents.defaults.temperature}
            onChange={(v) =>
              onUpdate(
                "agents.defaults.temperature",
                parseFloat(v) || 0,
              )
            }
            type="number"
          />
        </FormField>
      </CardContent>
    </Card>
  );
}

function ProvidersTab({
  draft,
  onUpdate,
}: {
  draft: NonNullable<ReturnType<typeof useConfigStore.getState>["draft"]>;
  onUpdate: (path: string, value: unknown) => void;
}) {
  return (
    <div className="space-y-4">
      {Object.entries(draft.providers).map(([name, provider]) => (
        <Card key={name}>
          <CardHeader>
            <div className="flex items-center justify-between">
              <CardTitle>{name}</CardTitle>
              <Badge
                variant={provider.enabled ? "success" : "secondary"}
              >
                {provider.enabled ? "enabled" : "disabled"}
              </Badge>
            </div>
          </CardHeader>
          <CardContent>
            <FormField label="API Base">
              <TextInput
                value={provider.api_base}
                onChange={(v) =>
                  onUpdate(`providers.${name}.api_base`, v)
                }
              />
            </FormField>
            <FormField label="API Key Set">
              <Badge
                variant={provider.api_key_set ? "success" : "outline"}
              >
                {provider.api_key_set ? "configured" : "not set"}
              </Badge>
            </FormField>
            <FormField label="Enabled">
              <ToggleInput
                checked={provider.enabled}
                onChange={(v) =>
                  onUpdate(`providers.${name}.enabled`, v)
                }
              />
            </FormField>
          </CardContent>
        </Card>
      ))}
    </div>
  );
}

function ChannelsTab({
  draft,
  onUpdate,
}: {
  draft: NonNullable<ReturnType<typeof useConfigStore.getState>["draft"]>;
  onUpdate: (path: string, value: unknown) => void;
}) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>Channel Configuration</CardTitle>
      </CardHeader>
      <CardContent>
        {Object.entries(draft.channels).map(([name, channel]) => (
          <FormField key={name} label={name}>
            <ToggleInput
              checked={channel.enabled}
              onChange={(v) =>
                onUpdate(`channels.${name}.enabled`, v)
              }
            />
          </FormField>
        ))}
      </CardContent>
    </Card>
  );
}

function GatewayTab({
  draft,
  onUpdate,
}: {
  draft: NonNullable<ReturnType<typeof useConfigStore.getState>["draft"]>;
  onUpdate: (path: string, value: unknown) => void;
}) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>Gateway Settings</CardTitle>
      </CardHeader>
      <CardContent>
        <FormField label="API Port">
          <TextInput
            value={draft.gateway.api_port}
            onChange={(v) =>
              onUpdate("gateway.api_port", parseInt(v, 10) || 0)
            }
            type="number"
          />
        </FormField>
        <FormField label="API Enabled">
          <ToggleInput
            checked={draft.gateway.api_enabled}
            onChange={(v) => onUpdate("gateway.api_enabled", v)}
          />
        </FormField>
      </CardContent>
    </Card>
  );
}

function ChangeSummary({
  config,
  draft,
}: {
  config: NonNullable<ReturnType<typeof useConfigStore.getState>["config"]>;
  draft: NonNullable<ReturnType<typeof useConfigStore.getState>["draft"]>;
}) {
  const changes = useMemo(() => {
    const diffs: { path: string; from: string; to: string }[] = [];

    function compare(a: unknown, b: unknown, path: string) {
      if (typeof a !== typeof b || typeof a !== "object" || a === null || b === null) {
        if (a !== b) {
          diffs.push({ path, from: String(a), to: String(b) });
        }
        return;
      }
      const objA = a as Record<string, unknown>;
      const objB = b as Record<string, unknown>;
      const keys = new Set([...Object.keys(objA), ...Object.keys(objB)]);
      for (const key of keys) {
        compare(objA[key], objB[key], `${path}.${key}`);
      }
    }

    compare(config, draft, "config");
    return diffs;
  }, [config, draft]);

  if (changes.length === 0) return null;

  return (
    <div className="rounded-md border border-yellow-300 bg-yellow-50 p-4 dark:border-yellow-700 dark:bg-yellow-900/20">
      <p className="mb-2 text-sm font-medium text-yellow-800 dark:text-yellow-300">
        Pending Changes ({changes.length}):
      </p>
      <div className="space-y-1 text-xs font-mono">
        {changes.map((c) => (
          <div key={c.path} className="text-yellow-700 dark:text-yellow-400">
            <span className="text-gray-500">{c.path}:</span>{" "}
            <span className="text-red-500 line-through">{c.from}</span>{" "}
            <span className="text-green-600">{c.to}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

export function ConfigPage() {
  const {
    config,
    draft,
    activeTab,
    loading,
    saving,
    hasChanges,
    setActiveTab,
    fetchConfig,
    updateDraft,
    saveConfig,
    resetDraft,
  } = useConfigStore();

  useEffect(() => {
    fetchConfig();
  }, [fetchConfig]);

  if (loading || !draft) {
    return (
      <div className="flex-1 overflow-auto p-6 space-y-6">
        <div>
          <Skeleton className="h-8 w-48" />
          <Skeleton className="mt-2 h-4 w-64" />
        </div>
        <Skeleton className="h-10 w-full max-w-lg" />
        <Skeleton className="h-64 w-full" />
      </div>
    );
  }

  return (
    <div className="flex-1 overflow-auto p-6 space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900 dark:text-gray-100">
            Configuration
          </h1>
          <p className="text-sm text-gray-500 dark:text-gray-400">
            Manage system configuration
          </p>
        </div>
        <div className="flex items-center gap-3">
          {hasChanges && (
            <Badge variant="default">unsaved changes</Badge>
          )}
          <Button
            variant="outline"
            size="sm"
            onClick={resetDraft}
            disabled={!hasChanges}
          >
            Reset
          </Button>
          <Button
            variant="default"
            size="sm"
            onClick={saveConfig}
            disabled={!hasChanges || saving}
          >
            {saving ? "Saving..." : "Save"}
          </Button>
        </div>
      </div>

      {/* Tabs */}
      <div className="flex gap-1 border-b border-gray-200 dark:border-gray-700">
        {TABS.map((tab) => (
          <button
            key={tab.id}
            onClick={() => setActiveTab(tab.id)}
            className={cn(
              "px-4 py-2 text-sm font-medium transition-colors border-b-2 -mb-px",
              activeTab === tab.id
                ? "border-blue-600 text-blue-600 dark:border-blue-400 dark:text-blue-400"
                : "border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200",
            )}
          >
            {tab.label}
          </button>
        ))}
      </div>

      {/* Change summary */}
      {hasChanges && config && (
        <ChangeSummary config={config} draft={draft} />
      )}

      {/* Tab content */}
      <div>
        {activeTab === "general" && (
          <Card>
            <CardHeader>
              <CardTitle>General</CardTitle>
            </CardHeader>
            <CardContent>
              <p className="text-sm text-gray-500 dark:text-gray-400">
                General settings are derived from other configuration sections. Use the specific tabs to modify values.
              </p>
            </CardContent>
          </Card>
        )}
        {activeTab === "agents" && (
          <AgentsTab draft={draft} onUpdate={updateDraft} />
        )}
        {activeTab === "providers" && (
          <ProvidersTab draft={draft} onUpdate={updateDraft} />
        )}
        {activeTab === "channels" && (
          <ChannelsTab draft={draft} onUpdate={updateDraft} />
        )}
        {activeTab === "gateway" && (
          <GatewayTab draft={draft} onUpdate={updateDraft} />
        )}
      </div>
    </div>
  );
}
