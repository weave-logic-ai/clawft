import { Fraunces, JetBrains_Mono } from 'next/font/google';
import './lewm.css';
import { HERO, PANELS, CLOSER } from './copy';
import LatentDots from './components/LatentDots';
import HeroZoomFrame from './components/HeroZoomFrame';
import SystemPanelSvg from './components/SystemPanelSvg';
import Pullquote from './components/Pullquote';
import InversionFlip from './components/InversionFlip';
import PanelPopup from './components/PanelPopup';
import CrossViewDissolve from './components/CrossViewDissolve';
import HoeaLoop from './components/HoeaLoop';
import AdrIndex from './components/AdrIndex';

const fraunces = Fraunces({
  subsets: ['latin'],
  axes: ['SOFT', 'WONK', 'opsz'],
  variable: '--font-fraunces',
  display: 'swap',
});

const mono = JetBrains_Mono({
  subsets: ['latin'],
  weight: ['400', '500'],
  variable: '--font-mono-lewm',
  display: 'swap',
});

export default function LewmWorldModelPage() {
  return (
    <div
      className={`lewm-scope ${fraunces.variable} ${mono.variable}`}
      style={{ position: 'relative', overflowX: 'hidden' }}
    >
      {/* Background grid, fixed under content */}
      <div
        className="lewm-grid-bg"
        aria-hidden="true"
        style={{
          position: 'fixed',
          inset: 0,
          zIndex: 0,
          pointerEvents: 'none',
        }}
      />

      <main style={{ position: 'relative', zIndex: 1 }}>
        {/* 1. Hero + 2. Zoom into system diagram */}
        <HeroZoomFrame>
          <header style={{ textAlign: 'center' }}>
            <div
              className="lewm-mono"
              style={{
                fontSize: '0.78rem',
                color: 'var(--lewm-mute)',
                letterSpacing: '0.05em',
                marginBottom: '2.5rem',
                textTransform: 'uppercase',
              }}
            >
              {HERO.eyebrow}
            </div>
            <h1
              className="lewm-editorial"
              style={{
                fontSize: 'clamp(2.8rem, 7vw, 5.4rem)',
                margin: 0,
                letterSpacing: '-0.025em',
                color: 'var(--lewm-white)',
                lineHeight: 1.05,
              }}
            >
              {HERO.title}
            </h1>
            <p
              className="lewm-mono"
              style={{
                marginTop: '1.2rem',
                color: 'var(--lewm-cyan)',
                fontSize: 'clamp(0.95rem, 1.6vw, 1.15rem)',
              }}
            >
              {HERO.subtitle}
            </p>
            <p
              className="lewm-mono"
              style={{
                marginTop: '0.4rem',
                color: 'var(--lewm-mute)',
                fontSize: '0.85rem',
              }}
            >
              {HERO.meta}
            </p>

            <div style={{ marginTop: '3.2rem', display: 'flex', justifyContent: 'center' }}>
              <LatentDots />
            </div>

            <p
              className="lewm-mono"
              style={{
                marginTop: '3rem',
                color: 'var(--lewm-mute)',
                fontSize: '0.78rem',
              }}
            >
              ↓  {HERO.scrollHint}
            </p>
          </header>
        </HeroZoomFrame>

        {/* System overview revealed beneath the zoom */}
        <section
          aria-label="WeftOS × LeWM system overview"
          style={{
            padding: '4vh 1.5rem 10vh',
            display: 'flex',
            justifyContent: 'center',
          }}
        >
          <div style={{ maxWidth: '78rem', width: '100%' }}>
            <SystemPanelSvg />
          </div>
        </section>

        {/* 3. TL;DR pullquote */}
        <Pullquote />

        {/* 4. The inversion (decoupling invariant) */}
        <InversionFlip />

        {/* 5–9. Individual panels unfurl */}
        <div style={{ padding: '4vh 0' }}>
          {PANELS.map((p) => (
            <PanelPopup
              key={p.id}
              id={p.id}
              number={p.number}
              title={p.title}
              tagline={p.tagline}
              adrs={p.adrs}
              body={p.body}
              accent={p.accent}
            />
          ))}
        </div>

        {/* 10. Cross-view dissolve → H-O-E-A */}
        <CrossViewDissolve />

        {/* 11. H-O-E-A loop (scroll velocity modulates cycle speed) */}
        <HoeaLoop />

        {/* 12. ADR index + closer */}
        <AdrIndex />

        <section
          aria-label="Closer"
          style={{
            padding: '8vh 1.5rem 18vh',
            display: 'flex',
            justifyContent: 'center',
            textAlign: 'center',
          }}
        >
          <div style={{ maxWidth: '52rem' }}>
            <p
              className="lewm-editorial"
              style={{
                fontSize: 'clamp(1.4rem, 2.4vw, 1.9rem)',
                lineHeight: 1.35,
                color: 'var(--lewm-ink)',
                margin: 0,
              }}
            >
              {CLOSER.quote}
            </p>
            <p
              style={{
                marginTop: '1.4rem',
                color: 'var(--lewm-mute)',
                fontSize: '0.95rem',
              }}
            >
              {CLOSER.kicker}
            </p>

            <div
              style={{
                marginTop: '2.5rem',
                display: 'flex',
                gap: '0.8rem',
                flexWrap: 'wrap',
                justifyContent: 'center',
              }}
            >
              {CLOSER.ctas.map((cta) => (
                <a
                  key={cta.href}
                  className="lewm-cta"
                  href={cta.href}
                  target="_blank"
                  rel="noopener"
                >
                  {cta.label}
                </a>
              ))}
            </div>

            <p
              className="lewm-mono"
              style={{
                marginTop: '4rem',
                color: 'var(--lewm-mute)',
                fontSize: '0.72rem',
              }}
            >
              weftos.weavelogic.ai · feature/lewm-worldmodel · 2026-04-21
            </p>
          </div>
        </section>
      </main>
    </div>
  );
}
