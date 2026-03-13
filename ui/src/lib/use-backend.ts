/**
 * Hooks for accessing the backend adapter and capabilities.
 *
 * Separated from mode-context.tsx to satisfy react-refresh rules
 * (files exporting components should not also export non-components).
 */

import { useContext } from "react";
import type { BackendCapabilities } from "./backend-adapter.ts";
import { ModeContext } from "./mode-store.ts";
import type { ModeContextValue } from "./mode-store.ts";

/**
 * Hook to access the current backend adapter and capabilities.
 */
export function useBackend(): ModeContextValue {
  const ctx = useContext(ModeContext);
  if (!ctx) {
    throw new Error("useBackend must be used within ModeProvider");
  }
  return ctx;
}

/**
 * Hook to check if a specific capability is available.
 *
 * Usage: const hasCron = useCapability("cron");
 */
export function useCapability(cap: keyof BackendCapabilities): boolean {
  const { capabilities } = useBackend();
  return capabilities[cap];
}
