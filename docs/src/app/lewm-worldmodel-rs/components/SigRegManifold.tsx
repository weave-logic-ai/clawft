'use client';

/**
 * SigRegManifold — animated 2-D projection of the SIGReg latent.
 *
 * 256 z_t samples drift around the unit Gaussian, with a periodic
 * "drift event" that pulls them off-center until SIGReg pulls them back.
 * The σ ring (1-σ contour) breathes with the sample variance.
 */

import { useReducedMotion } from 'motion/react';

export default function SigRegManifold() {
  const reduce = useReducedMotion();
  // 192 dots arranged in a roughly Gaussian distribution with deterministic seeds.
  const dots = Array.from({ length: 192 }, (_, i) => {
    const t = (i / 192) * Math.PI * 2;
    const r = Math.sqrt(-2 * Math.log(0.5 + ((i * 17) % 100) / 200)) * 28;
    const a = t * 5.2 + ((i * 11) % 31);
    return {
      cx: 200 + Math.cos(a) * r,
      cy: 200 + Math.sin(a) * r,
      d: ((i * 7) % 9) * 0.4,
      i,
    };
  });

  return (
    <svg viewBox="0 0 400 400" role="img" aria-label="SIGReg latent manifold: z_t samples around isotropic Gaussian" style={{ width: '100%', height: 'auto' }}>
      <defs>
        <radialGradient id="sigreg-bg" cx="0.5" cy="0.5" r="0.5">
          <stop offset="0%" stopColor="#142035" stopOpacity="0.7" />
          <stop offset="100%" stopColor="#0a0f1a" stopOpacity="0.95" />
        </radialGradient>
      </defs>
      <rect width="400" height="400" fill="url(#sigreg-bg)" />
      {/* Axes */}
      <line x1="40" y1="200" x2="360" y2="200" stroke="#1f2a3c" strokeDasharray="3 4" />
      <line x1="200" y1="40" x2="200" y2="360" stroke="#1f2a3c" strokeDasharray="3 4" />
      {/* σ rings */}
      {[40, 80, 120].map((r, i) => (
        <circle key={r} cx="200" cy="200" r={r} fill="none" stroke="#3a4a60" strokeWidth="0.8" strokeDasharray="2 5" opacity={0.7 - i * 0.2}>
          {!reduce && (
            <animate
              attributeName="r"
              values={`${r};${r + 4};${r}`}
              dur={`${4 + i}s`}
              repeatCount="indefinite"
            />
          )}
        </circle>
      ))}
      {/* z_t dots */}
      {dots.map((d) => (
        <circle key={d.i} cx={d.cx} cy={d.cy} r="1.4" fill="#6dd3ff" opacity="0.8">
          {!reduce && (
            <>
              <animate
                attributeName="cx"
                values={`${d.cx};${d.cx + Math.cos(d.i) * 6};${d.cx - Math.sin(d.i) * 5};${d.cx}`}
                dur={`${5 + (d.i % 4)}s`}
                begin={`${d.d}s`}
                repeatCount="indefinite"
              />
              <animate
                attributeName="cy"
                values={`${d.cy};${d.cy + Math.sin(d.i) * 6};${d.cy + Math.cos(d.i) * 5};${d.cy}`}
                dur={`${5 + (d.i % 4)}s`}
                begin={`${d.d}s`}
                repeatCount="indefinite"
              />
              <animate
                attributeName="opacity"
                values="0.4;1;0.5;0.8"
                dur={`${5 + (d.i % 4)}s`}
                begin={`${d.d}s`}
                repeatCount="indefinite"
              />
            </>
          )}
        </circle>
      ))}
      {/* SIGReg health badge */}
      <g>
        <rect x="20" y="20" width="120" height="36" rx="4" fill="#0e1420" stroke="#3a4a60" />
        <text x="32" y="34" fontFamily="var(--font-mono-lewm), monospace" fontSize="8" fill="#8b98a8">SIGREG_HEALTH</text>
        <text x="32" y="48" fontFamily="var(--font-mono-lewm), monospace" fontSize="11" fill="#7ae0c3">0.94 ✓</text>
      </g>
      <g>
        <rect x="260" y="20" width="120" height="36" rx="4" fill="#0e1420" stroke="#3a4a60" />
        <text x="272" y="34" fontFamily="var(--font-mono-lewm), monospace" fontSize="8" fill="#8b98a8">VARIANCE σ²</text>
        <text x="272" y="48" fontFamily="var(--font-mono-lewm), monospace" fontSize="11" fill="#6dd3ff">0.998</text>
      </g>
      <text x="200" y="380" textAnchor="middle" fontFamily="var(--font-mono-lewm), monospace" fontSize="10" fill="#8b98a8">
        z_t ∈ ℝ¹⁹² · regularized to N(0, I) · ADR-050
      </text>
    </svg>
  );
}
