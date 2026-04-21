'use client';

import { useRef } from 'react';
import { motion, useScroll, useTransform, useReducedMotion } from 'motion/react';
import { TLDR } from '../copy';

export default function Pullquote() {
  const ref = useRef<HTMLElement>(null);
  const reduce = useReducedMotion();

  const { scrollYProgress } = useScroll({
    target: ref,
    offset: ['start end', 'end start'],
  });

  const opacity = useTransform(
    scrollYProgress,
    [0.1, 0.3, 0.6, 0.8],
    reduce ? [1, 1, 1, 1] : [0, 1, 1, 0],
  );
  const y = useTransform(
    scrollYProgress,
    [0.1, 0.3],
    reduce ? ['0px', '0px'] : ['24px', '0px'],
  );

  return (
    <section
      ref={ref}
      className="relative flex items-center justify-center"
      style={{ height: '80vh', padding: '0 2rem' }}
    >
      <motion.div
        style={{ opacity, y, maxWidth: '52rem', textAlign: 'center' }}
      >
        <p
          className="lewm-editorial"
          style={{
            fontSize: 'clamp(1.4rem, 2.5vw, 2rem)',
            lineHeight: 1.3,
            color: 'var(--lewm-ink)',
            margin: 0,
          }}
        >
          “{TLDR.prequote}”
        </p>
        <p
          className="lewm-mono"
          style={{
            marginTop: '1.2rem',
            fontSize: '0.82rem',
            color: 'var(--lewm-mute)',
          }}
        >
          {TLDR.attribution}
        </p>
        <p
          style={{
            marginTop: '2rem',
            color: 'var(--lewm-mute)',
            fontSize: '1rem',
            lineHeight: 1.6,
          }}
        >
          {TLDR.body}
        </p>
      </motion.div>
    </section>
  );
}
