import { useEffect } from "react";
import { useChannelsStore } from "../stores/channels-store";
import { wsClient } from "../lib/ws-client";
import { Card, CardHeader, CardTitle, CardContent } from "../components/ui/card";
import { Badge } from "../components/ui/badge";
import { Skeleton } from "../components/ui/skeleton";
import { formatRelativeTime } from "../lib/utils";
import type { ChannelStatus } from "../lib/types";

function channelStatusVariant(
  status: ChannelStatus["status"],
): "success" | "secondary" | "destructive" {
  switch (status) {
    case "connected":
      return "success";
    case "disconnected":
      return "secondary";
    case "error":
      return "destructive";
  }
}

function channelTypeLabel(type: ChannelStatus["type"]): string {
  const labels: Record<ChannelStatus["type"], string> = {
    discord: "Discord",
    telegram: "Telegram",
    slack: "Slack",
    web: "Web",
    voice: "Voice",
  };
  return labels[type];
}

function ChannelCard({ channel }: { channel: ChannelStatus }) {
  return (
    <Card>
      <CardHeader>
        <div className="flex items-start justify-between">
          <div>
            <CardTitle>{channel.name}</CardTitle>
            <p className="mt-1 text-xs text-gray-500 dark:text-gray-400">
              {channelTypeLabel(channel.type)}
            </p>
          </div>
          <Badge variant={channelStatusVariant(channel.status)}>
            {channel.status}
          </Badge>
        </div>
      </CardHeader>
      <CardContent>
        <div className="space-y-2 text-sm text-gray-600 dark:text-gray-400">
          <div className="flex justify-between">
            <span>Messages</span>
            <span className="font-medium text-gray-900 dark:text-gray-200">
              {channel.message_count.toLocaleString()}
            </span>
          </div>
          {channel.last_activity && (
            <div className="flex justify-between">
              <span>Last Activity</span>
              <span className="font-medium text-gray-900 dark:text-gray-200">
                {formatRelativeTime(channel.last_activity)}
              </span>
            </div>
          )}
          {channel.routes_to && (
            <div className="flex justify-between">
              <span>Routes To</span>
              <Badge variant="outline" className="text-xs">
                {channel.routes_to}
              </Badge>
            </div>
          )}
        </div>

        {/* Routing visualization */}
        {channel.routes_to && (
          <div className="mt-4 flex items-center gap-2 rounded-md bg-gray-50 p-2 dark:bg-gray-700/50">
            <div className="flex items-center gap-1.5">
              <span
                className="inline-block h-2 w-2 rounded-full"
                style={{
                  backgroundColor:
                    channel.status === "connected"
                      ? "#22c55e"
                      : channel.status === "error"
                        ? "#ef4444"
                        : "#6b7280",
                }}
              />
              <span className="text-xs font-medium text-gray-700 dark:text-gray-300">
                {channel.name}
              </span>
            </div>
            <span className="text-xs text-gray-400">&#8594;</span>
            <span className="text-xs font-medium text-blue-600 dark:text-blue-400">
              {channel.routes_to}
            </span>
          </div>
        )}
      </CardContent>
    </Card>
  );
}

function ChannelSkeletonCard() {
  return (
    <Card>
      <CardHeader>
        <Skeleton className="h-5 w-32" />
        <Skeleton className="mt-1 h-3 w-16" />
      </CardHeader>
      <CardContent>
        <div className="space-y-2">
          <Skeleton className="h-4 w-full" />
          <Skeleton className="h-4 w-full" />
          <Skeleton className="h-4 w-3/4" />
        </div>
      </CardContent>
    </Card>
  );
}

export function ChannelsPage() {
  const { channels, loading, fetchChannels, updateChannel } =
    useChannelsStore();

  useEffect(() => {
    fetchChannels();
  }, [fetchChannels]);

  // Subscribe to real-time channel status updates via WebSocket
  useEffect(() => {
    wsClient.subscribe("channels");

    const off = wsClient.on("channel_status", (data) => {
      const payload = data as {
        name: string;
        status?: ChannelStatus["status"];
        message_count?: number;
        last_activity?: string;
      };
      if (payload.name) {
        updateChannel(payload.name, {
          ...(payload.status ? { status: payload.status } : {}),
          ...(payload.message_count !== undefined
            ? { message_count: payload.message_count }
            : {}),
          ...(payload.last_activity
            ? { last_activity: payload.last_activity }
            : {}),
        });
      }
    });

    return () => {
      wsClient.unsubscribe("channels");
      off();
    };
  }, [updateChannel]);

  const connectedCount = channels.filter(
    (c) => c.status === "connected",
  ).length;

  return (
    <div className="flex-1 overflow-auto p-6 space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900 dark:text-gray-100">
            Channels
          </h1>
          <p className="text-sm text-gray-500 dark:text-gray-400">
            Monitor channel connections and routing
          </p>
        </div>
        {channels.length > 0 && (
          <div className="flex items-center gap-2">
            <Badge variant="success">{connectedCount} connected</Badge>
            <Badge variant="secondary">{channels.length} total</Badge>
          </div>
        )}
      </div>

      <div className="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
        {loading
          ? Array.from({ length: 5 }).map((_, i) => (
              <ChannelSkeletonCard key={i} />
            ))
          : channels.map((channel) => (
              <ChannelCard key={channel.name} channel={channel} />
            ))}
      </div>

      {!loading && channels.length === 0 && (
        <div className="py-12 text-center">
          <p className="text-gray-500 dark:text-gray-400">
            No channels configured yet.
          </p>
        </div>
      )}
    </div>
  );
}
