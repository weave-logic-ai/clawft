import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { api } from "../lib/api-client";
import { formatRelativeTime } from "../lib/utils";
import { Card, CardHeader, CardContent } from "../components/ui/card";
import { Badge } from "../components/ui/badge";
import { Button } from "../components/ui/button";
import { Skeleton } from "../components/ui/skeleton";
import { Separator } from "../components/ui/separator";
import type { SessionSummary, SessionDetail, ChatMessage } from "../lib/types";

function MessagePreview({ message }: { message: ChatMessage }) {
  const roleColors: Record<string, string> = {
    user: "text-blue-600 dark:text-blue-400",
    assistant: "text-green-600 dark:text-green-400",
    system: "text-yellow-600 dark:text-yellow-400",
    tool: "text-purple-600 dark:text-purple-400",
  };

  return (
    <div className="flex gap-3 py-2">
      <span
        className={`w-16 shrink-0 text-xs font-semibold uppercase ${roleColors[message.role] ?? "text-gray-500"}`}
      >
        {message.role}
      </span>
      <p className="flex-1 truncate text-sm text-gray-700 dark:text-gray-300">
        {message.content}
      </p>
      <span className="shrink-0 text-xs text-gray-400">
        {formatRelativeTime(message.timestamp)}
      </span>
    </div>
  );
}

function SessionRow({
  session,
  isExpanded,
  onToggle,
}: {
  session: SessionSummary;
  isExpanded: boolean;
  onToggle: () => void;
}) {
  const queryClient = useQueryClient();

  const detail = useQuery<SessionDetail>({
    queryKey: ["session", session.key],
    queryFn: () => api.sessions.get(session.key),
    enabled: isExpanded,
  });

  const exportMutation = useMutation({
    mutationFn: () => api.sessions.export(session.key),
    onSuccess: (data) => {
      const blob = new Blob([JSON.stringify(data, null, 2)], {
        type: "application/json",
      });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `session-${session.key}.json`;
      a.click();
      URL.revokeObjectURL(url);
    },
  });

  const deleteMutation = useMutation({
    mutationFn: () => api.sessions.delete(session.key),
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: ["sessions"] }),
  });

  return (
    <div className="border-b border-gray-200 dark:border-gray-700 last:border-b-0">
      <button
        onClick={onToggle}
        className="flex w-full items-center gap-4 px-4 py-3 text-left hover:bg-gray-50 dark:hover:bg-gray-800/50 transition-colors"
      >
        <span className="text-xs text-gray-400">{isExpanded ? "v" : ">"}</span>
        <span className="flex-1 truncate text-sm font-medium text-gray-900 dark:text-gray-100">
          {session.key}
        </span>
        <span className="text-sm text-gray-500 dark:text-gray-400">
          {session.agent_id}
        </span>
        <Badge variant="secondary">{session.message_count} msgs</Badge>
        <span className="text-xs text-gray-400">
          {formatRelativeTime(session.updated_at)}
        </span>
      </button>

      {isExpanded && (
        <div className="bg-gray-50 px-4 pb-4 dark:bg-gray-800/30">
          <div className="mb-3 flex gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={() => exportMutation.mutate()}
              disabled={exportMutation.isPending}
            >
              {exportMutation.isPending ? "Exporting..." : "Export JSON"}
            </Button>
            <Button
              variant="destructive"
              size="sm"
              onClick={() => {
                if (window.confirm("Delete this session?")) {
                  deleteMutation.mutate();
                }
              }}
              disabled={deleteMutation.isPending}
            >
              {deleteMutation.isPending ? "Deleting..." : "Delete"}
            </Button>
          </div>

          <Separator className="my-2" />

          {detail.isLoading ? (
            <div className="space-y-2">
              {Array.from({ length: 3 }).map((_, i) => (
                <Skeleton key={i} className="h-8 w-full" />
              ))}
            </div>
          ) : detail.data?.messages.length === 0 ? (
            <p className="py-2 text-xs text-gray-400">No messages</p>
          ) : (
            <div className="max-h-64 overflow-y-auto divide-y divide-gray-200 dark:divide-gray-700">
              {detail.data?.messages.map((msg, i) => (
                <MessagePreview key={i} message={msg} />
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

export function SessionsPage() {
  const [expandedKey, setExpandedKey] = useState<string | null>(null);

  const { data: sessions, isLoading, isError, error } = useQuery<SessionSummary[]>({
    queryKey: ["sessions"],
    queryFn: api.sessions.list,
    refetchInterval: 15000,
  });

  const sorted = (sessions ?? []).sort(
    (a, b) =>
      new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime(),
  );

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900 dark:text-gray-100">
            Sessions
          </h1>
          <p className="text-sm text-gray-500 dark:text-gray-400">
            Browse and manage conversation sessions
          </p>
        </div>
        {sessions && (
          <Badge variant="secondary">{sessions.length} sessions</Badge>
        )}
      </div>

      {isError && (
        <div className="rounded-md border border-red-300 bg-red-50 p-4 dark:border-red-700 dark:bg-red-900/20">
          <p className="text-sm text-red-600 dark:text-red-400">
            Failed to load sessions: {error?.message ?? "Unknown error"}
          </p>
        </div>
      )}

      <Card>
        <CardHeader className="pb-0">
          <div className="flex items-center text-xs font-medium uppercase text-gray-500 dark:text-gray-400 gap-4 px-4">
            <span className="w-4" />
            <span className="flex-1">Session Key</span>
            <span>Agent</span>
            <span className="w-20 text-center">Messages</span>
            <span className="w-20 text-right">Updated</span>
          </div>
        </CardHeader>
        <CardContent className="p-0">
          {isLoading ? (
            <div className="p-4 space-y-3">
              {Array.from({ length: 5 }).map((_, i) => (
                <Skeleton key={i} className="h-12 w-full" />
              ))}
            </div>
          ) : sorted.length === 0 ? (
            <div className="py-12 text-center">
              <p className="text-gray-500 dark:text-gray-400">
                No sessions found
              </p>
            </div>
          ) : (
            sorted.map((session) => (
              <SessionRow
                key={session.key}
                session={session}
                isExpanded={expandedKey === session.key}
                onToggle={() =>
                  setExpandedKey(
                    expandedKey === session.key ? null : session.key,
                  )
                }
              />
            ))
          )}
        </CardContent>
      </Card>
    </div>
  );
}
