/**
 * Runtime axe-core accessibility suite (WEFT-561 / WEFT-575).
 *
 * Visits each dashboard route under the MSW-backed Vite preview and
 * fails the suite on any axe **critical** WCAG violation. Serious /
 * moderate / minor findings are logged for triage but do not fail the
 * gate (track residual in follow-up WEFT items).
 *
 *   npm run test:a11y
 *   scripts/build.sh ui-e2e   # full Playwright suite includes this file
 */

import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

/** All 14 clawft-ui routes (App.tsx routeTree). */
const ROUTES: ReadonlyArray<{ path: string; name: string }> = [
  { path: "/", name: "dashboard" },
  { path: "/agents", name: "agents" },
  { path: "/canvas", name: "canvas" },
  { path: "/chat", name: "chat" },
  { path: "/sessions", name: "sessions" },
  { path: "/tools", name: "tools" },
  { path: "/skills", name: "skills" },
  { path: "/memory", name: "memory" },
  { path: "/config", name: "config" },
  { path: "/cron", name: "cron" },
  { path: "/channels", name: "channels" },
  { path: "/delegation", name: "delegation" },
  { path: "/monitoring", name: "monitoring" },
  { path: "/voice", name: "voice" },
];

test.describe("axe-core a11y (WEFT-561)", () => {
  for (const route of ROUTES) {
    test(`${route.name} (${route.path}) has no critical violations`, async ({
      page,
    }) => {
      await page.goto(route.path);

      // Wait for React mount so axe does not scan an empty shell.
      await page.waitForFunction(
        () => (document.querySelector("#root")?.children.length ?? 0) > 0,
      );
      // Give route-level data fetches a beat under MSW.
      await page.waitForLoadState("networkidle").catch(() => undefined);

      const results = await new AxeBuilder({ page })
        .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"])
        .analyze();

      const critical = results.violations.filter((v) => v.impact === "critical");
      const serious = results.violations.filter((v) => v.impact === "serious");
      const other = results.violations.filter(
        (v) => v.impact !== "critical" && v.impact !== "serious",
      );

      if (serious.length > 0) {
        // Non-gating: surface for triage without failing CI.
        // eslint-disable-next-line no-console
        console.warn(
          `[a11y] ${route.path}: ${serious.length} serious violation(s):\n` +
            serious
              .map(
                (v) =>
                  `  - ${v.id}: ${v.help} (${v.nodes.length} node(s))`,
              )
              .join("\n"),
        );
      }
      if (other.length > 0) {
        // eslint-disable-next-line no-console
        console.warn(
          `[a11y] ${route.path}: ${other.length} moderate/minor finding(s) tracked, not gated`,
        );
      }

      expect(
        critical,
        critical
          .map(
            (v) =>
              `[critical] ${v.id}: ${v.help}\n` +
              v.nodes
                .slice(0, 3)
                .map((n) => `    ${n.target.join(" ")} — ${n.failureSummary}`)
                .join("\n"),
          )
          .join("\n"),
      ).toEqual([]);
    });
  }
});
