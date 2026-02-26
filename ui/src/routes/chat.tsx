import { useState, useRef, useEffect, useCallback } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { api } from "../lib/api-client";
import { wsClient } from "../lib/ws-client";
import { cn, formatRelativeTime } from "../lib/utils";
import { Button } from "../components/ui/button";
import { Badge } from "../components/ui/badge";
import { Skeleton } from "../components/ui/skeleton";
import { Separator } from "../components/ui/separator";
import type { SessionSummary, SessionDetail, ChatMessage } from "../lib/types";

function MessageBubble({ message }: { message: ChatMessage }) {
  const isUser = message.role === "user";
  const isSystem = message.role === "system";
  const isTool = message.role === "tool";

  return (
    <div
      className={cn("flex w-full", isUser ? "justify-end" : "justify-start")}
    >
      <div
        className={cn(
          "max-w-[80%] rounded-lg px-4 py-2.5 text-sm",
          isUser &&
            "bg-blue-600 text-white",
          !isUser &&
            !isSystem &&
            !isTool &&
            "bg-gray-100 text-gray-900 dark:bg-gray-700 dark:text-gray-100",
          isSystem &&
            "bg-yellow-50 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-300 border border-yellow-200 dark:border-yellow-800",
          isTool &&
            "bg-purple-50 text-purple-800 dark:bg-purple-900/30 dark:text-purple-300 border border-purple-200 dark:border-purple-800 font-mono text-xs",
        )}
      >
        {(isSystem || isTool) && (
          <span className="mb-1 block text-xs font-semibold uppercase opacity-70">
            {message.role}
          </span>
        )}
        <p className="whitespace-pre-wrap">{message.content}</p>
        {message.tool_calls && message.tool_calls.length > 0 && (
          <div className="mt-2 space-y-1 border-t border-white/20 pt-2">
            {message.tool_calls.map((tc) => (
              <div key={tc.id} className="text-xs opacity-80">
                <span className="font-semibold">{tc.name}</span>
                {tc.result && (
                  <span className="ml-1 opacity-60">
                    {" "}
                    - {tc.result.slice(0, 100)}
                  </span>
                )}
              </div>
            ))}
          </div>
        )}
        <span className="mt-1 block text-xs opacity-50">
          {formatRelativeTime(message.timestamp)}
        </span>
      </div>
    </div>
  );
}

function SessionSidebar({
  sessions,
  activeKey,
  onSelect,
  isLoading,
}: {
  sessions: SessionSummary[];
  activeKey: string | null;
  onSelect: (key: string) => void;
  isLoading: boolean;
}) {
  return (
    <div className="flex h-full w-64 shrink-0 flex-col border-r border-gray-200 bg-white dark:border-gray-700 dark:bg-gray-800">
      <div className="p-3">
        <h2 className="text-sm font-semibold text-gray-700 dark:text-gray-300">
          Sessions
        </h2>
      </div>
      <Separator />
      <div className="flex-1 overflow-y-auto p-2">
        {isLoading ? (
          <div className="space-y-2">
            {Array.from({ length: 4 }).map((_, i) => (
              <Skeleton key={i} className="h-12 w-full" />
            ))}
          </div>
        ) : sessions.length === 0 ? (
          <p className="px-2 py-4 text-xs text-gray-400">No sessions</p>
        ) : (
          sessions.map((s) => (
            <button
              key={s.key}
              onClick={() => onSelect(s.key)}
              className={cn(
                "w-full rounded-md px-3 py-2 text-left text-sm transition-colors",
                activeKey === s.key
                  ? "bg-blue-50 text-blue-700 dark:bg-blue-900/30 dark:text-blue-300"
                  : "text-gray-700 hover:bg-gray-100 dark:text-gray-300 dark:hover:bg-gray-700",
              )}
            >
              <p className="truncate font-medium">{s.key}</p>
              <p className="text-xs opacity-60">
                {s.message_count} msgs &middot;{" "}
                {formatRelativeTime(s.updated_at)}
              </p>
            </button>
          ))
        )}
      </div>
    </div>
  );
}

function StreamingIndicator() {
  return (
    <div className="flex justify-start">
      <div className="rounded-lg bg-gray-100 px-4 py-2.5 dark:bg-gray-700">
        <div className="flex items-center gap-1">
          <div className="h-2 w-2 animate-bounce rounded-full bg-gray-400 [animation-delay:0ms]" />
          <div className="h-2 w-2 animate-bounce rounded-full bg-gray-400 [animation-delay:150ms]" />
          <div className="h-2 w-2 animate-bounce rounded-full bg-gray-400 [animation-delay:300ms]" />
        </div>
      </div>
    </div>
  );
}

export function ChatPage() {
  const queryClient = useQueryClient();
  const [activeSession, setActiveSession] = useState<string | null>(null);
  const [input, setInput] = useState("");
  const [streamingContent, setStreamingContent] = useState<string | null>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  const sessions = useQuery<SessionSummary[]>({
    queryKey: ["sessions"],
    queryFn: api.sessions.list,
  });

  const sessionDetail = useQuery<SessionDetail>({
    queryKey: ["session", activeSession],
    queryFn: () => api.sessions.get(activeSession!),
    enabled: !!activeSession,
  });

  const sendMessage = useMutation({
    mutationFn: (content: string) => api.chat.send(activeSession!, content),
    onMutate: async (content) => {
      const optimisticMsg: ChatMessage = {
        role: "user",
        content,
        timestamp: new Date().toISOString(),
      };
      queryClient.setQueryData<SessionDetail>(
        ["session", activeSession],
        (old) => {
          if (!old) return old;
          return { ...old, messages: [...old.messages, optimisticMsg] };
        },
      );
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["session", activeSession] });
      queryClient.invalidateQueries({ queryKey: ["sessions"] });
    },
  });

  const scrollToBottom = useCallback(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, []);

  useEffect(() => {
    scrollToBottom();
  }, [sessionDetail.data?.messages, streamingContent, scrollToBottom]);

  // WebSocket streaming listener
  useEffect(() => {
    if (!activeSession) return;

    wsClient.subscribe(`session:${activeSession}`);

    const offChunk = wsClient.on("stream_chunk", (data) => {
      const msg = data as { content?: string };
      if (msg.content) {
        setStreamingContent((prev) => (prev ?? "") + msg.content);
      }
    });

    const offDone = wsClient.on("stream_done", () => {
      setStreamingContent(null);
      queryClient.invalidateQueries({ queryKey: ["session", activeSession] });
    });

    return () => {
      wsClient.unsubscribe(`session:${activeSession}`);
      offChunk();
      offDone();
    };
  }, [activeSession, queryClient]);

  const handleSend = () => {
    const trimmed = input.trim();
    if (!trimmed || !activeSession || sendMessage.isPending) return;
    setInput("");
    sendMessage.mutate(trimmed);
  };

  const messages = sessionDetail.data?.messages ?? [];

  return (
    <div className="flex h-full">
      <SessionSidebar
        sessions={sessions.data ?? []}
        activeKey={activeSession}
        onSelect={setActiveSession}
        isLoading={sessions.isLoading}
      />

      <div className="flex flex-1 flex-col">
        {!activeSession ? (
          <div className="flex flex-1 items-center justify-center">
            <div className="text-center">
              <p className="text-lg text-gray-500 dark:text-gray-400">
                Select a session to start chatting
              </p>
              <p className="mt-1 text-sm text-gray-400 dark:text-gray-500">
                Or create a new session from the Agents page
              </p>
            </div>
          </div>
        ) : (
          <>
            {/* Header */}
            <div className="flex items-center justify-between border-b border-gray-200 px-4 py-3 dark:border-gray-700">
              <div>
                <h2 className="text-sm font-semibold text-gray-900 dark:text-gray-100">
                  {activeSession}
                </h2>
                <p className="text-xs text-gray-500 dark:text-gray-400">
                  {messages.length} messages
                </p>
              </div>
              <Badge variant="secondary">
                {sessionDetail.data?.agent_id ?? "..."}
              </Badge>
            </div>

            {/* Messages */}
            <div className="flex-1 overflow-y-auto p-4 space-y-3">
              {sessionDetail.isLoading ? (
                <div className="space-y-3">
                  {Array.from({ length: 3 }).map((_, i) => (
                    <Skeleton key={i} className="h-16 w-3/4" />
                  ))}
                </div>
              ) : messages.length === 0 ? (
                <p className="py-8 text-center text-sm text-gray-400">
                  No messages yet. Send one below.
                </p>
              ) : (
                messages.map((msg, i) => (
                  <MessageBubble key={i} message={msg} />
                ))
              )}
              {streamingContent !== null && (
                <>
                  <div className="flex justify-start">
                    <div className="max-w-[80%] rounded-lg bg-gray-100 px-4 py-2.5 text-sm text-gray-900 dark:bg-gray-700 dark:text-gray-100">
                      <p className="whitespace-pre-wrap">{streamingContent}</p>
                    </div>
                  </div>
                </>
              )}
              {sendMessage.isPending && streamingContent === null && (
                <StreamingIndicator />
              )}
              <div ref={messagesEndRef} />
            </div>

            {/* Input */}
            <div className="border-t border-gray-200 p-4 dark:border-gray-700">
              <div className="flex gap-2">
                <input
                  type="text"
                  value={input}
                  onChange={(e) => setInput(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" && !e.shiftKey) {
                      e.preventDefault();
                      handleSend();
                    }
                  }}
                  placeholder="Type a message..."
                  className="flex-1 rounded-md border border-gray-300 bg-white px-3 py-2 text-sm text-gray-900 placeholder-gray-400 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500 dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100 dark:placeholder-gray-500"
                  disabled={sendMessage.isPending}
                />
                <Button
                  onClick={handleSend}
                  disabled={!input.trim() || sendMessage.isPending}
                >
                  Send
                </Button>
              </div>
              {sendMessage.isError && (
                <p className="mt-1 text-xs text-red-500">
                  Failed to send message. Try again.
                </p>
              )}
            </div>
          </>
        )}
      </div>
    </div>
  );
}
