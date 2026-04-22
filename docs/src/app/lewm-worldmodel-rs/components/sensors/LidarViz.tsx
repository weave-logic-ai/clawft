'use client';

/* LiDAR viz — radial sweep with point returns at varying ranges. */

import { useReducedMotion } from 'motion/react';

export default function LidarViz() {
  const reduce = useReducedMotion();
  const cx = 160;
  const cy = 90;
  const points = Array.from({ length: 96 }, (_, i) => {
    const ang = (i / 96) * Math.PI * 2;
    const r = 50 + ((i * 13) % 30) + Math.sin(i * 0.4) * 10;
    return {
      x: cx + Math.cos(ang) * r,
      y: cy + Math.sin(ang) * r * 0.8,
      o: 0.4 + ((i * 7) % 6) / 12,
      i,
    };
  });
  return (
    <svg viewBox="0 0 320 180" preserveAspectRatio="xMidYMid meet" style={{ width: '100%', height: '100%' }}>
      <rect width="320" height="180" fill="#0a141f" />
      {/* range rings */}
      {[30, 50, 70, 90].map((r) => (
        <ellipse
          key={r}
          cx={cx}
          cy={cy}
          rx={r}
          ry={r * 0.8}
          fill="none"
          stroke="#1f2a3c"
          strokeWidth="0.6"
          strokeDasharray="2 4"
        />
      ))}
      {/* origin */}
      <circle cx={cx} cy={cy} r="2.5" fill="#b18cff" />
      {/* points */}
      {points.map((p) => (
        <circle key={p.i} cx={p.x} cy={p.y} r="1.5" fill="#b18cff" opacity={p.o}>
          {!reduce && (
            <animate
              attributeName="opacity"
              values={`${p.o};${p.o * 0.3};${p.o}`}
              dur={`${3 + (p.i % 4)}s`}
              repeatCount="indefinite"
            />
          )}
        </circle>
      ))}
      {/* sweep arm */}
      {!reduce && (
        <line
          x1={cx}
          y1={cy}
          x2={cx + 95}
          y2={cy}
          stroke="#b18cff"
          strokeWidth="1"
          opacity="0.5"
          style={{ transformOrigin: `${cx}px ${cy}px` }}
        >
          <animateTransform
            attributeName="transform"
            type="rotate"
            from={`0 ${cx} ${cy}`}
            to={`360 ${cx} ${cy}`}
            dur="4s"
            repeatCount="indefinite"
          />
        </line>
      )}
      <text x="8" y="172" fontFamily="var(--font-mono-lewm), monospace" fontSize="9" fill="#b18cff">
        LiDAR · 20 Hz · 96-beam → 192d
      </text>
    </svg>
  );
}
