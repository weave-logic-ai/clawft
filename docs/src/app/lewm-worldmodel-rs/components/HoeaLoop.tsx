'use client';

import { useRef } from 'react';
import { motion, useScroll, useTransform, useVelocity, useSpring, useReducedMotion } from 'motion/react';
import { EASE } from './motion';
import { HOEA } from '../copy';

export default function HoeaLoop() {
  const ref = useRef<HTMLElement>(null);
  const reduce = useReducedMotion();

  const { scrollYProgress } = useScroll({
    target: ref,
    offset: ['start start', 'end end'],
  });

  // Scroll velocity modulates the rotation speed once revealed.
  const velocity = useVelocity(scrollYProgress);
  const smooth = useSpring(velocity, { damping: 40, stiffness: 140, mass: 0.8 });
  const spinDur = useTransform(smooth, (v) => {
    if (reduce) return '0s';
    const mag = Math.min(Math.abs(v), 3);
    // Baseline 12 s per revolution; with scroll motion, as fast as 2.4 s.
    return `${12 - mag * 3.2}s`;
  });

  // Arc reveals via strokeDashoffset (dashoffset from 1 → 0).
  const hypOff = useTransform(scrollYProgress, [0.08, 0.22], [1, 0]);
  const obsOff = useTransform(scrollYProgress, [0.15, 0.30], [1, 0]);
  const evalOff = useTransform(scrollYProgress, [0.22, 0.38], [1, 0]);
  const adjOff = useTransform(scrollYProgress, [0.30, 0.48], [1, 0]);

  const loopOpacity = useTransform(scrollYProgress, [0.0, 0.1, 0.9, 1.0], [0, 1, 1, 0.85]);

  return (
    <section
      ref={ref}
      style={{
        position: 'relative',
        height: '200vh',
      }}
    >
      <div
        className="sticky top-0 flex items-center justify-center"
        style={{ height: '100svh', padding: '0 1.5rem' }}
      >
        <motion.div
          style={{ opacity: loopOpacity, width: '100%', maxWidth: '64rem', textAlign: 'center' }}
        >
          <div className="lewm-badge" style={{ marginBottom: '0.6rem' }}>
            {HOEA.eyebrow}
          </div>
          <h2
            className="lewm-editorial"
            style={{
              fontSize: 'clamp(1.8rem, 3.4vw, 2.6rem)',
              margin: '0 0 0.6rem 0',
              letterSpacing: '-0.015em',
            }}
          >
            {HOEA.title}
          </h2>
          <p style={{ color: 'var(--lewm-mute)', maxWidth: '44rem', margin: '0 auto 1.8rem' }}>
            {HOEA.body}
          </p>

          <div style={{ position: 'relative', width: 'min(460px, 80vw)', margin: '0 auto' }}>
            <svg viewBox="0 0 400 400" role="img" aria-label="Real-time H-O-E-A cycle: hypothesize, observe, evaluate, adjust" style={{ width: '100%', height: 'auto' }}>
              <defs>
                <marker id="lewm-loop-arrow" viewBox="0 -5 10 10" refX="8" refY="0" markerWidth="7" markerHeight="7" orient="auto">
                  <path d="M0,-4L10,0L0,4" fill="currentColor" />
                </marker>
              </defs>

              {/* Background circle */}
              <circle cx="200" cy="200" r="140" fill="none" stroke="var(--lewm-line)" strokeWidth="0.75" strokeDasharray="2 4" />

              {/* Rotating group — scroll velocity drives duration */}
              <motion.g
                style={{
                  transformOrigin: '200px 200px',
                  animationName: reduce ? 'none' : 'lewm-spin',
                  animationDuration: spinDur,
                  animationTimingFunction: 'linear',
                  animationIterationCount: 'infinite',
                }}
              >
                <circle cx="200" cy="60" r="3" fill="var(--lewm-cyan)" />
              </motion.g>

              {/* Four arcs — reveal via strokeDashoffset */}
              {/* HYPOTHESIZE: top-right quadrant */}
              <motion.path
                d="M 200 60 A 140 140 0 0 1 340 200"
                fill="none"
                stroke="var(--lewm-cyan)"
                strokeWidth="2"
                pathLength={1}
                style={{ strokeDashoffset: hypOff, strokeDasharray: 1 }}
                color="var(--lewm-cyan)"
                markerEnd="url(#lewm-loop-arrow)"
              />
              {/* OBSERVE: bottom-right */}
              <motion.path
                d="M 340 200 A 140 140 0 0 1 200 340"
                fill="none"
                stroke="var(--lewm-mint)"
                strokeWidth="2"
                pathLength={1}
                style={{ strokeDashoffset: obsOff, strokeDasharray: 1 }}
                color="var(--lewm-mint)"
                markerEnd="url(#lewm-loop-arrow)"
              />
              {/* EVALUATE: bottom-left */}
              <motion.path
                d="M 200 340 A 140 140 0 0 1 60 200"
                fill="none"
                stroke="var(--lewm-violet)"
                strokeWidth="2"
                pathLength={1}
                style={{ strokeDashoffset: evalOff, strokeDasharray: 1 }}
                color="var(--lewm-violet)"
                markerEnd="url(#lewm-loop-arrow)"
              />
              {/* ADJUST: top-left */}
              <motion.path
                d="M 60 200 A 140 140 0 0 1 200 60"
                fill="none"
                stroke="var(--lewm-amber)"
                strokeWidth="2"
                pathLength={1}
                style={{ strokeDashoffset: adjOff, strokeDasharray: 1 }}
                color="var(--lewm-amber)"
                markerEnd="url(#lewm-loop-arrow)"
              />

              {/* Labels */}
              <text x="200" y="40" textAnchor="middle" fontFamily="var(--font-mono-lewm), monospace" fontSize="12" fill="var(--lewm-cyan)">HYPOTHESIZE</text>
              <text x="360" y="205" textAnchor="start" fontFamily="var(--font-mono-lewm), monospace" fontSize="12" fill="var(--lewm-mint)">OBSERVE</text>
              <text x="200" y="368" textAnchor="middle" fontFamily="var(--font-mono-lewm), monospace" fontSize="12" fill="var(--lewm-violet)">EVALUATE</text>
              <text x="40" y="205" textAnchor="end" fontFamily="var(--font-mono-lewm), monospace" fontSize="12" fill="var(--lewm-amber)">ADJUST</text>

              <text x="200" y="200" textAnchor="middle" fontFamily="var(--font-mono-lewm), monospace" fontSize="10" fill="var(--lewm-mute)">ECC · continuous</text>
              <text x="200" y="216" textAnchor="middle" fontFamily="var(--font-mono-lewm), monospace" fontSize="10" fill="var(--lewm-mute)">never batched</text>
            </svg>

            <style>{`@keyframes lewm-spin { to { transform: rotate(360deg); } }`}</style>
          </div>

          <ul
            className="lewm-mono"
            style={{
              listStyle: 'none',
              padding: 0,
              margin: '2rem auto 0',
              display: 'grid',
              gap: '0.4rem',
              maxWidth: '40rem',
              color: 'var(--lewm-mute)',
              fontSize: '0.85rem',
              textAlign: 'left',
            }}
          >
            {HOEA.states.map((s) => (
              <li key={s.label}>
                <span style={{ color: 'var(--lewm-ink)' }}>{s.label}</span>
                {' — '}
                {s.detail}
              </li>
            ))}
          </ul>
        </motion.div>
      </div>
    </section>
  );
}
