'use client';

/* Audio viz — log-mel spectrogram bars + waveform. */

import { useReducedMotion } from 'motion/react';

export default function AudioViz() {
  const reduce = useReducedMotion();
  const bars = 48;
  return (
    <svg viewBox="0 0 320 180" preserveAspectRatio="none" style={{ width: '100%', height: '100%' }}>
      <rect width="320" height="180" fill="#0a141f" />
      {/* spectrogram-ish bars */}
      {Array.from({ length: bars }, (_, i) => {
        const x = 8 + i * 6.4;
        const baseH = 6 + ((i * 17) % 50);
        const phase = i * 0.18;
        return (
          <rect
            key={i}
            x={x}
            y={120 - baseH}
            width="4"
            height={baseH}
            fill="#ffb65a"
            opacity="0.8"
            rx="1"
          >
            {!reduce && (
              <>
                <animate
                  attributeName="height"
                  values={`${baseH * 0.5};${baseH * 1.6};${baseH * 0.7};${baseH * 1.2};${baseH * 0.5}`}
                  dur={`${1.6 + (i % 3) * 0.3}s`}
                  begin={`${phase}s`}
                  repeatCount="indefinite"
                />
                <animate
                  attributeName="y"
                  values={`${120 - baseH * 0.5};${120 - baseH * 1.6};${120 - baseH * 0.7};${120 - baseH * 1.2};${120 - baseH * 0.5}`}
                  dur={`${1.6 + (i % 3) * 0.3}s`}
                  begin={`${phase}s`}
                  repeatCount="indefinite"
                />
              </>
            )}
          </rect>
        );
      })}
      {/* waveform */}
      <path
        d={`M 0 145 ${Array.from({ length: 80 }, (_, i) => {
          const x = (i / 79) * 320;
          const y = 145 + Math.sin(i * 0.3) * 8 + Math.sin(i * 0.7) * 4;
          return `L ${x} ${y}`;
        }).join(' ')}`}
        fill="none"
        stroke="#ffb65a"
        strokeWidth="1.2"
        opacity="0.9"
      >
        {!reduce && (
          <animate
            attributeName="d"
            values={`M 0 145 ${Array.from({ length: 80 }, (_, i) => {
              const x = (i / 79) * 320;
              const y = 145 + Math.sin(i * 0.3) * 8 + Math.sin(i * 0.7) * 4;
              return `L ${x} ${y}`;
            }).join(' ')};M 0 145 ${Array.from({ length: 80 }, (_, i) => {
              const x = (i / 79) * 320;
              const y = 145 + Math.sin(i * 0.3 + 1.2) * 12 + Math.sin(i * 0.6) * 5;
              return `L ${x} ${y}`;
            }).join(' ')};M 0 145 ${Array.from({ length: 80 }, (_, i) => {
              const x = (i / 79) * 320;
              const y = 145 + Math.sin(i * 0.3) * 8 + Math.sin(i * 0.7) * 4;
              return `L ${x} ${y}`;
            }).join(' ')}`}
            dur="3.2s"
            repeatCount="indefinite"
          />
        )}
      </path>
      <text x="8" y="172" fontFamily="var(--font-mono-lewm), monospace" fontSize="9" fill="#ffb65a">
        Audio · 16 kHz · 48-mel → 192d
      </text>
    </svg>
  );
}
