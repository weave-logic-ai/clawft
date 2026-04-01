/**
 * Engine barrel export.
 */
export { BlockRenderer, ElementRenderer } from './BlockRenderer';
export { registerBlock, getBlock, hasBlock, listBlocks } from './BlockRegistry';
export { useStateStore, resolveProps, startTauriSync, stopTauriSync } from './StateStore';
export { KernelDataProvider } from './KernelDataProvider';
export type { BlockComponentProps } from './BlockRegistry';
export type {
  BlockDescriptor,
  BlockElement,
  BlockType,
  BlockMeta,
  BlockAction,
  LayoutHints,
  PropValue,
  StateRef,
  FormatStateRef,
} from './types';
