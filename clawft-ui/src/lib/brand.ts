/**
 * Product display brand (WEFT-176 white-label).
 *
 * Mirrors `clawft_types::config::brand()` / `KernelConfig.brand`.
 * Defaults to "WeftOS"; overridden when the config API exposes
 * `kernel.brand`.
 */

export const DEFAULT_BRAND = "WeftOS";

export interface BrandSource {
  kernel?: {
    brand?: string;
  };
  brand?: string;
}

/** Resolve the display brand from optional config (empty → default). */
export function brand(config?: BrandSource | null): string {
  const fromKernel = config?.kernel?.brand?.trim();
  if (fromKernel) return fromKernel;
  const top = config?.brand?.trim();
  if (top) return top;
  return DEFAULT_BRAND;
}
