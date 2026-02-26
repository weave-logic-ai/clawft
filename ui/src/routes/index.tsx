import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { api } from "../lib/api-client";
import { formatRelativeTime } from "../lib/utils";
import { Card, CardHeader, CardTitle, CardContent } from "../components/ui/card";
import { Skeleton } from "../components/ui/skeleton";
import { Badge } from "../components/ui/badge";
import type { AgentSummary, SessionSummary, ToolInfo, SystemHealth } from "../lib/types";

function StatCard({
  title,
  value,
  subtitle,
  loading,
}: {
  title: string;
  value: string | number;
  subtitle?: string;
  loading?: boolean;
}) {
  return (
    <Card>
      <CardHeader>
        <p className="text-sm font-medium text-gray-500 dark:text-gray-400">
          {title}
        </p>
        {loading ? (
          <Skeleton className="h-8 w-20" />
        ) : (
          <CardTitle className="text-3xl">{value}</CardTitle>
        )}
      </CardHeader>
      {subtitle && (
        <CardContent>
          <p className="text-xs text-gray-500 dark:text-gray-400">
            {subtitle}
          </p>
        </CardContent>
      )}
    </Card>
  );
}

export function DashboardPage() {
  const agents = useQuery<AgentSummary[]>({
    queryKey: ["agents"],
    queryFn: api.agents.list,
  });

  const sessions = useQuery<SessionSummary[]>({
    queryKey: ["sessions"],
    queryFn: api.sessions.list,
  });

  const tools = useQuery<ToolInfo[]>({
    queryKey: ["tools"],
    queryFn: api.tools.list,
  });

  const health = useQuery<SystemHealth>({
    queryKey: ["health"],
    queryFn: api.system.health,
  });

  const isLoading =
    agents.isLoading || sessions.isLoading || tools.isLoading || health.isLoading;

  const runningAgents =
    agents.data?.filter((a) => a.status === "running").length ?? 0;

  const recentSessions = (sessions.data ?? [])
    .sort(
      (a, b) =>
        new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime(),
    )
    .slice(0, 5);

  const healthStatus = health.data?.status ?? "unknown";

  return (
    <div className="p-6 space-y-6">
      <div>
        <h1 className="text-2xl font-bold text-gray-900 dark:text-gray-100">
          Dashboard
        </h1>
        <p className="text-sm text-gray-500 dark:text-gray-400">
          ClawFT system overview
        </p>
      </div>

      <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-4">
        <StatCard
          title="Agents"
          value={agents.data?.length ?? 0}
          subtitle={`${runningAgents} running`}
          loading={isLoading}
        />
        <StatCard
          title="Sessions"
          value={sessions.data?.length ?? 0}
          loading={isLoading}
        />
        <StatCard
          title="Tools"
          value={tools.data?.length ?? 0}
          loading={isLoading}
        />
        <StatCard
          title="System Health"
          value={healthStatus}
          subtitle={
            health.data
              ? `Uptime: ${Math.floor(health.data.uptime_seconds / 3600)}h`
              : undefined
          }
          loading={isLoading}
        />
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Recent Sessions</CardTitle>
        </CardHeader>
        <CardContent>
          {sessions.isLoading ? (
            <div className="space-y-3">
              {Array.from({ length: 3 }).map((_, i) => (
                <Skeleton key={i} className="h-12 w-full" />
              ))}
            </div>
          ) : recentSessions.length === 0 ? (
            <p className="text-sm text-gray-500 dark:text-gray-400">
              No sessions yet
            </p>
          ) : (
            <div className="divide-y divide-gray-200 dark:divide-gray-700">
              {recentSessions.map((session) => (
                <Link
                  key={session.key}
                  to="/sessions"
                  className="flex items-center justify-between py-3 hover:bg-gray-50 dark:hover:bg-gray-750 px-2 rounded-md transition-colors"
                >
                  <div>
                    <p className="text-sm font-medium text-gray-900 dark:text-gray-100">
                      {session.key}
                    </p>
                    <p className="text-xs text-gray-500 dark:text-gray-400">
                      Agent: {session.agent_id}
                    </p>
                  </div>
                  <div className="flex items-center gap-3">
                    <Badge variant="secondary">
                      {session.message_count} messages
                    </Badge>
                    <span className="text-xs text-gray-400">
                      {formatRelativeTime(session.updated_at)}
                    </span>
                  </div>
                </Link>
              ))}
            </div>
          )}
        </CardContent>
      </Card>

      {(agents.isError || sessions.isError || tools.isError || health.isError) && (
        <Card className="border-red-300 dark:border-red-700">
          <CardContent className="pt-6">
            <p className="text-sm text-red-600 dark:text-red-400">
              Some data failed to load. The API server may be unavailable.
            </p>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
