'use client';

import { useRef } from 'react';
import { motion, useScroll, useTransform, useReducedMotion } from 'motion/react';
import { EASE } from './motion';
import { ADRS } from '../copy';

export default function AdrIndex() {
  const ref = useRef<HTMLElement>(null);
  const reduce = useReducedMotion();

  const { scrollYProgress } = useScroll({
    target: ref,
    offset: ['start end', 'end start'],
  });

  const titleOpacity = useTransform(scrollYProgress, [0.0, 0.15], [0, 1]);

  return (
    <section
      ref={ref}
      id="adr-index"
      style={{ padding: '12vh 1.5rem 18vh' }}
    >
      <motion.header
        style={{
          opacity: titleOpacity,
          textAlign: 'center',
          maxWidth: '48rem',
          margin: '0 auto 3rem',
        }}
      >
        <div className="lewm-badge">eleven decisions · one spine</div>
        <h2
          className="lewm-editorial"
          style={{
            fontSize: 'clamp(1.8rem, 3vw, 2.4rem)',
            marginTop: '0.8rem',
            letterSpacing: '-0.015em',
          }}
        >
          ADR-048 → ADR-058
        </h2>
      </motion.header>

      <ol
        style={{
          listStyle: 'none',
          padding: 0,
          margin: '0 auto',
          maxWidth: '56rem',
          display: 'grid',
          gap: '0.8rem',
        }}
      >
        {ADRS.map((a, i) => (
          <AdrRow key={a.n} adr={a} index={i} reduce={!!reduce} />
        ))}
      </ol>
    </section>
  );
}

type Adr = (typeof ADRS)[number];
type WithHero = Adr & { hero?: boolean };

function AdrRow({ adr, index, reduce }: { adr: Adr; index: number; reduce: boolean }) {
  const ref = useRef<HTMLLIElement>(null);
  const { scrollYProgress } = useScroll({
    target: ref,
    offset: ['start 0.9', 'center 0.55'],
  });
  const y = useTransform(scrollYProgress, [0, 1], reduce ? [0, 0] : [24, 0]);
  const opacity = useTransform(scrollYProgress, [0, 0.6], [0, 1]);
  const withHero = adr as WithHero;
  const isHero = !!withHero.hero;

  return (
    <motion.li
      ref={ref}
      style={{
        y,
        opacity,
        willChange: 'transform, opacity',
      }}
    >
      <article
        className={`lewm-panel ${isHero ? 'lewm-amber-glow' : ''}`}
        style={{
          padding: '1.2rem 1.4rem',
          borderColor: isHero ? 'var(--lewm-amber)' : 'var(--lewm-line)',
          display: 'grid',
          gridTemplateColumns: 'auto 1fr auto',
          gap: '1.1rem',
          alignItems: 'center',
        }}
      >
        <span
          className="lewm-mono"
          style={{
            fontSize: '0.95rem',
            color: isHero ? 'var(--lewm-amber)' : 'var(--lewm-cyan)',
            minWidth: '3.2rem',
          }}
        >
          ADR-{adr.n}
        </span>
        <div>
          <h3
            style={{
              margin: 0,
              fontSize: '1.05rem',
              color: 'var(--lewm-ink)',
              letterSpacing: '-0.01em',
            }}
          >
            {adr.title}
          </h3>
          <p
            style={{
              margin: '0.3rem 0 0',
              color: 'var(--lewm-mute)',
              fontSize: '0.88rem',
            }}
          >
            {adr.cite}
          </p>
        </div>
        <span
          className="lewm-mono"
          style={{
            fontSize: '0.72rem',
            color: 'var(--lewm-mute)',
          }}
        >
          {String(index + 1).padStart(2, '0')} / 11
        </span>
      </article>
    </motion.li>
  );
}
