'use client';

/* IMU viz — three traces (ax, ay, az) sweeping across the panel. */

import { useReducedMotion } from 'motion/react';

function trace(seed: number, amp: number, samples = 80) {
  const pts: string[] = [];
  for (let i = 0; i < samples; i++) {
    const t = i / samples;
    const v =
      Math.sin(t * Math.PI * 2 + seed) * amp +
      Math.sin(t * Math.PI * 6 + seed * 1.7) * amp * 0.3 +
      Math.sin(t * Math.PI * 10 + seed * 2.1) * amp * 0.15;
    pts.push(`${(i / (samples - 1)) * 320},${90 + v}`);
  }
  return 'M ' + pts.join(' L ');
}

export default function ImuViz() {
  const reduce = useReducedMotion();
  return (
    <svg viewBox="0 0 320 180" preserveAspectRatio="none" style={{ width: '100%', height: '100%' }}>
      <rect width="320" height="180" fill="#0a141f" />
      {/* zero line */}
      <line x1="0" y1="90" x2="320" y2="90" stroke="#1f2a3c" strokeWidth="1" strokeDasharray="2 4" />
      {/* axes */}
      {[
        { color: '#6dd3ff', amp: 28, seed: 0.4, label: 'ax' },
        { color: '#7ae0c3', amp: 22, seed: 1.2, label: 'ay' },
        { color: '#b18cff', amp: 34, seed: 2.1, label: 'az' },
      ].map((a, i) => (
        <g key={a.label}>
          <path d={trace(a.seed, a.amp)} fill="none" stroke={a.color} strokeWidth="1.4" opacity="0.9">
            {!reduce && (
              <animate
                attributeName="d"
                values={`${trace(a.seed, a.amp)};${trace(a.seed + 1.4, a.amp)};${trace(a.seed, a.amp)}`}
                dur={`${4 + i * 0.6}s`}
                repeatCount="indefinite"
              />
            )}
          </path>
          <text
            x="6"
            y={20 + i * 12}
            fontFamily="var(--font-mono-lewm), monospace"
            fontSize="9"
            fill={a.color}
          >
            {a.label}
          </text>
        </g>
      ))}
      {!reduce && (
        <rect width="2" height="180" fill="#6dd3ff" opacity="0.5">
          <animate attributeName="x" from="-2" to="320" dur="2.8s" repeatCount="indefinite" />
        </rect>
      )}
      <text x="8" y="172" fontFamily="var(--font-mono-lewm), monospace" fontSize="9" fill="#6dd3ff">
        IMU · 1 kHz · 6-axis → 192d
      </text>
    </svg>
  );
}
