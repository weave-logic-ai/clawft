'use client';

/**
 * SurpriseTimeline — VoE surprise + SIGReg health timeline that draws
 * itself in as the user scrolls. Annotates the four-condition AND rollback.
 */

import { useRef } from 'react';
import { motion, useScroll, useTransform, useReducedMotion } from 'motion/react';
import { EASE } from './motion';

export default function SurpriseTimeline() {
  const ref = useRef<HTMLElement>(null);
  const reduce = useReducedMotion();
  const { scrollYProgress } = useScroll({
    target: ref,
    offset: ['start 0.9', 'end 0.4'],
  });
  // Stroke offset for trace draw-in.
  const surpriseLen = useTransform(scrollYProgress, [0, 0.7], reduce ? [0, 0] : [1, 0]);
  const healthLen = useTransform(scrollYProgress, [0.1, 0.85], reduce ? [0, 0] : [1, 0]);
  const opCallout = useTransform(scrollYProgress, [0.6, 0.9], [0, 1]);

  return (
    <section
      ref={ref}
      style={{ padding: '8vh 1.5rem' }}
      aria-label="Surprise + SIGReg health timeline with rollback annotation"
    >
      <div style={{ maxWidth: '78rem', margin: '0 auto' }}>
        <header style={{ textAlign: 'center', marginBottom: '1.4rem' }}>
          <span className="lewm-badge" style={{ borderColor: 'var(--lewm-amber)', color: 'var(--lewm-amber)' }}>
            Diagram ⑥ · ADR-055 · streaming-merge guardrail
          </span>
          <h2
            className="lewm-editorial"
            style={{ marginTop: '0.7rem', fontSize: 'clamp(1.5rem, 2.6vw, 2rem)', letterSpacing: '-0.015em' }}
          >
            Four-condition AND rollback, in one trace.
          </h2>
          <p style={{ color: 'var(--lewm-mute)', maxWidth: '56rem', margin: '0.5rem auto 0' }}>
            SIGReg health · probing accuracy · VoE surprise differentiation · temporal-straightening
            score. Any single dip can be tolerated; the conjunction is what triggers an automatic
            rollback to the last attested checkpoint.
          </p>
        </header>

        <div
          className="lewm-panel"
          style={{ padding: '1.2rem', borderColor: 'var(--lewm-amber)', position: 'relative' }}
        >
          <svg viewBox="0 0 1100 280" style={{ width: '100%', height: 'auto', display: 'block' }}>
            <defs>
              <linearGradient id="st-fill-amber" x1="0" x2="0" y1="0" y2="1">
                <stop offset="0%" stopColor="#ffb65a" stopOpacity="0.35" />
                <stop offset="100%" stopColor="#ffb65a" stopOpacity="0" />
              </linearGradient>
            </defs>
            {/* Time axis */}
            <line x1="40" y1="240" x2="1080" y2="240" stroke="#3a4a60" strokeWidth="0.8" />
            {Array.from({ length: 11 }).map((_, i) => (
              <g key={i}>
                <line
                  x1={40 + i * 104}
                  y1="240"
                  x2={40 + i * 104}
                  y2="246"
                  stroke="#3a4a60"
                  strokeWidth="0.8"
                />
                <text
                  x={40 + i * 104}
                  y="262"
                  textAnchor="middle"
                  fontFamily="var(--font-mono-lewm), monospace"
                  fontSize="8"
                  fill="#8b98a8"
                >
                  t+{i * 3}s
                </text>
              </g>
            ))}
            {/* Health threshold line */}
            <line
              x1="40"
              y1="100"
              x2="1080"
              y2="100"
              stroke="#7ae0c3"
              strokeWidth="0.6"
              strokeDasharray="2 4"
              opacity="0.6"
            />
            <text
              x="48"
              y="96"
              fontFamily="var(--font-mono-lewm), monospace"
              fontSize="8"
              fill="#7ae0c3"
            >
              sigreg_health 0.85
            </text>

            {/* Surprise area */}
            <motion.path
              d={surprisePath()}
              fill="url(#st-fill-amber)"
              stroke="none"
              style={{
                pathLength: 1,
                strokeDasharray: 1,
                strokeDashoffset: surpriseLen,
                opacity: 0.9,
              }}
            />
            {/* Surprise stroke */}
            <motion.path
              d={surprisePath()}
              fill="none"
              stroke="#ffb65a"
              strokeWidth="1.6"
              pathLength={1}
              style={{ strokeDasharray: 1, strokeDashoffset: surpriseLen }}
            />
            {/* Health stroke */}
            <motion.path
              d={healthPath()}
              fill="none"
              stroke="#7ae0c3"
              strokeWidth="1.6"
              pathLength={1}
              style={{ strokeDasharray: 1, strokeDashoffset: healthLen }}
            />

            {/* Rollback marker */}
            <motion.g style={{ opacity: opCallout }}>
              <line x1="780" y1="40" x2="780" y2="240" stroke="#ffb65a" strokeWidth="1.2" strokeDasharray="3 3" />
              <rect x="708" y="20" width="144" height="22" rx="3" fill="#1f1408" stroke="#ffb65a" />
              <text
                x="780"
                y="35"
                textAnchor="middle"
                fontFamily="var(--font-mono-lewm), monospace"
                fontSize="10"
                fill="#ffb65a"
              >
                ROLLBACK · t+22s
              </text>
            </motion.g>

            {/* Legend */}
            <g>
              <rect x="900" y="180" width="180" height="44" rx="4" fill="#0e1420" stroke="#3a4a60" />
              <line x1="912" y1="194" x2="932" y2="194" stroke="#ffb65a" strokeWidth="1.6" />
              <text x="938" y="197" fontFamily="var(--font-mono-lewm), monospace" fontSize="9" fill="#ffb65a">VoE surprise</text>
              <line x1="912" y1="212" x2="932" y2="212" stroke="#7ae0c3" strokeWidth="1.6" />
              <text x="938" y="215" fontFamily="var(--font-mono-lewm), monospace" fontSize="9" fill="#7ae0c3">SIGReg health</text>
            </g>
          </svg>
        </div>
      </div>
    </section>
  );
}

function surprisePath(): string {
  // Build a noisy curve that spikes around t≈18s.
  const samples = 80;
  const pts: string[] = [];
  for (let i = 0; i < samples; i++) {
    const x = 40 + (i / (samples - 1)) * 1040;
    const t = i / (samples - 1);
    const base =
      170 -
      Math.sin(t * Math.PI * 4) * 8 -
      Math.sin(t * Math.PI * 13) * 3 -
      // spike
      Math.exp(-Math.pow((t - 0.66) / 0.05, 2)) * 90 -
      Math.exp(-Math.pow((t - 0.74) / 0.04, 2)) * 60;
    pts.push(`${i === 0 ? 'M' : 'L'} ${x} ${base}`);
  }
  return pts.join(' ');
}

function healthPath(): string {
  const samples = 80;
  const pts: string[] = [];
  for (let i = 0; i < samples; i++) {
    const x = 40 + (i / (samples - 1)) * 1040;
    const t = i / (samples - 1);
    const base =
      70 +
      Math.sin(t * Math.PI * 3) * 4 +
      Math.exp(-Math.pow((t - 0.7) / 0.08, 2)) * 60; // dip below threshold
    pts.push(`${i === 0 ? 'M' : 'L'} ${x} ${base}`);
  }
  return pts.join(' ');
}
