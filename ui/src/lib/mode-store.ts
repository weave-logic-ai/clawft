/**
 * Shared ModeContext and types, separated from ModeProvider component
 * to satisfy react-refresh rules (context and components in different files).
 */

import { createContext } from "react";
import type {
  BackendAdapter,
  BackendCapabilities,
  BackendMode,
} from "./backend-adapter.ts";
import type { LoadProgress } from "./wasm-loader.ts";

export interface ModeContextValue {
  adapter: BackendAdapter;
  mode: BackendMode;
  capabilities: BackendCapabilities;
  isReady: boolean;
  loadProgress: LoadProgress | null;
}

export const ModeContext = createContext<ModeContextValue | null>(null);
