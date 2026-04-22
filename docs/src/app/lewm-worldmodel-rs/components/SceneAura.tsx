'use client';

/**
 * SceneAura — atmospheric, parallax background painted per-scene.
 *
 * Strategy: pure SVG/CSS gradients + procedural noise + drifting blobs.
 * No external image assets. Each scene gets its own palette + figure
 * so the page reads as a sequence of "frames" with distinct backgrounds.
 *
 * Use as a sticky/fixed underlay inside a scrollable section. The component
 * fades and parallaxes its own contents based on the parent scroll progress.
 */

import { useRef, type ReactNode } from 'react';
import { motion, useScroll, useTransform, useReducedMotion } from 'motion/react';

type Variant = 'cyan' | 'mint' | 'violet' | 'amber' | 'mixed' | 'deep';

const PALETTES: Record<Variant, { primary: string; secondary: string; tertiary: string }> = {
  cyan:   { primary: '#0d3a52', secondary: '#0a2740', tertiary: '#06121f' },
  mint:   { primary: '#0d3a32', secondary: '#0a2723', tertiary: '#061211' },
  violet: { primary: '#251a4a', secondary: '#170f30', tertiary: '#0a081a' },
  amber:  { primary: '#3a2607', secondary: '#1f1408', tertiary: '#100a04' },
  mixed:  { primary: '#102036', secondary: '#1a1a2e', tertiary: '#080814' },
  deep:   { primary: '#070b14', secondary: '#0a0f1a', tertiary: '#050810' },
};

type Props = {
  variant?: Variant;
  parallax?: boolean;
  showGrid?: boolean;
  showStars?: boolean;
  showFlow?: boolean;
  scrollTarget?: React.RefObject<HTMLElement | null>;
  children?: ReactNode;
};

export default function SceneAura({
  variant = 'deep',
  parallax = true,
  showGrid = true,
  showStars = true,
  showFlow = false,
  scrollTarget,
  children,
}: Props) {
  const localRef = useRef<HTMLDivElement>(null);
  const reduce = useReducedMotion();
  const palette = PALETTES[variant];

  const { scrollYProgress } = useScroll({
    target: scrollTarget ?? localRef,
    offset: ['start end', 'end start'],
  });

  const yA = useTransform(
    scrollYProgress,
    [0, 1],
    reduce || !parallax ? ['0%', '0%'] : ['-8%', '8%'],
  );
  const yB = useTransform(
    scrollYProgress,
    [0, 1],
    reduce || !parallax ? ['0%', '0%'] : ['8%', '-12%'],
  );
  const opacity = useTransform(scrollYProgress, [0, 0.15, 0.85, 1], [0, 1, 1, 0.6]);

  return (
    <div
      ref={localRef}
      aria-hidden="true"
      style={{
        position: 'absolute',
        inset: 0,
        overflow: 'hidden',
        pointerEvents: 'none',
        zIndex: 0,
      }}
    >
      {/* Base gradient wash */}
      <div
        style={{
          position: 'absolute',
          inset: '-10%',
          background: `radial-gradient(ellipse 80% 60% at 30% 30%, ${palette.primary} 0%, transparent 55%),
                       radial-gradient(ellipse 90% 70% at 75% 70%, ${palette.secondary} 0%, transparent 60%),
                       linear-gradient(180deg, ${palette.tertiary} 0%, ${palette.tertiary} 100%)`,
          filter: 'saturate(1.1)',
        }}
      />

      {/* Drifting nebula blobs */}
      <motion.div style={{ position: 'absolute', inset: 0, y: yA, opacity }}>
        <div
          style={{
            position: 'absolute',
            top: '10%',
            left: '12%',
            width: '46vw',
            height: '46vw',
            background: `radial-gradient(circle at 30% 30%, ${palette.primary}aa 0%, transparent 65%)`,
            filter: 'blur(60px)',
            mixBlendMode: 'screen',
          }}
        />
      </motion.div>
      <motion.div style={{ position: 'absolute', inset: 0, y: yB, opacity }}>
        <div
          style={{
            position: 'absolute',
            bottom: '5%',
            right: '8%',
            width: '52vw',
            height: '52vw',
            background: `radial-gradient(circle at 60% 60%, ${palette.secondary}cc 0%, transparent 70%)`,
            filter: 'blur(80px)',
            mixBlendMode: 'screen',
          }}
        />
      </motion.div>

      {/* Procedural grain — tiny SVG noise tile, repeated */}
      <svg
        style={{
          position: 'absolute',
          inset: 0,
          width: '100%',
          height: '100%',
          opacity: 0.18,
          mixBlendMode: 'overlay',
        }}
      >
        <filter id="lewm-noise">
          <feTurbulence type="fractalNoise" baseFrequency="0.95" numOctaves="2" stitchTiles="stitch" />
          <feColorMatrix type="saturate" values="0" />
        </filter>
        <rect width="100%" height="100%" filter="url(#lewm-noise)" />
      </svg>

      {/* Optional engineering grid */}
      {showGrid && (
        <div
          style={{
            position: 'absolute',
            inset: 0,
            backgroundImage:
              'linear-gradient(rgba(110, 160, 220, 0.08) 1px, transparent 1px),' +
              'linear-gradient(90deg, rgba(110, 160, 220, 0.08) 1px, transparent 1px)',
            backgroundSize: '64px 64px',
            backgroundPosition: '-1px -1px',
            maskImage: 'radial-gradient(ellipse at center, rgba(0,0,0,1) 0%, rgba(0,0,0,0) 80%)',
          }}
        />
      )}

      {/* Distant star/dot field */}
      {showStars && <StarField />}

      {/* Optional horizontal flow lines (data plane) */}
      {showFlow && <FlowLines />}

      {/* Vignette */}
      <div
        style={{
          position: 'absolute',
          inset: 0,
          background:
            'radial-gradient(ellipse at center, transparent 40%, rgba(7,9,12,0.55) 100%)',
        }}
      />

      {children}
    </div>
  );
}

function StarField() {
  const stars = Array.from({ length: 64 }, (_, i) => {
    const x = (i * 137.508) % 100;
    const y = (i * 73.31) % 100;
    const r = 0.6 + ((i * 17) % 10) / 10;
    const o = 0.15 + ((i * 7) % 8) / 30;
    const dur = 3 + ((i * 11) % 6);
    return { x, y, r, o, dur, key: i };
  });
  return (
    <svg style={{ position: 'absolute', inset: 0, width: '100%', height: '100%' }}>
      {stars.map((s) => (
        <circle
          key={s.key}
          cx={`${s.x}%`}
          cy={`${s.y}%`}
          r={s.r}
          fill="#cfe5ff"
          opacity={s.o}
        >
          <animate
            attributeName="opacity"
            values={`${s.o};${s.o * 0.35};${s.o}`}
            dur={`${s.dur}s`}
            repeatCount="indefinite"
          />
        </circle>
      ))}
    </svg>
  );
}

function FlowLines() {
  return (
    <svg
      viewBox="0 0 1200 800"
      preserveAspectRatio="none"
      style={{ position: 'absolute', inset: 0, width: '100%', height: '100%', opacity: 0.45 }}
    >
      {Array.from({ length: 6 }).map((_, i) => {
        const y = 80 + i * 110;
        const dur = 6 + i * 1.2;
        return (
          <g key={i}>
            <line
              x1="0"
              y1={y}
              x2="1200"
              y2={y}
              stroke="rgba(109, 211, 255, 0.18)"
              strokeWidth="0.6"
              strokeDasharray="1 7"
            />
            <circle r="2.4" fill="#6dd3ff">
              <animateMotion
                dur={`${dur}s`}
                repeatCount="indefinite"
                path={`M0,${y} L1200,${y}`}
              />
              <animate
                attributeName="opacity"
                values="0;1;1;0"
                dur={`${dur}s`}
                repeatCount="indefinite"
              />
            </circle>
          </g>
        );
      })}
    </svg>
  );
}
