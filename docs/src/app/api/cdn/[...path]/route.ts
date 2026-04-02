import { type NextRequest, NextResponse } from 'next/server';

/**
 * Server-side proxy for CDN assets (WASM, KB).
 *
 * GitHub Releases redirects to release-assets.githubusercontent.com which
 * lacks CORS headers. This route handler fetches server-side and streams
 * the response back with correct Content-Type and caching headers.
 *
 * Routes:
 *   /api/cdn/wasm/clawft_wasm.js       → JS glue
 *   /api/cdn/wasm/clawft_wasm_bg.wasm  → WASM binary
 *   /api/cdn/kb/weftos-docs.rvf        → RVF knowledge base
 */

const CDN_ORIGIN =
  process.env.CDN_ORIGIN ||
  'https://github.com/weave-logic-ai/weftos/releases/download/cdn-assets';

const MIME_TYPES: Record<string, string> = {
  '.js': 'application/javascript',
  '.wasm': 'application/wasm',
  '.rvf': 'application/octet-stream',
};

// Allowlist of file paths that can be proxied.
const ALLOWED = new Set([
  'wasm/clawft_wasm.js',
  'wasm/clawft_wasm_bg.wasm',
  'kb/weftos-docs.rvf',
]);

export async function GET(
  _req: NextRequest,
  { params }: { params: Promise<{ path: string[] }> },
) {
  const { path } = await params;
  const filePath = path.join('/');

  if (!ALLOWED.has(filePath)) {
    return NextResponse.json({ error: 'not found' }, { status: 404 });
  }

  // The GitHub Release URL uses the filename directly (no subdirectories).
  const filename = filePath.split('/').pop()!;
  const upstream = `${CDN_ORIGIN}/${filename}`;

  const resp = await fetch(upstream, { redirect: 'follow' });
  if (!resp.ok) {
    return NextResponse.json(
      { error: `upstream ${resp.status}` },
      { status: resp.status },
    );
  }

  const ext = filename.substring(filename.lastIndexOf('.'));
  const contentType = MIME_TYPES[ext] || 'application/octet-stream';

  return new NextResponse(resp.body, {
    status: 200,
    headers: {
      'Content-Type': contentType,
      'Cache-Control': 'public, max-age=604800, immutable',
      'Access-Control-Allow-Origin': '*',
    },
  });
}
