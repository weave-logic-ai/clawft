'use client';

import { useRef } from 'react';
import { motion, useScroll, useTransform, useReducedMotion } from 'motion/react';
import { EASE } from './motion';
import SystemPanelSvg from './SystemPanelSvg';

export default function CrossViewDissolve() {
  const ref = useRef<HTMLElement>(null);
  const reduce = useReducedMotion();

  const { scrollYProgress } = useScroll({
    target: ref,
    offset: ['start start', 'end end'],
  });

  const panelOpacity = useTransform(scrollYProgress, [0.0, 0.4, 0.6], [1, 1, 0]);
  const panelScale = useTransform(
    scrollYProgress,
    [0.4, 0.8],
    reduce ? [1, 1] : [1.0, 0.72],
    { ease: EASE },
  );
  const panelRotate = useTransform(
    scrollYProgress,
    [0.4, 0.8],
    reduce ? [0, 0] : [0, 3],
  );
  const tagOpacity = useTransform(scrollYProgress, [0.3, 0.5], [0, 1]);

  return (
    <section
      ref={ref}
      style={{ position: 'relative', height: '180vh' }}
    >
      <div
        className="sticky top-0 flex items-center justify-center"
        style={{ height: '100svh', padding: '0 1.5rem' }}
      >
        <motion.div
          style={{
            opacity: panelOpacity,
            scale: panelScale,
            rotate: panelRotate,
            willChange: 'transform, opacity',
            maxWidth: '76rem',
            width: '100%',
          }}
        >
          <SystemPanelSvg compact />
        </motion.div>
        <motion.div
          style={{
            opacity: tagOpacity,
            position: 'absolute',
            bottom: '8vh',
            left: 0,
            right: 0,
            textAlign: 'center',
          }}
        >
          <p className="lewm-mono" style={{ color: 'var(--lewm-mute)', fontSize: '0.9rem' }}>
            the architecture · now · the cycle it thinks through ↓
          </p>
        </motion.div>
      </div>
    </section>
  );
}
