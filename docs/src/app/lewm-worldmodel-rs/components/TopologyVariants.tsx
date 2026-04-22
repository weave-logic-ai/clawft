'use client';

/**
 * TopologyVariants — the three deployment topologies for WorldModelService.
 *
 * Single primary · Raft-elected primary + standby · Peer-to-peer.
 * Each card is its own animated SVG with traffic flowing as appropriate.
 */

import { useRef } from 'react';
import { motion, useScroll, useTransform, useReducedMotion } from 'motion/react';
import { TOPOLOGIES } from '../copy';

export default function TopologyVariants() {
  const ref = useRef<HTMLElement>(null);
  const reduce = useReducedMotion();
  const { scrollYProgress } = useScroll({
    target: ref,
    offset: ['start end', 'end start'],
  });
  const opacity = useTransform(scrollYProgress, [0.1, 0.3], [0, 1]);
  const y = useTransform(scrollYProgress, [0.1, 0.4], reduce ? [0, 0] : [40, 0]);

  return (
    <section
      ref={ref}
      style={{ padding: '12vh 1.5rem', position: 'relative' }}
      aria-label="Three deployment topologies for WorldModelService"
    >
      <motion.header
        style={{ opacity, y, textAlign: 'center', maxWidth: '54rem', margin: '0 auto 3rem' }}
      >
        <span className="lewm-badge" style={{ borderColor: 'var(--lewm-violet)', color: 'var(--lewm-violet)' }}>
          Diagram ⑤ · ADR-054
        </span>
        <h2
          className="lewm-editorial"
          style={{ marginTop: '0.7rem', fontSize: 'clamp(1.6rem, 3vw, 2.4rem)', letterSpacing: '-0.015em' }}
        >
          Three deployment topologies.
        </h2>
        <p style={{ margin: '0.5rem 0 0', color: 'var(--lewm-mute)' }}>
          The world-model contract is identical. The deployment shape adapts to fleet size,
          mesh quality, and trust boundaries — every variant is observationally equivalent
          to local ECC consumers.
        </p>
      </motion.header>

      <div
        style={{
          display: 'grid',
          gap: '1.4rem',
          gridTemplateColumns: 'repeat(auto-fit, minmax(280px, 1fr))',
          maxWidth: '76rem',
          margin: '0 auto',
        }}
      >
        {TOPOLOGIES.map((t) => (
          <TopologyCard key={t.id} topo={t} />
        ))}
      </div>
    </section>
  );
}

function TopologyCard({ topo }: { topo: (typeof TOPOLOGIES)[number] }) {
  const ref = useRef<HTMLElement>(null);
  const reduce = useReducedMotion();
  const { scrollYProgress } = useScroll({
    target: ref,
    offset: ['start 0.95', 'center 0.5'],
  });
  const y = useTransform(scrollYProgress, [0, 1], reduce ? [0, 0] : [40, 0]);
  const opacity = useTransform(scrollYProgress, [0, 0.6], [0, 1]);

  return (
    <motion.article
      ref={ref}
      className="lewm-panel"
      style={{
        y,
        opacity,
        padding: '1.4rem',
        borderColor: 'var(--lewm-violet)',
        display: 'flex',
        flexDirection: 'column',
        gap: '1rem',
      }}
    >
      <header style={{ display: 'flex', alignItems: 'baseline', justifyContent: 'space-between', gap: '0.6rem' }}>
        <h3
          className="lewm-editorial"
          style={{ margin: 0, fontSize: '1.15rem', color: 'var(--lewm-white)' }}
        >
          {topo.title}
        </h3>
        <span className="lewm-mono" style={{ color: 'var(--lewm-mute)', fontSize: '0.72rem' }}>
          {topo.fleet}
        </span>
      </header>
      <div style={{ height: '180px', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
        {topo.id === 'single' && <SingleSvg />}
        {topo.id === 'standby' && <StandbySvg />}
        {topo.id === 'p2p' && <P2pSvg />}
      </div>
      <p style={{ margin: 0, color: 'var(--lewm-ink)', fontSize: '0.92rem', lineHeight: 1.5 }}>
        {topo.body}
      </p>
      <ul
        className="lewm-mono"
        style={{
          listStyle: 'none',
          padding: 0,
          margin: 0,
          color: 'var(--lewm-mute)',
          fontSize: '0.8rem',
          display: 'grid',
          gap: '0.3rem',
        }}
      >
        {topo.props.map((p) => (
          <li key={p}>· {p}</li>
        ))}
      </ul>
    </motion.article>
  );
}

function SingleSvg() {
  return (
    <svg viewBox="0 0 240 180" style={{ width: '100%', height: '100%' }}>
      {/* sensors */}
      {Array.from({ length: 5 }).map((_, i) => {
        const x = 30 + i * 45;
        return (
          <g key={i}>
            <circle cx={x} cy={150} r="10" fill="#0e1420" stroke="#6dd3ff" />
            <text x={x} y={154} textAnchor="middle" fontFamily="var(--font-mono-lewm), monospace" fontSize="8" fill="#6dd3ff">s{i}</text>
            <line x1={x} y1={140} x2={120} y2={60} stroke="#6dd3ff" strokeWidth="0.6" opacity="0.5" />
            <circle r="1.6" fill="#6dd3ff">
              <animateMotion dur={`${1.5 + i * 0.2}s`} repeatCount="indefinite" path={`M${x},140 L120,60`} />
              <animate attributeName="opacity" values="0;1;0" dur={`${1.5 + i * 0.2}s`} repeatCount="indefinite" />
            </circle>
          </g>
        );
      })}
      {/* world-model node */}
      <rect x="80" y="40" width="80" height="40" rx="4" fill="#1c1430" stroke="#b18cff" />
      <text x="120" y="65" textAnchor="middle" fontFamily="var(--font-mono-lewm), monospace" fontSize="10" fill="#b18cff">PRIMARY</text>
    </svg>
  );
}

function StandbySvg() {
  return (
    <svg viewBox="0 0 240 180" style={{ width: '100%', height: '100%' }}>
      {Array.from({ length: 5 }).map((_, i) => {
        const x = 24 + i * 47;
        return (
          <g key={i}>
            <circle cx={x} cy={155} r="8" fill="#0e1420" stroke="#6dd3ff" />
            <text x={x} y={158} textAnchor="middle" fontFamily="var(--font-mono-lewm), monospace" fontSize="7" fill="#6dd3ff">s{i}</text>
            <line x1={x} y1={147} x2={80} y2={70} stroke="#6dd3ff" strokeWidth="0.5" opacity="0.4" />
            <circle r="1.4" fill="#6dd3ff">
              <animateMotion dur={`${1.6 + i * 0.2}s`} repeatCount="indefinite" path={`M${x},147 L80,70`} />
              <animate attributeName="opacity" values="0;1;0" dur={`${1.6 + i * 0.2}s`} repeatCount="indefinite" />
            </circle>
          </g>
        );
      })}
      {/* primary */}
      <rect x="40" y="40" width="80" height="40" rx="4" fill="#1c1430" stroke="#b18cff" />
      <text x="80" y="65" textAnchor="middle" fontFamily="var(--font-mono-lewm), monospace" fontSize="10" fill="#b18cff">PRIMARY</text>
      {/* standby */}
      <rect x="140" y="40" width="80" height="40" rx="4" fill="#0e1420" stroke="#3a4a60" strokeDasharray="3 3" />
      <text x="180" y="65" textAnchor="middle" fontFamily="var(--font-mono-lewm), monospace" fontSize="10" fill="#8b98a8">STANDBY</text>
      {/* raft replication */}
      <line x1="120" y1="60" x2="140" y2="60" stroke="#ffb65a" strokeWidth="1.2" strokeDasharray="2 2" />
      <text x="130" y="35" textAnchor="middle" fontFamily="var(--font-mono-lewm), monospace" fontSize="8" fill="#ffb65a">raft</text>
    </svg>
  );
}

function P2pSvg() {
  // five nodes in a ring connected in a partial mesh
  const cx = 120;
  const cy = 90;
  const r = 60;
  const nodes = Array.from({ length: 5 }, (_, i) => {
    const a = (i / 5) * Math.PI * 2 - Math.PI / 2;
    return { x: cx + Math.cos(a) * r, y: cy + Math.sin(a) * r, i };
  });
  return (
    <svg viewBox="0 0 240 180" style={{ width: '100%', height: '100%' }}>
      {/* mesh edges */}
      {nodes.map((n, i) =>
        nodes
          .filter((_, j) => j > i && (j - i) % 2 === 1)
          .map((m, k) => (
            <g key={`${i}-${m.i}`}>
              <line x1={n.x} y1={n.y} x2={m.x} y2={m.y} stroke="#3a4a60" strokeWidth="0.8" strokeDasharray="2 3" />
              <circle r="1.6" fill="#7ae0c3">
                <animateMotion
                  dur={`${2 + ((i + k) % 3)}s`}
                  repeatCount="indefinite"
                  path={`M${n.x},${n.y} L${m.x},${m.y}`}
                />
                <animate attributeName="opacity" values="0;1;0" dur={`${2 + ((i + k) % 3)}s`} repeatCount="indefinite" />
              </circle>
            </g>
          )),
      )}
      {nodes.map((n) => (
        <g key={n.i}>
          <circle cx={n.x} cy={n.y} r="11" fill="#1c1430" stroke="#b18cff" />
          <text x={n.x} y={n.y + 3} textAnchor="middle" fontFamily="var(--font-mono-lewm), monospace" fontSize="9" fill="#b18cff">w{n.i}</text>
        </g>
      ))}
    </svg>
  );
}
