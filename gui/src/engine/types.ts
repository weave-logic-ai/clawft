/**
 * Block descriptor types aligned with block-descriptor-schema.json v0.2.0.
 *
 * These are the runtime types used by the block engine. The canonical schema
 * lives in docs/weftos/specs/block-descriptor-schema.json.
 */

// ---------------------------------------------------------------------------
// Block element types recognised by the catalog
// ---------------------------------------------------------------------------

export type BlockType =
  | 'Column'
  | 'Row'
  | 'Grid'
  | 'Tabs'
  | 'Metric'
  | 'DataTable'
  | 'ChainViewer'
  | 'CausalGraph'
  | 'DiffViewer'
  | 'CodeEditor'
  | 'Button'
  | 'ConsolePan'
  | 'ApprovalGate'
  | 'TextInput'
  | 'Markdown'
  | 'WebBrowser'
  | 'ResourceTree'
  | 'ServiceMap'
  | 'StatusBar'
  | 'RadialTopology'
  | 'HintBar'
  | 'ProgressBar'
  | 'Budget';

// ---------------------------------------------------------------------------
// $state reference
// ---------------------------------------------------------------------------

export interface StateRef {
  $state: string;
  $default?: unknown;
  $transform?: string;
}

export interface FormatStateRef {
  $state: string;
  format: string;
}

export type PropValue =
  | string
  | number
  | boolean
  | null
  | PropValue[]
  | StateRef
  | FormatStateRef
  | { [key: string]: PropValue };

export function isStateRef(v: unknown): v is StateRef {
  return typeof v === 'object' && v !== null && '$state' in v && !('format' in v);
}

export function isFormatStateRef(v: unknown): v is FormatStateRef {
  return typeof v === 'object' && v !== null && '$state' in v && 'format' in v;
}

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

export type ActionType =
  | 'kernel_exec'
  | 'governance_check'
  | 'chain_query'
  | 'ecc_search'
  | 'agent_chat'
  | 'navigate'
  | 'open_block'
  | 'close_block';

export interface BlockAction {
  action: ActionType;
  params?: Record<string, PropValue>;
  governed?: boolean;
}

// ---------------------------------------------------------------------------
// Layout hints
// ---------------------------------------------------------------------------

export interface LayoutHints {
  width?: number;
  height?: number;
  x?: number;
  y?: number;
  minWidth?: number;
  minHeight?: number;
  resizable?: boolean;
}

// ---------------------------------------------------------------------------
// Port bindings
// ---------------------------------------------------------------------------

export interface PortBinding {
  direction: 'in' | 'out';
  data_type: string;
  source?: { element: string; port: string };
  publish?: string;
}

// ---------------------------------------------------------------------------
// Block element
// ---------------------------------------------------------------------------

export interface BlockElement {
  type: BlockType | string; // allow custom types via string fallback
  children?: string[];
  props?: Record<string, PropValue>;
  on?: Record<string, BlockAction>;
  ports?: Record<string, PortBinding>;
  layout?: LayoutHints;
}

// ---------------------------------------------------------------------------
// Block descriptor (top-level document)
// ---------------------------------------------------------------------------

export interface BlockMeta {
  creator?: string;
  created_at?: string;
  title?: string;
  tags?: string[];
  governance_seq?: number;
  target_hint?: string;
  refresh_hz?: number;
}

export interface BlockDescriptor {
  version: string;
  root: string;
  elements: Record<string, BlockElement>;
  meta?: BlockMeta;
}
