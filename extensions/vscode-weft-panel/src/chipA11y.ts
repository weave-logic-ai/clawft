// WEFT-558 — DOM-side chip-strip a11y / E2E surface.
//
// Tray chips paint inside the egui canvas (wasm), so plain
// `webview.html` introspection cannot see them. This module owns a
// **DOM-side mirror** of the tray chip set:
//
//   • Stable identifiers match `clawft-gui-egui::shell::tray::ChipId`
//     labels (kernel / mesh / exochain / explorer).
//   • The strip is rendered into the webview HTML as
//     `#weft-chip-a11y [data-weft-chip][data-chip-id=…]` elements the
//     host (and E2E tests) can parse without touching the canvas.
//   • Production keeps the strip empty (container only). E2E injects
//     a known mock set via `injectMockChipsIntoHtml` / the
//     `weft._test.chipStripSnapshot` command.
//
// Pure functions — unit-testable without a VSCode host.

/** One chip as exposed on the DOM a11y / E2E surface. */
export interface A11yChip {
    /** Stable id — lowercase ChipId name. */
    id: string;
    /** Human label (matches tray). */
    label: string;
    /** Visual tone for the mock / a11y mirror. */
    tone: "ok" | "warn" | "off" | "crit";
}

/**
 * Canonical mock set for E2E. Mirrors the four tray ChipIds so the
 * assertion has stable identifiers independent of daemon state.
 */
export const MOCK_E2E_CHIPS: readonly A11yChip[] = Object.freeze([
    { id: "kernel", label: "Kernel", tone: "ok" as const },
    { id: "mesh", label: "Mesh", tone: "off" as const },
    { id: "exochain", label: "ExoChain", tone: "off" as const },
    { id: "explorer", label: "Explorer", tone: "ok" as const },
]);

/** Marker attributes the E2E suite greps for. */
export const CHIP_STRIP_ID = "weft-chip-a11y";
/** Container marker — deliberately not a prefix of CHIP_ATTR. */
export const CHIP_STRIP_ATTR = "data-weft-status-strip";
/** Per-chip marker (list items only). */
export const CHIP_ATTR = "data-weft-chip";
export const CHIP_ID_ATTR = "data-chip-id";

/** Visually-hidden but still in the accessibility / DOM tree. */
const SR_ONLY_STYLE =
    "position:absolute;width:1px;height:1px;padding:0;margin:-1px;" +
    "overflow:hidden;clip:rect(0,0,0,0);white-space:nowrap;border:0";

function escapeHtml(s: string): string {
    return s
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;")
        .replace(/"/g, "&quot;")
        .replace(/'/g, "&#39;");
}

/**
 * Render the chip-strip container (optionally populated).
 * Always returns a single outer `#weft-chip-a11y` element so replace
 * logic can find it deterministically.
 */
export function renderChipA11yHtml(chips: readonly A11yChip[] = []): string {
    const items = chips
        .map(
            (c) =>
                `<span role="listitem" ${CHIP_ATTR} ${CHIP_ID_ATTR}="${escapeHtml(c.id)}" ` +
                `data-chip-tone="${escapeHtml(c.tone)}" data-chip-label="${escapeHtml(c.label)}">` +
                `${escapeHtml(c.label)}</span>`,
        )
        .join("");
    return (
        `<div id="${CHIP_STRIP_ID}" role="list" ${CHIP_STRIP_ATTR} ` +
        `aria-label="WeftOS status chips" style="${SR_ONLY_STYLE}">` +
        `${items}</div>`
    );
}

/**
 * Parse chip entries out of a full webview HTML document (or fragment).
 * Returns [] when the strip is missing or empty.
 */
export function parseChipsFromHtml(html: string): A11yChip[] {
    if (!html) {
        return [];
    }
    // Word-boundary after CHIP_ATTR so the container's
    // data-weft-status-strip (or any *-chip-* attr) is not a hit.
    const elementRe = /<span\b[^>]*\bdata-weft-chip(?:\s|=|>)[^>]*>/gi;
    const chips: A11yChip[] = [];
    let m: RegExpExecArray | null;
    while ((m = elementRe.exec(html)) !== null) {
        const tag = m[0];
        const id = attr(tag, "data-chip-id");
        if (!id) continue;
        const toneRaw = attr(tag, "data-chip-tone") ?? "off";
        const label = attr(tag, "data-chip-label") ?? id;
        const tone = normalizeTone(toneRaw);
        chips.push({ id, label, tone });
    }
    return chips;
}

function attr(tag: string, name: string): string | undefined {
    const m = new RegExp(`\\b${name}="([^"]*)"`, "i").exec(tag);
    return m ? m[1] : undefined;
}

function normalizeTone(t: string): A11yChip["tone"] {
    if (t === "ok" || t === "warn" || t === "off" || t === "crit") return t;
    return "off";
}

/**
 * Insert or replace the chip strip inside a full HTML document.
 * Idempotent: re-running with a new chip set overwrites the previous
 * strip contents without duplicating the container.
 */
export function injectMockChipsIntoHtml(
    html: string,
    chips: readonly A11yChip[] = MOCK_E2E_CHIPS,
): string {
    const strip = renderChipA11yHtml(chips);
    // Match the whole strip div (greedy to the first matching close).
    const stripRe =
        /<div\b[^>]*\bid=["']weft-chip-a11y["'][^>]*>[\s\S]*?<\/div>/i;
    if (stripRe.test(html)) {
        return html.replace(stripRe, strip);
    }
    if (html.includes("</body>")) {
        return html.replace("</body>", `${strip}\n</body>`);
    }
    return html + strip;
}

/** True when the E2E harness (or an operator) requested mock chips. */
export function isE2eChipMode(
    env: NodeJS.ProcessEnv = process.env,
): boolean {
    const v = (env.WEFT_PANEL_E2E ?? "").trim().toLowerCase();
    return v === "1" || v === "true" || v === "yes";
}

/** Snapshot payload returned by `weft._test.chipStripSnapshot`. */
export interface ChipStripSnapshot {
    chips: A11yChip[];
    source: "html";
}
