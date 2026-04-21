'use client';

import { useRef, type ReactNode } from 'react';
import { motion, useScroll, useTransform, useReducedMotion } from 'motion/react';
import { EASE } from './motion';

export default function HeroZoomFrame({ children }: { children: ReactNode }) {
  const ref = useRef<HTMLElement>(null);
  const reduce = useReducedMotion();

  const { scrollYProgress } = useScroll({
    target: ref,
    offset: ['start start', 'end start'],
  });

  const scale = useTransform(
    scrollYProgress,
    [0.0, 0.28, 0.35],
    reduce ? [1, 1, 1] : [1.0, 1.5, 1.6],
    { ease: EASE },
  );
  const y = useTransform(
    scrollYProgress,
    [0.0, 0.35],
    reduce ? ['0vh', '0vh'] : ['0vh', '-12vh'],
  );
  const opacity = useTransform(scrollYProgress, [0.25, 0.45], [1, 0]);

  return (
    <section ref={ref} className="relative" style={{ height: '180vh' }}>
      <div
        className="sticky top-0 flex items-center justify-center"
        style={{ height: '100svh' }}
      >
        <motion.div
          style={{
            scale,
            y,
            opacity,
            willChange: 'transform, opacity',
            width: '100%',
            maxWidth: '78rem',
            padding: '0 2rem',
          }}
        >
          {children}
        </motion.div>
      </div>
    </section>
  );
}
