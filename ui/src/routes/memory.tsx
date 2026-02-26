import { useState, useEffect, useCallback, useRef } from "react";
import { useMemoryStore } from "../stores/memory-store";
import { Badge } from "../components/ui/badge";
import { Button } from "../components/ui/button";
import { Skeleton } from "../components/ui/skeleton";
import { Dialog, DialogHeader, DialogTitle, DialogFooter } from "../components/ui/dialog";
import { truncate, formatRelativeTime } from "../lib/utils";

export function MemoryPage() {
  const {
    entries,
    namespaces,
    searchQuery,
    threshold,
    selectedNamespace,
    selectedTags,
    loading,
    setSearchQuery,
    setThreshold,
    setSelectedNamespace,
    setSelectedTags,
    fetchEntries,
    search,
    createEntry,
    deleteEntry,
  } = useMemoryStore();

  const [createOpen, setCreateOpen] = useState(false);
  const [newKey, setNewKey] = useState("");
  const [newValue, setNewValue] = useState("");
  const [newNamespace, setNewNamespace] = useState("default");
  const [newTags, setNewTags] = useState("");
  const debounceRef = useRef<ReturnType<typeof setTimeout>>(null);

  useEffect(() => {
    fetchEntries();
  }, [fetchEntries]);

  const handleSearch = useCallback(
    (query: string) => {
      setSearchQuery(query);
      if (debounceRef.current) clearTimeout(debounceRef.current);
      debounceRef.current = setTimeout(() => {
        if (query.trim()) {
          search(query, threshold);
        } else {
          fetchEntries();
        }
      }, 400);
    },
    [setSearchQuery, search, threshold, fetchEntries],
  );

  const handleThresholdChange = useCallback(
    (value: number) => {
      setThreshold(value);
      if (searchQuery.trim()) {
        search(searchQuery, value);
      }
    },
    [setThreshold, searchQuery, search],
  );

  const handleCreate = async () => {
    if (!newKey.trim()) return;
    await createEntry({
      key: newKey,
      value: newValue,
      namespace: newNamespace,
      tags: newTags
        .split(",")
        .map((t) => t.trim())
        .filter(Boolean),
    });
    setNewKey("");
    setNewValue("");
    setNewNamespace("default");
    setNewTags("");
    setCreateOpen(false);
  };

  const handleTagClick = (tag: string) => {
    if (selectedTags.includes(tag)) {
      setSelectedTags(selectedTags.filter((t) => t !== tag));
    } else {
      setSelectedTags([...selectedTags, tag]);
    }
  };

  // Collect all unique tags from entries
  const allTags = [...new Set(entries.flatMap((e) => e.tags))].sort();

  // Filter by namespace and tags
  const filteredEntries = entries.filter((e) => {
    if (selectedNamespace && e.namespace !== selectedNamespace) return false;
    if (
      selectedTags.length > 0 &&
      !selectedTags.some((t) => e.tags.includes(t))
    )
      return false;
    return true;
  });

  return (
    <div className="flex-1 overflow-auto p-6 space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900 dark:text-gray-100">
            Memory Explorer
          </h1>
          <p className="text-sm text-gray-500 dark:text-gray-400">
            Browse and search memory entries
          </p>
        </div>
        <div className="flex items-center gap-3">
          {entries.length > 0 && (
            <Badge variant="secondary">{entries.length} entries</Badge>
          )}
          <Button
            variant="default"
            size="sm"
            onClick={() => setCreateOpen(true)}
          >
            New Entry
          </Button>
        </div>
      </div>

      {/* Filters */}
      <div className="flex flex-wrap items-end gap-4">
        {/* Semantic search */}
        <div className="flex-1 min-w-[200px]">
          <label className="mb-1 block text-xs font-medium text-gray-600 dark:text-gray-400">
            Semantic Search
          </label>
          <input
            type="text"
            placeholder="Search memory..."
            value={searchQuery}
            onChange={(e) => handleSearch(e.target.value)}
            className="w-full rounded-md border border-gray-300 bg-white px-3 py-2 text-sm text-gray-900 placeholder-gray-400 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500 dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100 dark:placeholder-gray-500"
          />
        </div>

        {/* Threshold slider */}
        <div className="w-48">
          <label className="mb-1 block text-xs font-medium text-gray-600 dark:text-gray-400">
            Similarity: {threshold.toFixed(2)}
          </label>
          <input
            type="range"
            min="0"
            max="1"
            step="0.05"
            value={threshold}
            onChange={(e) => handleThresholdChange(parseFloat(e.target.value))}
            className="w-full"
          />
        </div>

        {/* Namespace filter */}
        <div className="w-40">
          <label className="mb-1 block text-xs font-medium text-gray-600 dark:text-gray-400">
            Namespace
          </label>
          <select
            value={selectedNamespace}
            onChange={(e) => setSelectedNamespace(e.target.value)}
            className="w-full rounded-md border border-gray-300 bg-white px-3 py-2 text-sm text-gray-900 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500 dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100"
          >
            <option value="">All</option>
            {namespaces.map((ns) => (
              <option key={ns} value={ns}>
                {ns}
              </option>
            ))}
          </select>
        </div>
      </div>

      {/* Tag filter chips */}
      {allTags.length > 0 && (
        <div className="flex flex-wrap gap-1.5">
          {allTags.map((tag) => (
            <button
              key={tag}
              onClick={() => handleTagClick(tag)}
              className="cursor-pointer"
            >
              <Badge
                variant={selectedTags.includes(tag) ? "default" : "outline"}
                className="text-xs"
              >
                {tag}
              </Badge>
            </button>
          ))}
          {selectedTags.length > 0 && (
            <button onClick={() => setSelectedTags([])}>
              <Badge variant="secondary" className="text-xs">
                clear
              </Badge>
            </button>
          )}
        </div>
      )}

      {/* Data table */}
      <div className="overflow-hidden rounded-lg border border-gray-200 dark:border-gray-700">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-gray-200 bg-gray-50 dark:border-gray-700 dark:bg-gray-800/50">
              <th className="px-4 py-3 text-left font-medium text-gray-600 dark:text-gray-400">
                Key
              </th>
              <th className="px-4 py-3 text-left font-medium text-gray-600 dark:text-gray-400">
                Namespace
              </th>
              <th className="px-4 py-3 text-left font-medium text-gray-600 dark:text-gray-400">
                Tags
              </th>
              <th className="px-4 py-3 text-left font-medium text-gray-600 dark:text-gray-400">
                Value
              </th>
              {searchQuery && (
                <th className="px-4 py-3 text-left font-medium text-gray-600 dark:text-gray-400">
                  Sim
                </th>
              )}
              <th className="px-4 py-3 text-left font-medium text-gray-600 dark:text-gray-400">
                Updated
              </th>
              <th className="px-4 py-3 text-right font-medium text-gray-600 dark:text-gray-400">
                Actions
              </th>
            </tr>
          </thead>
          <tbody>
            {loading ? (
              Array.from({ length: 5 }).map((_, i) => (
                <tr
                  key={i}
                  className="border-b border-gray-100 dark:border-gray-700"
                >
                  <td className="px-4 py-3">
                    <Skeleton className="h-4 w-28" />
                  </td>
                  <td className="px-4 py-3">
                    <Skeleton className="h-4 w-16" />
                  </td>
                  <td className="px-4 py-3">
                    <Skeleton className="h-4 w-24" />
                  </td>
                  <td className="px-4 py-3">
                    <Skeleton className="h-4 w-48" />
                  </td>
                  <td className="px-4 py-3">
                    <Skeleton className="h-4 w-12" />
                  </td>
                  <td className="px-4 py-3">
                    <Skeleton className="h-6 w-14" />
                  </td>
                </tr>
              ))
            ) : filteredEntries.length === 0 ? (
              <tr>
                <td
                  colSpan={searchQuery ? 7 : 6}
                  className="px-4 py-12 text-center text-gray-500 dark:text-gray-400"
                >
                  No entries found.
                </td>
              </tr>
            ) : (
              filteredEntries.map((entry) => (
                <tr
                  key={entry.key}
                  className="border-b border-gray-100 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-800/50"
                >
                  <td className="px-4 py-3 font-medium text-gray-900 dark:text-gray-100">
                    {entry.key}
                  </td>
                  <td className="px-4 py-3">
                    <Badge variant="outline" className="text-xs">
                      {entry.namespace}
                    </Badge>
                  </td>
                  <td className="px-4 py-3">
                    <div className="flex flex-wrap gap-1">
                      {entry.tags.map((tag) => (
                        <Badge
                          key={tag}
                          variant="secondary"
                          className="text-xs"
                        >
                          {tag}
                        </Badge>
                      ))}
                    </div>
                  </td>
                  <td className="px-4 py-3 text-gray-600 dark:text-gray-400">
                    {truncate(entry.value, 60)}
                  </td>
                  {searchQuery && (
                    <td className="px-4 py-3 text-gray-600 dark:text-gray-400">
                      {entry.similarity !== undefined
                        ? entry.similarity.toFixed(2)
                        : "-"}
                    </td>
                  )}
                  <td className="px-4 py-3 text-gray-500 dark:text-gray-400">
                    {formatRelativeTime(entry.updated_at)}
                  </td>
                  <td className="px-4 py-3 text-right">
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => deleteEntry(entry.key)}
                    >
                      Delete
                    </Button>
                  </td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>

      {/* Create Entry Dialog */}
      <Dialog open={createOpen} onClose={() => setCreateOpen(false)}>
        <DialogHeader>
          <DialogTitle>New Memory Entry</DialogTitle>
        </DialogHeader>

        <div className="space-y-4">
          <div>
            <label className="mb-1 block text-sm font-medium text-gray-700 dark:text-gray-300">
              Key
            </label>
            <input
              type="text"
              value={newKey}
              onChange={(e) => setNewKey(e.target.value)}
              placeholder="e.g. pattern-auth"
              className="w-full rounded-md border border-gray-300 bg-white px-3 py-2 text-sm text-gray-900 placeholder-gray-400 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500 dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100 dark:placeholder-gray-500"
            />
          </div>
          <div>
            <label className="mb-1 block text-sm font-medium text-gray-700 dark:text-gray-300">
              Value
            </label>
            <textarea
              value={newValue}
              onChange={(e) => setNewValue(e.target.value)}
              rows={3}
              placeholder="Memory content..."
              className="w-full rounded-md border border-gray-300 bg-white px-3 py-2 text-sm text-gray-900 placeholder-gray-400 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500 dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100 dark:placeholder-gray-500"
            />
          </div>
          <div className="flex gap-4">
            <div className="flex-1">
              <label className="mb-1 block text-sm font-medium text-gray-700 dark:text-gray-300">
                Namespace
              </label>
              <input
                type="text"
                value={newNamespace}
                onChange={(e) => setNewNamespace(e.target.value)}
                className="w-full rounded-md border border-gray-300 bg-white px-3 py-2 text-sm text-gray-900 placeholder-gray-400 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500 dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100 dark:placeholder-gray-500"
              />
            </div>
            <div className="flex-1">
              <label className="mb-1 block text-sm font-medium text-gray-700 dark:text-gray-300">
                Tags (comma-separated)
              </label>
              <input
                type="text"
                value={newTags}
                onChange={(e) => setNewTags(e.target.value)}
                placeholder="tag1, tag2"
                className="w-full rounded-md border border-gray-300 bg-white px-3 py-2 text-sm text-gray-900 placeholder-gray-400 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500 dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100 dark:placeholder-gray-500"
              />
            </div>
          </div>
        </div>

        <DialogFooter>
          <Button
            variant="outline"
            size="sm"
            onClick={() => setCreateOpen(false)}
          >
            Cancel
          </Button>
          <Button
            variant="default"
            size="sm"
            onClick={handleCreate}
            disabled={!newKey.trim()}
          >
            Create
          </Button>
        </DialogFooter>
      </Dialog>
    </div>
  );
}
