import { useState, useEffect, useCallback, useMemo } from "react";
import { Link, Outlet, useNavigate } from "@tanstack/react-router";
import { useThemeStore } from "../../stores/theme-store";
import { useAgentStore } from "../../stores/agent-store";
import { useSkillsStore } from "../../stores/skills-store";
import { useChannelsStore } from "../../stores/channels-store";
import { wsClient } from "../../lib/ws-client";
import { cn } from "../../lib/utils";
import { Badge } from "../ui/badge";
import { VoiceStatusBar } from "../voice/status-bar";
import { TalkModeOverlay } from "../voice/talk-overlay";
import { useBackend } from "../../lib/use-backend.ts";
import type { BackendCapabilities } from "../../lib/backend-adapter.ts";
import { CommandPalette, type PaletteItem } from "./command-palette";
import { brand as productBrand, type BrandSource } from "../../lib/brand";
import { useConfigStore } from "../../stores/config-store";
import { OfflineBanner } from "./offline-banner";

interface NavItem {
  path: string;
  label: string;
  icon: string;
  /** If set, this nav item is only shown when the named capability is true. */
  requiresCap?: keyof BackendCapabilities;
}

const navItems: NavItem[] = [
  { path: "/", label: "Dashboard", icon: "D" },
  { path: "/agents", label: "Agents", icon: "A" },
  { path: "/canvas", label: "Canvas", icon: "V" },
  { path: "/chat", label: "Chat", icon: "C" },
  { path: "/sessions", label: "Sessions", icon: "S" },
  { path: "/tools", label: "Tools", icon: "T" },
  { path: "/skills", label: "Skills", icon: "K" },
  { path: "/memory", label: "Memory", icon: "M" },
  { path: "/config", label: "Config", icon: "G" },
  { path: "/cron", label: "Cron", icon: "R", requiresCap: "cron" },
  { path: "/channels", label: "Channels", icon: "H", requiresCap: "channels" },
  { path: "/delegation", label: "Delegation", icon: "E", requiresCap: "delegation" },
  { path: "/monitoring", label: "Monitoring", icon: "O", requiresCap: "monitoring" },
  { path: "/voice", label: "Voice", icon: "W" },
];

export function MainLayout() {
  const [collapsed, setCollapsed] = useState(false);
  const [cmdKOpen, setCmdKOpen] = useState(false);
  const { theme, toggleTheme } = useThemeStore();
  const { wsConnected, setWsConnected } = useAgentStore();
  const skills = useSkillsStore((s) => s.skills);
  const fetchSkills = useSkillsStore((s) => s.fetchSkills);
  const channels = useChannelsStore((s) => s.channels);
  const fetchChannels = useChannelsStore((s) => s.fetchChannels);
  const { adapter, capabilities, mode } = useBackend();
  const navigate = useNavigate();
  // WEFT-176: product brand from config (defaults WeftOS).
  const config = useConfigStore((s) => s.config);
  const fetchConfig = useConfigStore((s) => s.fetchConfig);
  const displayBrand = productBrand(config as BrandSource | null);

  // Live entity lists for Cmd+K index (WEFT-568).
  const [agents, setAgents] = useState<
    Array<{ id: string; name: string }>
  >([]);
  const [sessions, setSessions] = useState<
    Array<{ key: string }>
  >([]);
  const [tools, setTools] = useState<
    Array<{ name: string; description: string }>
  >([]);

  useEffect(() => {
    if (!config) {
      void fetchConfig();
    }
  }, [config, fetchConfig]);

  // Warm palette entity lists when the layout mounts / backend is ready.
  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const [a, s, t] = await Promise.all([
          adapter.listAgents().catch(() => []),
          adapter.listSessions().catch(() => []),
          adapter.listTools().catch(() => []),
        ]);
        if (cancelled) return;
        setAgents(a.map((x) => ({ id: x.id, name: x.name })));
        setSessions(s.map((x) => ({ key: x.key })));
        setTools(t.map((x) => ({ name: x.name, description: x.description })));
      } catch {
        // Best-effort index; nav routes still work.
      }
    })();
    void fetchSkills();
    if (capabilities.channels) {
      void fetchChannels();
    }
    return () => {
      cancelled = true;
    };
  }, [adapter, capabilities.channels, fetchSkills, fetchChannels]);

  // Filter nav items based on backend capabilities
  const visibleNavItems = useMemo(
    () =>
      navItems.filter(
        (item) => !item.requiresCap || capabilities[item.requiresCap],
      ),
    [capabilities],
  );

  // WEFT-308 / WEFT-568: palette indexes routes + agents/sessions/tools/
  // skills/channels + utility actions.
  const paletteItems = useMemo<PaletteItem[]>(() => {
    const navActions: PaletteItem[] = visibleNavItems.map((item) => ({
      id: `goto:${item.path}`,
      label: `Go to ${item.label}`,
      hint: item.path,
      icon: item.icon,
      action: () => navigate({ to: item.path }),
    }));

    const agentActions: PaletteItem[] = agents.map((a) => ({
      id: `agent:${a.id}`,
      label: a.name || a.id,
      hint: "Agent",
      icon: "A",
      action: () => navigate({ to: "/agents" }),
    }));

    const sessionActions: PaletteItem[] = sessions.map((s) => ({
      id: `session:${s.key}`,
      label: s.key,
      hint: "Session",
      icon: "S",
      action: () => navigate({ to: "/sessions" }),
    }));

    const toolActions: PaletteItem[] = tools.map((t) => ({
      id: `tool:${t.name}`,
      label: t.name,
      hint: t.description ? t.description.slice(0, 48) : "Tool",
      icon: "T",
      action: () => navigate({ to: "/tools" }),
    }));

    const skillActions: PaletteItem[] = skills.map((sk) => ({
      id: `skill:${sk.name}`,
      label: sk.name,
      hint: "Skill",
      icon: "K",
      action: () => navigate({ to: "/skills" }),
    }));

    const channelActions: PaletteItem[] = channels.map((ch) => ({
      id: `channel:${ch.name}`,
      label: ch.name,
      hint: "Channel",
      icon: "H",
      action: () => navigate({ to: "/channels" }),
    }));

    const utilActions: PaletteItem[] = [
      {
        id: "action:toggle-theme",
        label: theme === "dark" ? "Switch to light mode" : "Switch to dark mode",
        hint: "Theme",
        icon: theme === "dark" ? "L" : "D",
        action: toggleTheme,
      },
      {
        id: "action:toggle-sidebar",
        label: collapsed ? "Expand sidebar" : "Collapse sidebar",
        hint: "Layout",
        icon: collapsed ? ">" : "<",
        action: () => setCollapsed((c) => !c),
      },
    ];

    return [
      ...navActions,
      ...agentActions,
      ...sessionActions,
      ...toolActions,
      ...skillActions,
      ...channelActions,
      ...utilActions,
    ];
  }, [
    visibleNavItems,
    navigate,
    theme,
    toggleTheme,
    collapsed,
    agents,
    sessions,
    tools,
    skills,
    channels,
  ]);

  // Apply theme class to html element
  useEffect(() => {
    const root = document.documentElement;
    if (theme === "dark") {
      root.classList.add("dark");
    } else {
      root.classList.remove("dark");
    }
  }, [theme]);

  // Connect WebSocket
  useEffect(() => {
    wsClient.connect();

    const offConnect = wsClient.on("connected", () => setWsConnected(true));
    const offDisconnect = wsClient.on("disconnected", () =>
      setWsConnected(false),
    );

    return () => {
      offConnect();
      offDisconnect();
      wsClient.disconnect();
    };
  }, [setWsConnected]);

  // Cmd+K shortcut
  const handleKeyDown = useCallback((e: KeyboardEvent) => {
    if ((e.metaKey || e.ctrlKey) && e.key === "k") {
      e.preventDefault();
      setCmdKOpen((prev) => !prev);
    }
    if (e.key === "Escape") {
      setCmdKOpen(false);
    }
  }, []);

  useEffect(() => {
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [handleKeyDown]);

  return (
    <div className="flex h-screen bg-gray-50 dark:bg-gray-900">
      {/* Sidebar */}
      <aside
        className={cn(
          "flex flex-col border-r border-gray-200 bg-white transition-all dark:border-gray-700 dark:bg-gray-800",
          collapsed ? "w-16" : "w-64",
        )}
      >
        {/* Logo / Collapse toggle */}
        <div className="flex items-center justify-between border-b border-gray-200 p-4 dark:border-gray-700">
          {!collapsed && (
            <span className="text-lg font-bold text-gray-900 dark:text-gray-100">
              {displayBrand}
            </span>
          )}
          <button
            onClick={() => setCollapsed(!collapsed)}
            className="rounded-md p-1 text-gray-500 hover:bg-gray-100 dark:text-gray-400 dark:hover:bg-gray-700"
            aria-label={collapsed ? "Expand sidebar" : "Collapse sidebar"}
          >
            {collapsed ? ">>" : "<<"}
          </button>
        </div>

        {/* Navigation */}
        <nav className="flex-1 space-y-1 p-2">
          {visibleNavItems.map((item) => (
            <Link
              key={item.path}
              to={item.path}
              className="flex items-center gap-3 rounded-md px-3 py-2 text-sm text-gray-700 transition-colors hover:bg-gray-100 dark:text-gray-300 dark:hover:bg-gray-700 [&.active]:bg-blue-50 [&.active]:text-blue-700 dark:[&.active]:bg-blue-900/30 dark:[&.active]:text-blue-300"
            >
              <span className="inline-flex h-5 w-5 items-center justify-center text-xs font-bold">
                {item.icon}
              </span>
              {!collapsed && <span>{item.label}</span>}
            </Link>
          ))}
        </nav>

        {/* Bottom section */}
        <div className="border-t border-gray-200 p-3 dark:border-gray-700">
          {/* Mode indicator */}
          {!collapsed && mode === "wasm" && (
            <div className="mb-2">
              <Badge variant="outline" className="w-full justify-center text-xs border-amber-500 text-amber-600 dark:text-amber-400">
                Browser Mode
              </Badge>
            </div>
          )}

          {/* WS Status */}
          <div className="mb-2 flex items-center gap-2">
            <span
              className={cn(
                "h-2 w-2 rounded-full",
                mode === "wasm"
                  ? "bg-amber-500"
                  : wsConnected
                    ? "bg-green-500"
                    : "bg-red-500",
              )}
            />
            {!collapsed && (
              <span className="text-xs text-gray-500 dark:text-gray-400">
                {mode === "wasm"
                  ? "WASM"
                  : wsConnected
                    ? "Connected"
                    : "Disconnected"}
              </span>
            )}
          </div>

          {/* Voice status */}
          {!collapsed && (
            <div className="mb-2">
              <VoiceStatusBar />
            </div>
          )}

          {/* Theme toggle */}
          <button
            onClick={toggleTheme}
            className="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-xs text-gray-600 hover:bg-gray-100 dark:text-gray-400 dark:hover:bg-gray-700"
            aria-label="Toggle theme"
          >
            <span>{theme === "dark" ? "Light" : "Dark"}</span>
            {!collapsed && <span>Mode</span>}
          </button>

          {/* Cmd+K hint */}
          {!collapsed && (
            <div className="mt-2">
              <Badge variant="outline" className="w-full justify-center text-xs">
                Ctrl+K
              </Badge>
            </div>
          )}
        </div>
      </aside>

      {/* Main content */}
      <main className="flex flex-1 flex-col overflow-hidden">
        {/* WEFT-573: offline banner when SW shell is served without network */}
        <OfflineBanner />
        <div className="flex-1 overflow-auto">
          <Outlet />
        </div>
      </main>

      {/* Talk Mode overlay */}
      <TalkModeOverlay />

      {/* WEFT-308 / WEFT-568: fuzzy-search command palette with entity index */}
      <CommandPalette
        open={cmdKOpen}
        onClose={() => setCmdKOpen(false)}
        items={paletteItems}
      />
    </div>
  );
}
