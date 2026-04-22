'use client';

/**
 * SensorHorizontalScroll — pinned vertical-to-horizontal scroll panel.
 *
 * Pattern from motion.dev: a tall outer section pins a viewport-sized
 * sticky child whose inner track translates horizontally as the page
 * scrolls. Five sensor cards pass under a fixed reticle, each with its
 * own ticker visualization and pipeline pin-out.
 */

import { useRef } from 'react';
import { motion, useScroll, useTransform, useReducedMotion } from 'motion/react';
import { EASE } from './motion';
import { SENSORS } from '../copy';
import RgbdViz from './sensors/RgbdViz';
import ImuViz from './sensors/ImuViz';
import ProprioViz from './sensors/ProprioViz';
import LidarViz from './sensors/LidarViz';
import AudioViz from './sensors/AudioViz';

const VIZ = {
  rgbd: RgbdViz,
  imu: ImuViz,
  proprio: ProprioViz,
  lidar: LidarViz,
  audio: AudioViz,
} as const;

export default function SensorHorizontalScroll() {
  const ref = useRef<HTMLElement>(null);
  const reduce = useReducedMotion();

  const { scrollYProgress } = useScroll({
    target: ref,
    offset: ['start start', 'end end'],
  });

  // Translate the inner track from 0% to -80% (5 cards, 1 viewport-width each).
  const x = useTransform(
    scrollYProgress,
    [0, 1],
    reduce ? ['0%', '0%'] : ['0%', '-80%'],
    { ease: EASE },
  );

  const headerOpacity = useTransform(scrollYProgress, [0, 0.06, 0.94, 1], [0, 1, 1, 0]);

  return (
    <section
      ref={ref}
      aria-label="Sensor lab — horizontal pan through five sensor classes"
      style={{
        position: 'relative',
        // 5 cards × ~100vh of scroll each, plus a head/tail pad
        height: '520vh',
      }}
    >
      <div
        style={{
          position: 'sticky',
          top: 0,
          height: '100svh',
          overflow: 'hidden',
          display: 'flex',
          flexDirection: 'column',
        }}
      >
        {/* Section header */}
        <motion.header
          style={{
            opacity: headerOpacity,
            padding: '4vh 1.5rem 1.4rem',
            display: 'flex',
            alignItems: 'baseline',
            justifyContent: 'space-between',
            gap: '1rem',
            flexWrap: 'wrap',
          }}
        >
          <div>
            <span className="lewm-badge" style={{ borderColor: 'var(--lewm-cyan)', color: 'var(--lewm-cyan)' }}>
              Diagram ③ · sensor lab
            </span>
            <h2
              className="lewm-editorial"
              style={{
                margin: '0.7rem 0 0',
                fontSize: 'clamp(1.6rem, 3vw, 2.4rem)',
                letterSpacing: '-0.015em',
              }}
            >
              Five sensors, one manifold.
            </h2>
            <p style={{ margin: '0.5rem 0 0', color: 'var(--lewm-mute)', maxWidth: '52rem' }}>
              Each sensor class runs its own three-step pipeline. All emit onto the same
              SIGReg-regularized isotropic-Gaussian latent so any consumer can subscribe
              uniformly.
            </p>
          </div>
          <div className="lewm-mono" style={{ color: 'var(--lewm-mute)', fontSize: '0.78rem' }}>
            ↓ scroll · pan →
          </div>
        </motion.header>

        {/* Track */}
        <div style={{ position: 'relative', flex: 1 }}>
          <motion.div
            style={{
              x,
              willChange: 'transform',
              display: 'flex',
              height: '100%',
              gap: '0',
            }}
          >
            {SENSORS.map((s) => {
              const Viz = VIZ[s.id as keyof typeof VIZ];
              return (
                <article
                  key={s.id}
                  style={{
                    flex: '0 0 100vw',
                    height: '100%',
                    padding: '0 4vw 4vh',
                    display: 'grid',
                    gridTemplateColumns: 'minmax(280px, 1fr) minmax(320px, 1.3fr)',
                    gap: '3vw',
                    alignItems: 'center',
                  }}
                >
                  <div>
                    <div
                      className="lewm-mono"
                      style={{
                        fontSize: '0.78rem',
                        color: 'var(--lewm-mute)',
                        letterSpacing: '0.05em',
                        textTransform: 'uppercase',
                      }}
                    >
                      Sensor {s.idx} / 5 · {s.codec}
                    </div>
                    <h3
                      className="lewm-editorial"
                      style={{
                        margin: '0.5rem 0 0.4rem',
                        fontSize: 'clamp(1.6rem, 2.6vw, 2.2rem)',
                        color: 'var(--lewm-white)',
                      }}
                    >
                      {s.title}
                    </h3>
                    <p style={{ margin: 0, color: 'var(--lewm-mute)', fontSize: '0.95rem' }}>
                      {s.tagline}
                    </p>
                    <ul
                      className="lewm-mono"
                      style={{
                        listStyle: 'none',
                        padding: 0,
                        margin: '1.5rem 0 0',
                        display: 'grid',
                        gap: '0.5rem',
                        fontSize: '0.85rem',
                      }}
                    >
                      {s.specs.map((spec) => (
                        <li key={spec.label} style={{ display: 'grid', gridTemplateColumns: '8.5rem 1fr', gap: '0.5rem' }}>
                          <span style={{ color: 'var(--lewm-mute)' }}>{spec.label}</span>
                          <span style={{ color: 'var(--lewm-ink)' }}>{spec.value}</span>
                        </li>
                      ))}
                    </ul>
                    <div style={{ marginTop: '1.5rem', display: 'flex', gap: '0.4rem', flexWrap: 'wrap' }}>
                      {s.tags.map((t) => (
                        <span
                          key={t}
                          className="lewm-badge"
                          style={{
                            borderColor: 'var(--lewm-cyan)',
                            color: 'var(--lewm-cyan)',
                          }}
                        >
                          {t}
                        </span>
                      ))}
                    </div>
                  </div>

                  <div
                    className="lewm-panel"
                    style={{
                      padding: '1.4rem',
                      borderColor: 'var(--lewm-cyan)',
                      height: 'min(54vh, 460px)',
                      display: 'flex',
                      flexDirection: 'column',
                    }}
                  >
                    <div
                      className="lewm-mono"
                      style={{
                        fontSize: '0.72rem',
                        color: 'var(--lewm-mute)',
                        letterSpacing: '0.04em',
                      }}
                    >
                      live preview · {s.codec}
                    </div>
                    <div style={{ flex: 1, marginTop: '0.6rem', minHeight: 0 }}>
                      <Viz />
                    </div>
                    <div
                      className="lewm-mono"
                      style={{
                        marginTop: '0.6rem',
                        fontSize: '0.72rem',
                        color: 'var(--lewm-mute)',
                        display: 'flex',
                        gap: '0.6rem',
                        flexWrap: 'wrap',
                      }}
                    >
                      <span>collect</span>
                      <span>→</span>
                      <span>aggregate</span>
                      <span>→</span>
                      <span style={{ color: 'var(--lewm-cyan)' }}>encode → z_t ∈ ℝ¹⁹²</span>
                    </div>
                  </div>
                </article>
              );
            })}
          </motion.div>

          {/* Sticky reticle / progress bar */}
          <ProgressRail progress={scrollYProgress} count={5} />
        </div>
      </div>
    </section>
  );
}

function ProgressRail({
  progress,
  count,
}: {
  progress: ReturnType<typeof useScroll>['scrollYProgress'];
  count: number;
}) {
  const width = useTransform(progress, [0, 1], ['0%', '100%']);
  return (
    <div
      style={{
        position: 'absolute',
        bottom: '2vh',
        left: '4vw',
        right: '4vw',
        display: 'flex',
        alignItems: 'center',
        gap: '1rem',
        pointerEvents: 'none',
      }}
    >
      <span className="lewm-mono" style={{ fontSize: '0.7rem', color: 'var(--lewm-mute)' }}>
        00
      </span>
      <div
        style={{
          flex: 1,
          height: '2px',
          background: 'var(--lewm-line)',
          position: 'relative',
        }}
      >
        <motion.div
          style={{
            position: 'absolute',
            inset: 0,
            background: 'linear-gradient(90deg, var(--lewm-cyan), var(--lewm-violet))',
            transformOrigin: 'left',
            width,
          }}
        />
        {Array.from({ length: count }).map((_, i) => (
          <span
            key={i}
            style={{
              position: 'absolute',
              left: `${(i / (count - 1)) * 100}%`,
              top: '-3px',
              transform: 'translateX(-50%)',
              width: '8px',
              height: '8px',
              borderRadius: '999px',
              background: 'var(--lewm-bg)',
              border: '1px solid var(--lewm-line)',
            }}
          />
        ))}
      </div>
      <span className="lewm-mono" style={{ fontSize: '0.7rem', color: 'var(--lewm-mute)' }}>
        {String(count).padStart(2, '0')}
      </span>
    </div>
  );
}
