'use client';

/**
 * DeepDive — scroll-driven mask reveal of the Collect/Aggregate/Encode internals.
 *
 * As you descend, a clip-path opens vertically to expose the full internal
 * pipeline diagram. Annotations fade in at progress thresholds.
 */

import { useRef } from 'react';
import { motion, useScroll, useTransform, useReducedMotion } from 'motion/react';
import { EASE } from './motion';
import { DEEPDIVE } from '../copy';

export default function DeepDive() {
  const ref = useRef<HTMLElement>(null);
  const reduce = useReducedMotion();

  const { scrollYProgress } = useScroll({
    target: ref,
    offset: ['start start', 'end end'],
  });

  // Vertical clip-path reveal.
  const clip = useTransform(
    scrollYProgress,
    [0, 0.55],
    reduce
      ? ['inset(0% 0% 0% 0%)', 'inset(0% 0% 0% 0%)']
      : ['inset(0% 0% 100% 0%)', 'inset(0% 0% 0% 0%)'],
    { ease: EASE },
  );

  // Annotation opacities at three progress beats.
  const opCollect = useTransform(scrollYProgress, [0.10, 0.25], [0, 1]);
  const opAggregate = useTransform(scrollYProgress, [0.30, 0.45], [0, 1]);
  const opEncode = useTransform(scrollYProgress, [0.50, 0.65], [0, 1]);
  const opCaption = useTransform(scrollYProgress, [0.60, 0.75], [0, 1]);

  return (
    <section
      ref={ref}
      style={{ position: 'relative', height: '320vh' }}
      aria-label="Deep dive: pipeline internals revealed by scroll"
    >
      <div
        style={{
          position: 'sticky',
          top: 0,
          height: '100svh',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          padding: '0 1.5rem',
          overflow: 'hidden',
        }}
      >
        <div style={{ width: '100%', maxWidth: '78rem' }}>
          <header style={{ textAlign: 'center', marginBottom: '1.4rem' }}>
            <span
              className="lewm-badge"
              style={{ borderColor: 'var(--lewm-violet)', color: 'var(--lewm-violet)' }}
            >
              Diagram ④ · pipeline internals
            </span>
            <h2
              className="lewm-editorial"
              style={{
                margin: '0.6rem 0 0',
                fontSize: 'clamp(1.6rem, 3vw, 2.4rem)',
                letterSpacing: '-0.015em',
              }}
            >
              {DEEPDIVE.title}
            </h2>
            <p style={{ margin: '0.4rem auto 0', color: 'var(--lewm-mute)', maxWidth: '52rem' }}>
              {DEEPDIVE.subtitle}
            </p>
          </header>

          <motion.div
            className="lewm-panel"
            style={{
              clipPath: clip,
              willChange: 'clip-path',
              padding: '1.6rem',
              borderColor: 'var(--lewm-violet)',
            }}
          >
            <PipelineInternalsSvg />
          </motion.div>

          <div
            style={{
              marginTop: '1.2rem',
              display: 'grid',
              gridTemplateColumns: 'repeat(3, 1fr)',
              gap: '0.8rem',
              fontSize: '0.85rem',
            }}
          >
            <motion.div style={{ opacity: opCollect }}>
              <AnnotationCard color="cyan" head="① collect" body={DEEPDIVE.collect} />
            </motion.div>
            <motion.div style={{ opacity: opAggregate }}>
              <AnnotationCard color="mint" head="② aggregate" body={DEEPDIVE.aggregate} />
            </motion.div>
            <motion.div style={{ opacity: opEncode }}>
              <AnnotationCard color="violet" head="③ encode" body={DEEPDIVE.encode} />
            </motion.div>
          </div>

          <motion.p
            style={{
              opacity: opCaption,
              margin: '1.4rem 0 0',
              textAlign: 'center',
              color: 'var(--lewm-mute)',
              fontSize: '0.85rem',
            }}
            className="lewm-mono"
          >
            {DEEPDIVE.caption}
          </motion.p>
        </div>
      </div>
    </section>
  );
}

function AnnotationCard({
  color,
  head,
  body,
}: {
  color: 'cyan' | 'mint' | 'violet';
  head: string;
  body: string;
}) {
  return (
    <div
      className="lewm-panel"
      style={{
        padding: '0.9rem 1rem',
        borderColor: `var(--lewm-${color})`,
      }}
    >
      <div
        className="lewm-mono"
        style={{ color: `var(--lewm-${color})`, fontSize: '0.78rem', letterSpacing: '0.04em' }}
      >
        {head}
      </div>
      <div style={{ marginTop: '0.3rem', color: 'var(--lewm-ink)' }}>{body}</div>
    </div>
  );
}

function PipelineInternalsSvg() {
  return (
    <svg
      viewBox="0 0 1100 360"
      role="img"
      aria-label="Pipeline internals: Collect (transmit-gate) → Aggregate (multi-sensor fusion) → Encode (SIGReg-manifold ViT-tiny) emitting z_t to the Observation Wire."
      style={{ width: '100%', height: 'auto', display: 'block' }}
    >
      <defs>
        <marker id="dd-arrow" viewBox="0 -5 10 10" refX="10" refY="0" markerWidth="7" markerHeight="7" orient="auto">
          <path d="M0,-4L10,0L0,4" fill="currentColor" />
        </marker>
        <linearGradient id="dd-cyan" x1="0" x2="1" y1="0" y2="0">
          <stop offset="0%" stopColor="#6dd3ff" stopOpacity="0.3" />
          <stop offset="100%" stopColor="#6dd3ff" stopOpacity="0.05" />
        </linearGradient>
        <linearGradient id="dd-mint" x1="0" x2="1" y1="0" y2="0">
          <stop offset="0%" stopColor="#7ae0c3" stopOpacity="0.3" />
          <stop offset="100%" stopColor="#7ae0c3" stopOpacity="0.05" />
        </linearGradient>
        <linearGradient id="dd-violet" x1="0" x2="1" y1="0" y2="0">
          <stop offset="0%" stopColor="#b18cff" stopOpacity="0.3" />
          <stop offset="100%" stopColor="#b18cff" stopOpacity="0.05" />
        </linearGradient>
      </defs>

      {/* Source channels (left) */}
      <g>
        {['rgb-d', 'imu', 'proprio', 'lidar', 'audio'].map((s, i) => {
          const y = 30 + i * 56;
          return (
            <g key={s}>
              <rect x="20" y={y} width="86" height="40" rx="4" fill="#0e1420" stroke="#3a4a60" />
              <text x="63" y={y + 24} fontFamily="var(--font-mono-lewm), monospace" fontSize="10" fill="#e6edf3" textAnchor="middle">{s}</text>
              <line x1="106" y1={y + 20} x2="180" y2={y + 20} stroke="#6dd3ff" strokeWidth="1.2" markerEnd="url(#dd-arrow)" color="#6dd3ff" />
              {/* moving sample */}
              <circle r="1.8" fill="#6dd3ff">
                <animateMotion dur={`${1.4 + i * 0.3}s`} repeatCount="indefinite" path={`M106,${y + 20} L178,${y + 20}`} />
                <animate attributeName="opacity" values="0;1;0" dur={`${1.4 + i * 0.3}s`} repeatCount="indefinite" />
              </circle>
            </g>
          );
        })}
      </g>

      {/* COLLECT block */}
      <g>
        <rect x="180" y="20" width="180" height="320" rx="6" fill="url(#dd-cyan)" stroke="#6dd3ff" />
        <text x="270" y="46" textAnchor="middle" fontFamily="var(--font-mono-lewm), monospace" fontSize="11" fill="#6dd3ff">COLLECT · transmit-gate</text>
        {/* Internals: ring buffer + gate */}
        <rect x="200" y="70" width="140" height="40" rx="3" fill="#0e1420" stroke="#3a4a60" />
        <text x="270" y="95" textAnchor="middle" fontFamily="var(--font-mono-lewm), monospace" fontSize="9" fill="#e6edf3">ring buffer · 100 ms</text>
        <rect x="200" y="120" width="140" height="40" rx="3" fill="#0e1420" stroke="#3a4a60" />
        <text x="270" y="145" textAnchor="middle" fontFamily="var(--font-mono-lewm), monospace" fontSize="9" fill="#e6edf3">novelty gate · MAD</text>
        <rect x="200" y="170" width="140" height="40" rx="3" fill="#0e1420" stroke="#3a4a60" />
        <text x="270" y="195" textAnchor="middle" fontFamily="var(--font-mono-lewm), monospace" fontSize="9" fill="#e6edf3">drop-redundant</text>
        <text x="270" y="232" textAnchor="middle" fontFamily="var(--font-mono-lewm), monospace" fontSize="8" fill="#8b98a8">≈ 7× bandwidth reduction</text>
        <text x="270" y="252" textAnchor="middle" fontFamily="var(--font-mono-lewm), monospace" fontSize="8" fill="#8b98a8">trained per sensor class</text>
        <text x="270" y="290" textAnchor="middle" fontFamily="var(--font-mono-lewm), monospace" fontSize="8" fill="#6dd3ff">RVF segment ① · ADR-056</text>
      </g>

      <line x1="360" y1="180" x2="420" y2="180" stroke="#7ae0c3" strokeWidth="1.5" markerEnd="url(#dd-arrow)" color="#7ae0c3" />
      <circle r="2" fill="#7ae0c3">
        <animateMotion dur="1.2s" repeatCount="indefinite" path="M360,180 L418,180" />
        <animate attributeName="opacity" values="0;1;0" dur="1.2s" repeatCount="indefinite" />
      </circle>

      {/* AGGREGATE block */}
      <g>
        <rect x="420" y="20" width="220" height="320" rx="6" fill="url(#dd-mint)" stroke="#7ae0c3" />
        <text x="530" y="46" textAnchor="middle" fontFamily="var(--font-mono-lewm), monospace" fontSize="11" fill="#7ae0c3">AGGREGATE · multi-sensor fusion</text>
        {/* tokenizer + cross-attn */}
        <rect x="440" y="70" width="180" height="40" rx="3" fill="#0e1420" stroke="#3a4a60" />
        <text x="530" y="95" textAnchor="middle" fontFamily="var(--font-mono-lewm), monospace" fontSize="9" fill="#e6edf3">per-modality tokenizer</text>
        <rect x="440" y="120" width="180" height="40" rx="3" fill="#0e1420" stroke="#3a4a60" />
        <text x="530" y="145" textAnchor="middle" fontFamily="var(--font-mono-lewm), monospace" fontSize="9" fill="#e6edf3">temporal align · 100 ms</text>
        <rect x="440" y="170" width="180" height="40" rx="3" fill="#0e1420" stroke="#3a4a60" />
        <text x="530" y="195" textAnchor="middle" fontFamily="var(--font-mono-lewm), monospace" fontSize="9" fill="#e6edf3">cross-modal attention</text>
        <rect x="440" y="220" width="180" height="40" rx="3" fill="#0e1420" stroke="#3a4a60" />
        <text x="530" y="245" textAnchor="middle" fontFamily="var(--font-mono-lewm), monospace" fontSize="9" fill="#e6edf3">sensor-drop graceful</text>
        <text x="530" y="290" textAnchor="middle" fontFamily="var(--font-mono-lewm), monospace" fontSize="8" fill="#7ae0c3">RVF segment ② · ADR-056</text>
      </g>

      <line x1="640" y1="180" x2="700" y2="180" stroke="#b18cff" strokeWidth="1.5" markerEnd="url(#dd-arrow)" color="#b18cff" />
      <circle r="2" fill="#b18cff">
        <animateMotion dur="1.4s" repeatCount="indefinite" path="M640,180 L698,180" />
        <animate attributeName="opacity" values="0;1;0" dur="1.4s" repeatCount="indefinite" />
      </circle>

      {/* ENCODE block */}
      <g>
        <rect x="700" y="20" width="240" height="320" rx="6" fill="url(#dd-violet)" stroke="#b18cff" />
        <text x="820" y="46" textAnchor="middle" fontFamily="var(--font-mono-lewm), monospace" fontSize="11" fill="#b18cff">ENCODE · SIGReg ViT-tiny</text>
        <rect x="720" y="70" width="200" height="40" rx="3" fill="#0e1420" stroke="#3a4a60" />
        <text x="820" y="95" textAnchor="middle" fontFamily="var(--font-mono-lewm), monospace" fontSize="9" fill="#e6edf3">ViT-tiny · 6L · 192d</text>
        <rect x="720" y="120" width="200" height="40" rx="3" fill="#0e1420" stroke="#3a4a60" />
        <text x="820" y="145" textAnchor="middle" fontFamily="var(--font-mono-lewm), monospace" fontSize="9" fill="#e6edf3">SIGReg · isotropic Gauss</text>
        <rect x="720" y="170" width="200" height="40" rx="3" fill="#0e1420" stroke="#3a4a60" />
        <text x="820" y="195" textAnchor="middle" fontFamily="var(--font-mono-lewm), monospace" fontSize="9" fill="#e6edf3">VICReg variance + invariance</text>
        <rect x="720" y="220" width="200" height="40" rx="3" fill="#0e1420" stroke="#3a4a60" />
        <text x="820" y="245" textAnchor="middle" fontFamily="var(--font-mono-lewm), monospace" fontSize="9" fill="#e6edf3">CBOR · Ed25519 · ExoChain idx</text>
        <text x="820" y="290" textAnchor="middle" fontFamily="var(--font-mono-lewm), monospace" fontSize="8" fill="#b18cff">RVF segment ③ · ADR-057</text>
      </g>

      {/* z_t emission */}
      <line x1="940" y1="180" x2="1080" y2="180" stroke="#b18cff" strokeWidth="1.8" markerEnd="url(#dd-arrow)" color="#b18cff" />
      <text x="1010" y="172" textAnchor="middle" fontFamily="var(--font-mono-lewm), monospace" fontSize="10" fill="#b18cff">z_t ∈ ℝ¹⁹²</text>
      <text x="1010" y="198" textAnchor="middle" fontFamily="var(--font-mono-lewm), monospace" fontSize="8" fill="#8b98a8">→ mesh.sensor.v1.encoded</text>
      <circle r="3" fill="#b18cff">
        <animateMotion dur="1s" repeatCount="indefinite" path="M940,180 L1078,180" />
        <animate attributeName="opacity" values="0;1;1;0" dur="1s" repeatCount="indefinite" />
      </circle>
    </svg>
  );
}
