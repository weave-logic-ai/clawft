import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { api } from "../lib/api-client";
import { Card, CardHeader, CardTitle, CardContent } from "../components/ui/card";
import { Badge } from "../components/ui/badge";
import { Button } from "../components/ui/button";
import { Skeleton } from "../components/ui/skeleton";
import type { ToolInfo } from "../lib/types";

function ToolCard({ tool }: { tool: ToolInfo }) {
  const [showSchema, setShowSchema] = useState(false);

  return (
    <Card>
      <CardHeader>
        <div className="flex items-start justify-between">
          <CardTitle className="text-base">{tool.name}</CardTitle>
          {tool.schema && (
            <Badge variant="outline" className="text-xs">
              schema
            </Badge>
          )}
        </div>
      </CardHeader>
      <CardContent>
        <p className="text-sm text-gray-600 dark:text-gray-400">
          {tool.description || "No description available"}
        </p>

        {tool.schema && (
          <div className="mt-3">
            <Button
              variant="ghost"
              size="sm"
              onClick={() => setShowSchema(!showSchema)}
            >
              {showSchema ? "Hide Schema" : "View Schema"}
            </Button>

            {showSchema && (
              <pre className="mt-2 max-h-64 overflow-auto rounded-md bg-gray-100 p-3 text-xs text-gray-800 dark:bg-gray-900 dark:text-gray-300">
                {JSON.stringify(tool.schema, null, 2)}
              </pre>
            )}
          </div>
        )}
      </CardContent>
    </Card>
  );
}

function ToolSkeletonCard() {
  return (
    <Card>
      <CardHeader>
        <Skeleton className="h-5 w-40" />
      </CardHeader>
      <CardContent>
        <Skeleton className="h-4 w-full" />
        <Skeleton className="mt-2 h-4 w-2/3" />
      </CardContent>
    </Card>
  );
}

export function ToolsPage() {
  const [search, setSearch] = useState("");
  const [viewMode, setViewMode] = useState<"grid" | "list">("grid");

  const { data: tools, isLoading, isError, error } = useQuery<ToolInfo[]>({
    queryKey: ["tools"],
    queryFn: api.tools.list,
  });

  const filtered = (tools ?? []).filter(
    (t) =>
      t.name.toLowerCase().includes(search.toLowerCase()) ||
      t.description?.toLowerCase().includes(search.toLowerCase()),
  );

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900 dark:text-gray-100">
            Tools
          </h1>
          <p className="text-sm text-gray-500 dark:text-gray-400">
            Available tools and their schemas
          </p>
        </div>
        {tools && (
          <Badge variant="secondary">{tools.length} tools</Badge>
        )}
      </div>

      <div className="flex gap-3">
        <input
          type="text"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder="Search tools..."
          className="flex-1 rounded-md border border-gray-300 bg-white px-3 py-2 text-sm text-gray-900 placeholder-gray-400 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500 dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100 dark:placeholder-gray-500"
        />
        <Button
          variant={viewMode === "grid" ? "secondary" : "ghost"}
          size="sm"
          onClick={() => setViewMode("grid")}
        >
          Grid
        </Button>
        <Button
          variant={viewMode === "list" ? "secondary" : "ghost"}
          size="sm"
          onClick={() => setViewMode("list")}
        >
          List
        </Button>
      </div>

      {isError && (
        <div className="rounded-md border border-red-300 bg-red-50 p-4 dark:border-red-700 dark:bg-red-900/20">
          <p className="text-sm text-red-600 dark:text-red-400">
            Failed to load tools: {error?.message ?? "Unknown error"}
          </p>
        </div>
      )}

      {viewMode === "grid" ? (
        <div className="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
          {isLoading
            ? Array.from({ length: 6 }).map((_, i) => (
                <ToolSkeletonCard key={i} />
              ))
            : filtered.map((tool) => (
                <ToolCard key={tool.name} tool={tool} />
              ))}
        </div>
      ) : (
        <Card>
          <CardContent className="p-0">
            {isLoading ? (
              <div className="p-4 space-y-3">
                {Array.from({ length: 6 }).map((_, i) => (
                  <Skeleton key={i} className="h-10 w-full" />
                ))}
              </div>
            ) : (
              <div className="divide-y divide-gray-200 dark:divide-gray-700">
                {filtered.map((tool) => (
                  <ToolListRow key={tool.name} tool={tool} />
                ))}
              </div>
            )}
          </CardContent>
        </Card>
      )}

      {!isLoading && filtered.length === 0 && (
        <div className="py-12 text-center">
          <p className="text-gray-500 dark:text-gray-400">
            {search ? "No tools match your search" : "No tools available"}
          </p>
        </div>
      )}
    </div>
  );
}

function ToolListRow({ tool }: { tool: ToolInfo }) {
  const [showSchema, setShowSchema] = useState(false);

  return (
    <div className="px-4 py-3">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <span className="text-sm font-medium text-gray-900 dark:text-gray-100">
            {tool.name}
          </span>
          {tool.schema && (
            <Badge variant="outline" className="text-xs">
              schema
            </Badge>
          )}
        </div>
        <div className="flex items-center gap-2">
          <span className="max-w-sm truncate text-sm text-gray-500 dark:text-gray-400">
            {tool.description}
          </span>
          {tool.schema && (
            <Button
              variant="ghost"
              size="sm"
              onClick={() => setShowSchema(!showSchema)}
            >
              {showSchema ? "Hide" : "Schema"}
            </Button>
          )}
        </div>
      </div>
      {showSchema && tool.schema && (
        <pre className="mt-2 max-h-48 overflow-auto rounded-md bg-gray-100 p-3 text-xs text-gray-800 dark:bg-gray-900 dark:text-gray-300">
          {JSON.stringify(tool.schema, null, 2)}
        </pre>
      )}
    </div>
  );
}
