import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { api } from "../lib/api-client";
import { formatRelativeTime } from "../lib/utils";
import { Card, CardHeader, CardTitle, CardContent, CardFooter } from "../components/ui/card";
import { Badge } from "../components/ui/badge";
import { Button } from "../components/ui/button";
import { Skeleton } from "../components/ui/skeleton";
import type { AgentSummary, AgentStatus } from "../lib/types";

function statusBadgeVariant(
  status: AgentStatus,
): "success" | "secondary" | "destructive" {
  switch (status) {
    case "running":
      return "success";
    case "stopped":
      return "secondary";
    case "error":
      return "destructive";
  }
}

function AgentCard({ agent }: { agent: AgentSummary }) {
  const queryClient = useQueryClient();

  const startMutation = useMutation({
    mutationFn: () => api.agents.start(agent.id),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["agents"] }),
  });

  const stopMutation = useMutation({
    mutationFn: () => api.agents.stop(agent.id),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["agents"] }),
  });

  const isPending = startMutation.isPending || stopMutation.isPending;

  return (
    <Card>
      <CardHeader>
        <div className="flex items-start justify-between">
          <div>
            <CardTitle>{agent.name}</CardTitle>
            <p className="mt-1 text-xs text-gray-500 dark:text-gray-400">
              {agent.id}
            </p>
          </div>
          <Badge variant={statusBadgeVariant(agent.status)}>
            {agent.status}
          </Badge>
        </div>
      </CardHeader>
      <CardContent>
        <div className="space-y-2 text-sm text-gray-600 dark:text-gray-400">
          {agent.model && (
            <div className="flex justify-between">
              <span>Model</span>
              <span className="font-medium text-gray-900 dark:text-gray-200">
                {agent.model}
              </span>
            </div>
          )}
          <div className="flex justify-between">
            <span>Created</span>
            <span className="font-medium text-gray-900 dark:text-gray-200">
              {formatRelativeTime(agent.created_at)}
            </span>
          </div>
        </div>
      </CardContent>
      <CardFooter className="gap-2">
        {agent.status === "running" ? (
          <Button
            variant="outline"
            size="sm"
            onClick={() => stopMutation.mutate()}
            disabled={isPending}
          >
            {stopMutation.isPending ? "Stopping..." : "Stop"}
          </Button>
        ) : (
          <Button
            variant="default"
            size="sm"
            onClick={() => startMutation.mutate()}
            disabled={isPending}
          >
            {startMutation.isPending ? "Starting..." : "Start"}
          </Button>
        )}
        {(startMutation.isError || stopMutation.isError) && (
          <span className="text-xs text-red-500">Action failed</span>
        )}
      </CardFooter>
    </Card>
  );
}

function AgentSkeletonCard() {
  return (
    <Card>
      <CardHeader>
        <Skeleton className="h-5 w-32" />
        <Skeleton className="mt-1 h-3 w-48" />
      </CardHeader>
      <CardContent>
        <div className="space-y-2">
          <Skeleton className="h-4 w-full" />
          <Skeleton className="h-4 w-full" />
        </div>
      </CardContent>
      <CardFooter>
        <Skeleton className="h-8 w-16" />
      </CardFooter>
    </Card>
  );
}

export function AgentsPage() {
  const { data: agents, isLoading, isError, error } = useQuery<AgentSummary[]>({
    queryKey: ["agents"],
    queryFn: api.agents.list,
    refetchInterval: 10000,
  });

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900 dark:text-gray-100">
            Agents
          </h1>
          <p className="text-sm text-gray-500 dark:text-gray-400">
            Manage and monitor your agents
          </p>
        </div>
        {agents && (
          <Badge variant="secondary">{agents.length} total</Badge>
        )}
      </div>

      {isError && (
        <div className="rounded-md border border-red-300 bg-red-50 p-4 dark:border-red-700 dark:bg-red-900/20">
          <p className="text-sm text-red-600 dark:text-red-400">
            Failed to load agents: {error?.message ?? "Unknown error"}
          </p>
        </div>
      )}

      <div className="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
        {isLoading
          ? Array.from({ length: 6 }).map((_, i) => (
              <AgentSkeletonCard key={i} />
            ))
          : agents?.map((agent) => (
              <AgentCard key={agent.id} agent={agent} />
            ))}
      </div>

      {!isLoading && agents?.length === 0 && (
        <div className="py-12 text-center">
          <p className="text-gray-500 dark:text-gray-400">
            No agents configured yet.
          </p>
        </div>
      )}
    </div>
  );
}
