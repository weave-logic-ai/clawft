'use client';

import { useRef, type ReactNode } from 'react';
import { motion, useScroll, useTransform, useReducedMotion } from 'motion/react';
import { EASE } from './motion';

type Props = {
  id: string;
  number: string;
  title: string;
  tagline: string;
  adrs: readonly string[];
  body: readonly string[];
  accent?: 'cyan' | 'mint' | 'violet' | 'amber';
  children?: ReactNode;
};

const accentVar = (a: Props['accent'] = 'cyan') => `var(--lewm-${a})`;

export default function PanelPopup({
  id,
  number,
  title,
  tagline,
  adrs,
  body,
  accent = 'cyan',
  children,
}: Props) {
  const ref = useRef<HTMLElement>(null);
  const reduce = useReducedMotion();

  const { scrollYProgress } = useScroll({
    target: ref,
    offset: ['start 0.85', 'center 0.45'],
  });

  const scale = useTransform(
    scrollYProgress,
    [0, 1],
    reduce ? [1, 1] : [0.92, 1.0],
    { ease: EASE },
  );
  const y = useTransform(scrollYProgress, [0, 1], reduce ? [0, 0] : [40, 0]);
  const opacity = useTransform(scrollYProgress, [0, 0.4], [0, 1]);
  const clip = useTransform(
    scrollYProgress,
    [0, 1],
    reduce
      ? ['inset(0% 0% 0% 0% round 8px)', 'inset(0% 0% 0% 0% round 8px)']
      : [
          'inset(42% 8% 42% 8% round 8px)',
          'inset(0% 0% 0% 0% round 8px)',
        ],
  );

  return (
    <section
      ref={ref}
      id={id}
      style={{
        padding: '8vh 1.5rem',
        display: 'flex',
        justifyContent: 'center',
      }}
    >
      <motion.article
        className="lewm-panel"
        style={{
          scale,
          y,
          opacity,
          clipPath: clip,
          willChange: 'transform, opacity, clip-path',
          width: '100%',
          maxWidth: '60rem',
          padding: '2rem',
          borderColor: accentVar(accent),
        }}
      >
        <header style={{ display: 'flex', alignItems: 'baseline', gap: '1rem', flexWrap: 'wrap' }}>
          <span
            className="lewm-mono"
            style={{
              fontSize: '1.3rem',
              color: accentVar(accent),
            }}
          >
            {number}
          </span>
          <h3
            style={{
              margin: 0,
              fontSize: 'clamp(1.5rem, 2.5vw, 2rem)',
              letterSpacing: '-0.01em',
            }}
          >
            {title}
          </h3>
          <span
            className="lewm-mono"
            style={{
              fontSize: '0.78rem',
              color: 'var(--lewm-mute)',
              marginLeft: 'auto',
            }}
          >
            {tagline}
          </span>
        </header>

        <div style={{ marginTop: '1.2rem', display: 'grid', gap: '0.8rem' }}>
          {body.map((p, i) => (
            <p
              key={i}
              style={{
                margin: 0,
                fontSize: '1rem',
                lineHeight: 1.6,
                color: 'var(--lewm-ink)',
              }}
            >
              {p}
            </p>
          ))}
        </div>

        {children && (
          <div style={{ marginTop: '1.5rem' }}>{children}</div>
        )}

        <footer
          style={{
            marginTop: '1.5rem',
            display: 'flex',
            gap: '0.5rem',
            flexWrap: 'wrap',
          }}
        >
          {adrs.map((a) => (
            <span key={a} className="lewm-badge" style={{ borderColor: accentVar(accent), color: accentVar(accent) }}>
              {a}
            </span>
          ))}
        </footer>
      </motion.article>
    </section>
  );
}
