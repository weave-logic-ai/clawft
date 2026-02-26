import { useState, useEffect, useMemo } from "react";
import { useCronStore } from "../stores/cron-store";
import { Badge } from "../components/ui/badge";
import { Button } from "../components/ui/button";
import { Skeleton } from "../components/ui/skeleton";
import { Dialog, DialogHeader, DialogTitle, DialogFooter } from "../components/ui/dialog";
import { formatRelativeTime } from "../lib/utils";
import type { CronJob } from "../lib/types";

function cronStatusVariant(
  status: CronJob["status"],
): "success" | "secondary" | "destructive" {
  switch (status) {
    case "running":
      return "success";
    case "idle":
      return "secondary";
    case "failed":
      return "destructive";
  }
}

/**
 * Computes the next N fire times for a cron expression.
 * This is a simplified parser that handles basic `m h dom mon dow` patterns.
 */
function computeNextFireTimes(schedule: string, count: number): Date[] {
  const results: Date[] = [];
  const parts = schedule.split(/\s+/);
  if (parts.length < 5) return results;

  const now = new Date();
  const cursor = new Date(now);
  cursor.setSeconds(0, 0);
  cursor.setMinutes(cursor.getMinutes() + 1);

  const matchesPart = (value: number, part: string): boolean => {
    if (part === "*") return true;
    if (part.startsWith("*/")) {
      const step = parseInt(part.slice(2), 10);
      return value % step === 0;
    }
    const nums = part.split(",").map((n) => parseInt(n, 10));
    return nums.includes(value);
  };

  let iterations = 0;
  while (results.length < count && iterations < 525600) {
    iterations++;
    const minute = cursor.getMinutes();
    const hour = cursor.getHours();
    const dom = cursor.getDate();
    const month = cursor.getMonth() + 1;
    const dow = cursor.getDay();

    if (
      matchesPart(minute, parts[0]) &&
      matchesPart(hour, parts[1]) &&
      matchesPart(dom, parts[2]) &&
      matchesPart(month, parts[3]) &&
      matchesPart(dow, parts[4])
    ) {
      results.push(new Date(cursor));
    }
    cursor.setMinutes(cursor.getMinutes() + 1);
  }

  return results;
}

function NextFirePreview({ schedule }: { schedule: string }) {
  const times = useMemo(() => computeNextFireTimes(schedule, 5), [schedule]);

  if (times.length === 0) {
    return (
      <span className="text-xs text-gray-400">Unable to compute</span>
    );
  }

  return (
    <div className="space-y-0.5">
      {times.map((t, i) => (
        <div key={i} className="text-xs text-gray-500 dark:text-gray-400">
          {t.toLocaleString()}
        </div>
      ))}
    </div>
  );
}

export function CronPage() {
  const {
    jobs,
    loading,
    fetchJobs,
    createJob,
    deleteJob,
    runNow,
    toggleJob,
  } = useCronStore();

  const [dialogOpen, setDialogOpen] = useState(false);
  const [previewJob, setPreviewJob] = useState<string | null>(null);
  const [newName, setNewName] = useState("");
  const [newSchedule, setNewSchedule] = useState("*/5 * * * *");
  const [newPayload, setNewPayload] = useState("");

  useEffect(() => {
    fetchJobs();
  }, [fetchJobs]);

  const handleCreate = async () => {
    if (!newName.trim() || !newSchedule.trim()) return;
    await createJob({
      name: newName,
      schedule: newSchedule,
      enabled: true,
      payload: newPayload || undefined,
    });
    setNewName("");
    setNewSchedule("*/5 * * * *");
    setNewPayload("");
    setDialogOpen(false);
  };

  return (
    <div className="flex-1 overflow-auto p-6 space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900 dark:text-gray-100">
            Cron Jobs
          </h1>
          <p className="text-sm text-gray-500 dark:text-gray-400">
            Schedule and manage recurring tasks
          </p>
        </div>
        <div className="flex items-center gap-3">
          {jobs.length > 0 && (
            <Badge variant="secondary">{jobs.length} jobs</Badge>
          )}
          <Button
            variant="default"
            size="sm"
            onClick={() => setDialogOpen(true)}
          >
            Create Job
          </Button>
        </div>
      </div>

      {/* Data table */}
      <div className="overflow-hidden rounded-lg border border-gray-200 dark:border-gray-700">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-gray-200 bg-gray-50 dark:border-gray-700 dark:bg-gray-800/50">
              <th className="px-4 py-3 text-left font-medium text-gray-600 dark:text-gray-400">
                Name
              </th>
              <th className="px-4 py-3 text-left font-medium text-gray-600 dark:text-gray-400">
                Schedule
              </th>
              <th className="px-4 py-3 text-left font-medium text-gray-600 dark:text-gray-400">
                Status
              </th>
              <th className="px-4 py-3 text-left font-medium text-gray-600 dark:text-gray-400">
                Last Run
              </th>
              <th className="px-4 py-3 text-left font-medium text-gray-600 dark:text-gray-400">
                Next Run
              </th>
              <th className="px-4 py-3 text-right font-medium text-gray-600 dark:text-gray-400">
                Actions
              </th>
            </tr>
          </thead>
          <tbody>
            {loading ? (
              Array.from({ length: 4 }).map((_, i) => (
                <tr
                  key={i}
                  className="border-b border-gray-100 dark:border-gray-700"
                >
                  <td className="px-4 py-3">
                    <Skeleton className="h-4 w-28" />
                  </td>
                  <td className="px-4 py-3">
                    <Skeleton className="h-4 w-24" />
                  </td>
                  <td className="px-4 py-3">
                    <Skeleton className="h-5 w-16" />
                  </td>
                  <td className="px-4 py-3">
                    <Skeleton className="h-4 w-20" />
                  </td>
                  <td className="px-4 py-3">
                    <Skeleton className="h-4 w-20" />
                  </td>
                  <td className="px-4 py-3">
                    <Skeleton className="h-8 w-32" />
                  </td>
                </tr>
              ))
            ) : jobs.length === 0 ? (
              <tr>
                <td
                  colSpan={6}
                  className="px-4 py-12 text-center text-gray-500 dark:text-gray-400"
                >
                  No cron jobs configured.
                </td>
              </tr>
            ) : (
              jobs.map((job) => (
                <tr
                  key={job.id}
                  className="border-b border-gray-100 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-800/50"
                >
                  <td className="px-4 py-3 font-medium text-gray-900 dark:text-gray-100">
                    {job.name}
                  </td>
                  <td className="px-4 py-3">
                    <button
                      className="font-mono text-xs text-blue-600 hover:underline dark:text-blue-400"
                      onClick={() =>
                        setPreviewJob(
                          previewJob === job.id ? null : job.id,
                        )
                      }
                      title="Click to preview next fire times"
                    >
                      {job.schedule}
                    </button>
                    {previewJob === job.id && (
                      <div className="mt-2 rounded border border-gray-200 bg-gray-50 p-2 dark:border-gray-700 dark:bg-gray-800">
                        <p className="mb-1 text-xs font-medium text-gray-600 dark:text-gray-400">
                          Next 5 fire times:
                        </p>
                        <NextFirePreview schedule={job.schedule} />
                      </div>
                    )}
                  </td>
                  <td className="px-4 py-3">
                    <Badge variant={cronStatusVariant(job.status)}>
                      {job.status}
                    </Badge>
                  </td>
                  <td className="px-4 py-3 text-gray-500 dark:text-gray-400">
                    {job.last_run
                      ? formatRelativeTime(job.last_run)
                      : "-"}
                  </td>
                  <td className="px-4 py-3 text-gray-500 dark:text-gray-400">
                    {job.next_run
                      ? formatRelativeTime(job.next_run)
                      : "-"}
                  </td>
                  <td className="px-4 py-3 text-right">
                    <div className="flex items-center justify-end gap-2">
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() =>
                          toggleJob(job.id, !job.enabled)
                        }
                      >
                        {job.enabled ? "Disable" : "Enable"}
                      </Button>
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => runNow(job.id)}
                        disabled={!job.enabled}
                      >
                        Run Now
                      </Button>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => deleteJob(job.id)}
                      >
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

      {/* Create Dialog */}
      <Dialog open={dialogOpen} onClose={() => setDialogOpen(false)}>
        <DialogHeader>
          <DialogTitle>Create Cron Job</DialogTitle>
        </DialogHeader>

        <div className="space-y-4">
          <div>
            <label className="mb-1 block text-sm font-medium text-gray-700 dark:text-gray-300">
              Name
            </label>
            <input
              type="text"
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
              placeholder="e.g. health-check"
              className="w-full rounded-md border border-gray-300 bg-white px-3 py-2 text-sm text-gray-900 placeholder-gray-400 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500 dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100 dark:placeholder-gray-500"
            />
          </div>
          <div>
            <label className="mb-1 block text-sm font-medium text-gray-700 dark:text-gray-300">
              Cron Expression
            </label>
            <input
              type="text"
              value={newSchedule}
              onChange={(e) => setNewSchedule(e.target.value)}
              placeholder="*/5 * * * *"
              className="w-full rounded-md border border-gray-300 bg-white px-3 py-2 text-sm font-mono text-gray-900 placeholder-gray-400 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500 dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100 dark:placeholder-gray-500"
            />
            {newSchedule && (
              <div className="mt-2 rounded border border-gray-200 bg-gray-50 p-2 dark:border-gray-700 dark:bg-gray-800">
                <p className="mb-1 text-xs font-medium text-gray-600 dark:text-gray-400">
                  Preview:
                </p>
                <NextFirePreview schedule={newSchedule} />
              </div>
            )}
          </div>
          <div>
            <label className="mb-1 block text-sm font-medium text-gray-700 dark:text-gray-300">
              Payload (JSON, optional)
            </label>
            <textarea
              value={newPayload}
              onChange={(e) => setNewPayload(e.target.value)}
              rows={2}
              placeholder='{"key": "value"}'
              className="w-full rounded-md border border-gray-300 bg-white px-3 py-2 text-sm font-mono text-gray-900 placeholder-gray-400 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500 dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100 dark:placeholder-gray-500"
            />
          </div>
        </div>

        <DialogFooter>
          <Button
            variant="outline"
            size="sm"
            onClick={() => setDialogOpen(false)}
          >
            Cancel
          </Button>
          <Button
            variant="default"
            size="sm"
            onClick={handleCreate}
            disabled={!newName.trim() || !newSchedule.trim()}
          >
            Create
          </Button>
        </DialogFooter>
      </Dialog>
    </div>
  );
}
