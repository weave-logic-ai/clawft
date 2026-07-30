// Unit tests for WEFT-558 chip a11y / E2E helpers.
//
// Runnable with `npx --yes tsx --test src/chipA11y.test.ts` from this
// package directory (no VSCode host required).

import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
    CHIP_ATTR,
    CHIP_ID_ATTR,
    CHIP_STRIP_ATTR,
    CHIP_STRIP_ID,
    MOCK_E2E_CHIPS,
    injectMockChipsIntoHtml,
    isE2eChipMode,
    parseChipsFromHtml,
    renderChipA11yHtml,
} from "./chipA11y";

describe("MOCK_E2E_CHIPS", () => {
    it("exposes >=1 chip with stable kernel id", () => {
        assert.ok(MOCK_E2E_CHIPS.length >= 1);
        assert.ok(MOCK_E2E_CHIPS.some((c) => c.id === "kernel"));
    });

    it("covers all four tray ChipIds", () => {
        const ids = MOCK_E2E_CHIPS.map((c) => c.id).sort();
        assert.deepEqual(ids, ["exochain", "explorer", "kernel", "mesh"]);
    });
});

describe("renderChipA11yHtml", () => {
    it("renders empty strip with markers when no chips", () => {
        const html = renderChipA11yHtml([]);
        assert.match(html, new RegExp(`id="${CHIP_STRIP_ID}"`));
        assert.match(html, new RegExp(CHIP_STRIP_ATTR));
        // Per-chip marker must not appear (use word-ish boundary).
        assert.equal(/\bdata-weft-chip(?:\s|=|>)/.test(html), false);
    });

    it("renders each chip with data-chip-id", () => {
        const html = renderChipA11yHtml(MOCK_E2E_CHIPS);
        for (const c of MOCK_E2E_CHIPS) {
            assert.match(
                html,
                new RegExp(`${CHIP_ID_ATTR}="${c.id}"`),
                `missing ${c.id}`,
            );
        }
        assert.equal(
            (html.match(/\bdata-weft-chip(?:\s|=|>)/g) ?? []).length,
            MOCK_E2E_CHIPS.length,
        );
    });

    it("escapes HTML in labels", () => {
        const html = renderChipA11yHtml([
            { id: "x", label: `<script>"evil"</script>`, tone: "crit" },
        ]);
        assert.equal(html.includes("<script>"), false);
        assert.match(html, /&lt;script&gt;/);
    });
});

describe("parseChipsFromHtml", () => {
    it("round-trips MOCK_E2E_CHIPS", () => {
        const html = renderChipA11yHtml(MOCK_E2E_CHIPS);
        const parsed = parseChipsFromHtml(html);
        assert.equal(parsed.length, MOCK_E2E_CHIPS.length);
        assert.deepEqual(
            parsed.map((c) => c.id).sort(),
            MOCK_E2E_CHIPS.map((c) => c.id).sort(),
        );
        const kernel = parsed.find((c) => c.id === "kernel");
        assert.ok(kernel);
        assert.equal(kernel!.label, "Kernel");
        assert.equal(kernel!.tone, "ok");
    });

    it("returns [] for unrelated HTML", () => {
        assert.deepEqual(parseChipsFromHtml("<html><body>hi</body></html>"), []);
        assert.deepEqual(parseChipsFromHtml(""), []);
    });
});

describe("injectMockChipsIntoHtml", () => {
    it("replaces an empty strip in a document", () => {
        const empty = `<!DOCTYPE html><html><body>${renderChipA11yHtml([])}</body></html>`;
        const out = injectMockChipsIntoHtml(empty);
        const chips = parseChipsFromHtml(out);
        assert.ok(chips.length >= 1);
        assert.ok(chips.some((c) => c.id === "kernel"));
        // Single strip only.
        assert.equal((out.match(/id="weft-chip-a11y"/g) ?? []).length, 1);
    });

    it("appends strip when missing", () => {
        const bare = `<!DOCTYPE html><html><body><div id="weft-root"></div></body></html>`;
        const out = injectMockChipsIntoHtml(bare);
        assert.ok(parseChipsFromHtml(out).length >= 1);
        assert.match(out, /<\/div>\s*<\/body>/);
    });
});

describe("isE2eChipMode", () => {
    it("defaults off", () => {
        assert.equal(isE2eChipMode({}), false);
        assert.equal(isE2eChipMode({ WEFT_PANEL_E2E: "" }), false);
        assert.equal(isE2eChipMode({ WEFT_PANEL_E2E: "0" }), false);
    });

    it("honours 1/true/yes", () => {
        assert.equal(isE2eChipMode({ WEFT_PANEL_E2E: "1" }), true);
        assert.equal(isE2eChipMode({ WEFT_PANEL_E2E: "true" }), true);
        assert.equal(isE2eChipMode({ WEFT_PANEL_E2E: "YES" }), true);
    });
});
