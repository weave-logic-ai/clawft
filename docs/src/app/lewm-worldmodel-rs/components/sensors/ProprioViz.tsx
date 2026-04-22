'use client';

/* Proprioception viz — 12 joint bars (per-joint angle + torque), live-pulsing. */

import { useReducedMotion } from 'motion/react';

const JOINTS = [
  'hip_l', 'hip_r', 'knee_l', 'knee_r', 'ank_l', 'ank_r',
  'sho_l', 'sho_r', 'elb_l', 'elb_r', 'wri_l', 'wri_r',
];

export default function ProprioViz() {
  const reduce = useReducedMotion();
  return (
    <svg viewBox="0 0 320 180" preserveAspectRatio="none" style={{ width: '100%', height: '100%' }}>
      <rect width="320" height="180" fill="#0a141f" />
      {JOINTS.map((j, i) => {
        const x = 12 + i * 25;
        const baseline = 100;
        const phase = i * 0.38;
        return (
          <g key={j}>
            <line x1={x} y1={baseline} x2={x} y2={baseline - 60} stroke="#1f2a3c" strokeWidth="1" />
            <rect
              x={x - 4}
              y={baseline - 30}
              width="8"
              height="30"
              fill="#7ae0c3"
              rx="1"
              opacity="0.85"
            >
              {!reduce && (
                <>
                  <animate
                    attributeName="height"
                    values="10;55;20;45;10"
                    dur={`${3 + (i % 3)}s`}
                    begin={`${phase}s`}
                    repeatCount="indefinite"
                  />
                  <animate
                    attributeName="y"
                    values="90;45;80;55;90"
                    dur={`${3 + (i % 3)}s`}
                    begin={`${phase}s`}
                    repeatCount="indefinite"
                  />
                </>
              )}
            </rect>
            {/* torque dot */}
            <circle cx={x} cy={baseline + 18} r="3" fill="#b18cff">
              {!reduce && (
                <animate
                  attributeName="r"
                  values="2;4;2"
                  dur={`${2 + (i % 2)}s`}
                  begin={`${phase * 0.7}s`}
                  repeatCount="indefinite"
                />
              )}
            </circle>
            <text
              x={x}
              y={baseline + 38}
              textAnchor="middle"
              fontFamily="var(--font-mono-lewm), monospace"
              fontSize="6.5"
              fill="#8b98a8"
            >
              {j}
            </text>
          </g>
        );
      })}
      <text x="8" y="172" fontFamily="var(--font-mono-lewm), monospace" fontSize="9" fill="#7ae0c3">
        Proprio · 1 kHz · 12 joints → 192d
      </text>
    </svg>
  );
}
