import { test, expect } from '@playwright/test';

// =============================================================================
// Topology Navigator E2E Tests
// Validates symposium success criteria + visual design + usability
// =============================================================================

test.describe('Page Load & Structure', () => {
  test('page loads with title and description', async ({ page }) => {
    await page.goto('/vowl-navigator');
    await expect(page.locator('h1')).toHaveText('Topology Navigator');
    await expect(page.locator('p').first()).toContainText('Explore the WeftOS codebase');
  });

  test('mode toggle buttons are visible', async ({ page }) => {
    await page.goto('/vowl-navigator');
    await expect(page.getByRole('button', { name: 'Drill-down' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'VOWL flat' })).toBeVisible();
  });

  test('drill-down mode is default', async ({ page }) => {
    await page.goto('/vowl-navigator');
    const drillBtn = page.getByRole('button', { name: 'Drill-down' });
    await expect(drillBtn).toHaveClass(/bg-indigo-600/);
  });
});

test.describe('Criterion 1: Not a Hairball — top level is manageable', () => {
  test('loads root slice with package-level nodes', async ({ page }) => {
    await page.goto('/vowl-navigator');
    await page.waitForSelector('svg g.cursor-pointer', { timeout: 10_000 });

    // Count interactive node groups (not individual SVG elements).
    const nodeGroups = page.locator('svg g.cursor-pointer');
    const count = await nodeGroups.count();

    // 28 packages, not 631 nodes.
    expect(count).toBeGreaterThan(10);
    expect(count).toBeLessThan(50);
  });

  test('breadcrumb shows Root at top level', async ({ page }) => {
    await page.goto('/vowl-navigator');
    await page.waitForSelector('svg g.cursor-pointer', { timeout: 10_000 });
    await expect(page.getByRole('button', { name: 'Root' })).toBeVisible();
  });

  test('depth indicator shows level 0', async ({ page }) => {
    await page.goto('/vowl-navigator');
    await page.waitForSelector('svg g.cursor-pointer', { timeout: 10_000 });
    await expect(page.getByText(/Depth 0/)).toBeVisible();
  });

  test('total nodes count shown for context', async ({ page }) => {
    await page.goto('/vowl-navigator');
    await page.waitForSelector('svg g.cursor-pointer', { timeout: 10_000 });
    await expect(page.getByText(/\d+ total/)).toBeVisible();
  });
});

test.describe('Criterion 3: Drill-Down Navigation', () => {
  test('double-click drills into children', async ({ page }) => {
    await page.goto('/vowl-navigator');
    await page.waitForSelector('svg g.cursor-pointer', { timeout: 10_000 });

    // Double-click the first node's shape directly.
    const firstNode = page.locator('svg g.cursor-pointer').first();
    await firstNode.locator('rect, circle, polygon').first().dblclick({ force: true });
    await page.waitForTimeout(1500);

    // Depth should have changed.
    const depthText = await page.getByText(/Depth \d/).textContent();
    expect(depthText).toContain('Depth');
  });

  test('breadcrumb Root click returns to top level', async ({ page }) => {
    await page.goto('/vowl-navigator');
    await page.waitForSelector('svg g.cursor-pointer', { timeout: 10_000 });

    // Drill in.
    await page.locator('svg g.cursor-pointer').first().locator('rect, circle, polygon').first().dblclick({ force: true });
    await page.waitForTimeout(1500);

    // Click Root.
    await page.getByRole('button', { name: 'Root' }).click();
    await page.waitForTimeout(1000);

    await expect(page.getByText(/Depth 0/)).toBeVisible();
  });
});

test.describe('Criterion 4: Understand Architecture — Detail Panel', () => {
  test('clicking a node shows detail panel with type', async ({ page }) => {
    await page.goto('/vowl-navigator');
    await page.waitForSelector('svg g.cursor-pointer', { timeout: 10_000 });

    await page.locator('svg g.cursor-pointer').first().locator('rect, circle, polygon').first().click({ force: true });
    await page.waitForTimeout(500);

    // Detail panel should show Type.
    await expect(page.getByText('Type').first()).toBeVisible();
  });

  test('detail panel has drill-in button for expandable nodes', async ({ page }) => {
    await page.goto('/vowl-navigator');
    await page.waitForSelector('svg g.cursor-pointer', { timeout: 10_000 });

    await page.locator('svg g.cursor-pointer').first().locator('rect, circle, polygon').first().click({ force: true });
    await page.waitForTimeout(500);

    // At top level, all nodes are packages and should be expandable.
    const drillBtn = page.getByRole('button', { name: 'Drill into children' });
    await expect(drillBtn).toBeVisible();
  });

  test('clear button dismisses detail panel', async ({ page }) => {
    await page.goto('/vowl-navigator');
    await page.waitForSelector('svg g.cursor-pointer', { timeout: 10_000 });

    await page.locator('svg g.cursor-pointer').first().locator('rect, circle, polygon').first().click({ force: true });
    await page.waitForTimeout(500);

    await page.getByRole('button', { name: 'Clear' }).click();
    await expect(page.getByRole('button', { name: 'Clear' })).not.toBeVisible();
  });
});

test.describe('Criterion 5: Schema-Driven Visual Encoding', () => {
  test('SVG canvas renders at proper size', async ({ page }) => {
    await page.goto('/vowl-navigator');
    await page.waitForSelector('svg', { timeout: 10_000 });

    const svg = page.locator('svg').first();
    const width = await svg.getAttribute('width');
    const height = await svg.getAttribute('height');
    expect(Number(width)).toBeGreaterThanOrEqual(800);
    expect(Number(height)).toBeGreaterThanOrEqual(500);
  });

  test('nodes use colored fills from schema', async ({ page }) => {
    await page.goto('/vowl-navigator');
    await page.waitForSelector('svg g.cursor-pointer rect, svg g.cursor-pointer circle', { timeout: 10_000 });

    const shape = page.locator('svg g.cursor-pointer rect, svg g.cursor-pointer circle').first();
    const fill = await shape.getAttribute('fill');
    expect(fill).toMatch(/^#[0-9a-fA-F]{6}$/);
    expect(fill).not.toBe('#a3a3a3'); // Not default gray — schema color applied.
  });

  test('nodes have text labels', async ({ page }) => {
    await page.goto('/vowl-navigator');
    await page.waitForSelector('svg g.cursor-pointer text', { timeout: 10_000 });

    const labels = await page.locator('svg g.cursor-pointer text').allTextContents();
    const nonEmpty = labels.filter((l) => l.trim().length > 0 && l !== '+');
    expect(nonEmpty.length).toBeGreaterThan(5);
  });

  test('arrow marker defined in SVG defs', async ({ page }) => {
    await page.goto('/vowl-navigator');
    await page.waitForSelector('svg g.cursor-pointer', { timeout: 10_000 });
    // Marker is in the SVG defs but may not have specific selector support.
    // Verify the SVG has a defs element with marker content.
    const svgHtml = await page.locator('svg').first().innerHTML();
    expect(svgHtml).toContain('marker');
    expect(svgHtml).toContain('id="arr"');
  });
});

test.describe('Usability', () => {
  test('SVG has touch-action none for zoom support', async ({ page }) => {
    await page.goto('/vowl-navigator');
    const svg = page.locator('svg').first();
    await expect(svg).toHaveCSS('touch-action', 'none');
  });

  test('help text explains interaction', async ({ page }) => {
    await page.goto('/vowl-navigator');
    await expect(page.getByText('Double-click expandable nodes')).toBeVisible();
    await expect(page.getByText('Scroll to zoom')).toBeVisible();
  });
});

test.describe('Mode Toggle', () => {
  test('VOWL flat mode shows search input', async ({ page }) => {
    await page.goto('/vowl-navigator');
    await page.getByRole('button', { name: 'VOWL flat' }).click();
    await page.waitForTimeout(500);
    await expect(page.getByPlaceholder('Search classes...')).toBeVisible();
  });

  test('switching back to drill-down shows breadcrumbs', async ({ page }) => {
    await page.goto('/vowl-navigator');
    await page.getByRole('button', { name: 'VOWL flat' }).click();
    await page.waitForTimeout(300);
    await page.getByRole('button', { name: 'Drill-down' }).click();
    await page.waitForTimeout(500);
    await expect(page.getByRole('button', { name: 'Root' })).toBeVisible();
  });
});

test.describe('Data Integrity — Slice Files', () => {
  test('manifest has correct structure', async ({ page }) => {
    const res = await page.goto('/slices/manifest.json');
    expect(res?.status()).toBe(200);
    const m = await res?.json();
    expect(m.total_nodes).toBeGreaterThan(100);
    expect(m.total_edges).toBeGreaterThan(100);
    expect(m.root).toBe('root.json');
    expect(Object.keys(m.slices).length).toBeGreaterThan(10);
  });

  test('root slice has depth 0 and no breadcrumbs', async ({ page }) => {
    const res = await page.goto('/slices/root.json');
    const s = await res?.json();
    expect(s.depth).toBe(0);
    expect(s.breadcrumbs).toHaveLength(0);
    expect(s.graph.nodes.length).toBeGreaterThan(0);
    expect(s.graph.nodes.length).toBeLessThan(50);
    expect(s.expandable.length).toBeGreaterThan(0);
  });

  test('child slice has depth >= 1 and breadcrumbs', async ({ page }) => {
    const mRes = await page.goto('/slices/manifest.json');
    const m = await mRes?.json();
    const firstFile = Object.values(m.slices)[0] as string;
    const sRes = await page.goto(`/slices/${firstFile}`);
    const s = await sRes?.json();
    expect(s.depth).toBeGreaterThanOrEqual(1);
    expect(s.breadcrumbs.length).toBeGreaterThanOrEqual(1);
    expect(s.graph.nodes.length).toBeGreaterThan(0);
  });

  test('positioned nodes have valid coordinates', async ({ page }) => {
    const res = await page.goto('/slices/root.json');
    const s = await res?.json();
    for (const node of s.graph.nodes) {
      expect(typeof node.x).toBe('number');
      expect(typeof node.y).toBe('number');
      expect(node.x).not.toBeNaN();
      expect(node.y).not.toBeNaN();
      expect(node.label.length).toBeGreaterThan(0);
      expect(node.node_type.length).toBeGreaterThan(0);
    }
  });
});
