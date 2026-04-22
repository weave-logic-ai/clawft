'use client';

/* RGB-D viz — depth field with sweeping scanline + animated depth band. */

import { useReducedMotion } from 'motion/react';

export default function RgbdViz() {
  const reduce = useReducedMotion();
  const cells = Array.from({ length: 16 * 9 }, (_, i) => i);
  return (
    <svg viewBox="0 0 320 180" preserveAspectRatio="xMidYMid meet" style={{ width: '100%', height: '100%' }}>
      <defs>
        <linearGradient id="rgbd-depth" x1="0" y1="0" x2="1" y2="1">
          <stop offset="0%" stopColor="#6dd3ff" stopOpacity="0.6" />
          <stop offset="50%" stopColor="#7ae0c3" stopOpacity="0.4" />
          <stop offset="100%" stopColor="#b18cff" stopOpacity="0.6" />
        </linearGradient>
      </defs>
      <rect width="320" height="180" fill="#0a141f" />
      {/* depth grid cells with pseudo-random brightness */}
      {cells.map((i) => {
        const col = i % 16;
        const row = Math.floor(i / 16);
        const phase = (col * 0.4 + row * 0.7) % 6.28;
        const fill = `hsl(${190 + ((i * 7) % 60)}, 60%, ${24 + ((i * 13) % 30)}%)`;
        return (
          <rect
            key={i}
            x={col * 20}
            y={row * 20}
            width="19"
            height="19"
            fill={fill}
            opacity={0.55 + ((i * 11) % 5) / 12}
          >
            {!reduce && (
              <animate
                attributeName="opacity"
                values="0.45;0.85;0.45"
                dur={`${4 + (i % 3)}s`}
                begin={`${phase}s`}
                repeatCount="indefinite"
              />
            )}
          </rect>
        );
      })}
      {/* depth haze overlay */}
      <rect width="320" height="180" fill="url(#rgbd-depth)" opacity="0.25" />
      {/* sweeping scanline */}
      {!reduce && (
        <rect width="6" height="180" fill="#6dd3ff" opacity="0.7">
          <animate attributeName="x" from="-6" to="320" dur="3.6s" repeatCount="indefinite" />
        </rect>
      )}
      {/* HUD */}
      <text x="8" y="172" fontFamily="var(--font-mono-lewm), monospace" fontSize="9" fill="#6dd3ff">
        RGB-D · 30 fps · 320×180 → 192d
      </text>
    </svg>
  );
}
