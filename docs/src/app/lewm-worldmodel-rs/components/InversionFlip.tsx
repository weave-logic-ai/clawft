'use client';

import { useRef } from 'react';
import { motion, useScroll, useTransform, useReducedMotion } from 'motion/react';
import { EASE } from './motion';
import { INVERSION } from '../copy';

export default function InversionFlip() {
  const ref = useRef<HTMLElement>(null);
  const reduce = useReducedMotion();

  const { scrollYProgress } = useScroll({
    target: ref,
    offset: ['start start', 'end end'],
  });

  const rotate = useTransform(
    scrollYProgress,
    [0.15, 0.75],
    reduce ? [0, 0] : [0, 180],
    { ease: EASE },
  );
  const beforeOpacity = useTransform(scrollYProgress, [0.35, 0.5], [1, 0]);
  const afterOpacity = useTransform(scrollYProgress, [0.5, 0.65], [0, 1]);

  return (
    <section
      ref={ref}
      className="relative"
      style={{ height: '160vh' }}
    >
      <div
        className="sticky top-0 flex items-center justify-center"
        style={{ height: '100svh', padding: '0 2rem' }}
      >
        <div style={{ maxWidth: '72rem', width: '100%' }}>
          <header style={{ textAlign: 'center', marginBottom: '2.2rem' }}>
            <div
              className="lewm-badge"
              style={{
                color: 'var(--lewm-amber)',
                borderColor: 'var(--lewm-amber)',
              }}
            >
              {INVERSION.eyebrow}
            </div>
            <h2
              className="lewm-editorial"
              style={{
                fontSize: 'clamp(2rem, 4vw, 3.2rem)',
                marginTop: '0.8rem',
                letterSpacing: '-0.02em',
              }}
            >
              {INVERSION.title}
            </h2>
          </header>

          <div
            style={{
              perspective: '1800px',
              display: 'flex',
              justifyContent: 'center',
            }}
          >
            <motion.div
              style={{
                rotateY: rotate,
                transformStyle: 'preserve-3d',
                position: 'relative',
                width: 'min(640px, 86vw)',
                height: '360px',
              }}
            >
              {/* Before face */}
              <motion.div
                className="lewm-panel"
                style={{
                  opacity: beforeOpacity,
                  backfaceVisibility: 'hidden',
                  position: 'absolute',
                  inset: 0,
                  padding: '2rem',
                  display: 'flex',
                  flexDirection: 'column',
                  justifyContent: 'center',
                  borderColor: 'var(--lewm-line)',
                }}
              >
                <div className="lewm-mono" style={{ color: 'var(--lewm-mute)', fontSize: '0.75rem' }}>
                  {INVERSION.before.label}
                </div>
                <p style={{ marginTop: '0.8rem', fontSize: '1.1rem', lineHeight: 1.5 }}>
                  {INVERSION.before.body}
                </p>
              </motion.div>

              {/* After face */}
              <motion.div
                className="lewm-panel lewm-amber-glow"
                style={{
                  opacity: afterOpacity,
                  backfaceVisibility: 'hidden',
                  transform: 'rotateY(180deg)',
                  position: 'absolute',
                  inset: 0,
                  padding: '2rem',
                  display: 'flex',
                  flexDirection: 'column',
                  justifyContent: 'center',
                  borderColor: 'var(--lewm-amber)',
                }}
              >
                <div
                  className="lewm-mono"
                  style={{ color: 'var(--lewm-amber)', fontSize: '0.75rem' }}
                >
                  {INVERSION.after.label}
                </div>
                <p style={{ marginTop: '0.8rem', fontSize: '1.1rem', lineHeight: 1.5 }}>
                  {INVERSION.after.body}
                </p>
              </motion.div>
            </motion.div>
          </div>

          <ol
            style={{
              marginTop: '2.5rem',
              padding: 0,
              listStyle: 'none',
              display: 'grid',
              gap: '0.6rem',
              maxWidth: '60rem',
              marginInline: 'auto',
            }}
          >
            {INVERSION.rules.map((rule, i) => (
              <li
                key={i}
                className="lewm-mono"
                style={{
                  display: 'flex',
                  gap: '0.8rem',
                  fontSize: '0.88rem',
                  color: 'var(--lewm-mute)',
                  paddingLeft: '0.4rem',
                }}
              >
                <span style={{ color: 'var(--lewm-amber)', minWidth: '1.5rem' }}>
                  {String(i + 1).padStart(2, '0')}
                </span>
                <span>{rule}</span>
              </li>
            ))}
          </ol>
        </div>
      </div>
    </section>
  );
}
