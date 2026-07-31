import { useEffect, useState } from "react";

/**
 * WEFT-573: non-blocking offline banner when the app is running from the
 * service-worker cached shell (or the browser reports `navigator.onLine === false`).
 */
export function OfflineBanner() {
  const [offline, setOffline] = useState(() =>
    typeof navigator !== "undefined" ? !navigator.onLine : false,
  );

  useEffect(() => {
    const goOffline = () => setOffline(true);
    const goOnline = () => setOffline(false);
    window.addEventListener("offline", goOffline);
    window.addEventListener("online", goOnline);
    // Re-check on mount in case the event fired before listeners attached.
    setOffline(!navigator.onLine);
    return () => {
      window.removeEventListener("offline", goOffline);
      window.removeEventListener("online", goOnline);
    };
  }, []);

  if (!offline) return null;

  return (
    <div
      role="status"
      aria-live="polite"
      className="border-b border-amber-300 bg-amber-50 px-4 py-2 text-center text-sm text-amber-900 dark:border-amber-700 dark:bg-amber-950/60 dark:text-amber-100"
    >
      You are offline. Real-time data and write actions (Send, Save, Run) are
      unavailable until you reconnect. Showing the cached app shell.
    </div>
  );
}
