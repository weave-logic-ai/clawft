import { Fraunces, JetBrains_Mono } from 'next/font/google';
import './lewm.css';
import { HERO, PANELS, CLOSER } from './copy';
import LatentDots from './components/LatentDots';
import HeroZoomFrame from './components/HeroZoomFrame';
import FlowingSystemDiagram from './components/FlowingSystemDiagram';
import Pullquote from './components/Pullquote';
import InversionFlip from './components/InversionFlip';
import PanelPopup from './components/PanelPopup';
import CrossViewDissolve from './components/CrossViewDissolve';
import HoeaLoop from './components/HoeaLoop';
import AdrIndex from './components/AdrIndex';
import SceneAura from './components/SceneAura';
import SensorHorizontalScroll from './components/SensorHorizontalScroll';
import DeepDive from './components/DeepDive';
import SigRegManifold from './components/SigRegManifold';
import TopologyVariants from './components/TopologyVariants';
import SurpriseTimeline from './components/SurpriseTimeline';

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
      {/* Persistent grid wash beneath everything */}
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
        {/* SCENE 1 — Hero zoom into the system */}
        <div style={{ position: 'relative' }}>
          <SceneAura variant="deep" showStars showGrid={false} showFlow />
          <HeroZoomFrame>
            <header style={{ textAlign: 'center', position: 'relative', zIndex: 1 }}>
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
        </div>

        {/* SCENE 2 — System diagram (now with packets flowing) */}
        <section
          aria-label="WeftOS × LeWM system overview"
          style={{
            position: 'relative',
            padding: '4vh 1.5rem 12vh',
            display: 'flex',
            justifyContent: 'center',
          }}
        >
          <SceneAura variant="cyan" showStars={false} />
          <div style={{ position: 'relative', maxWidth: '78rem', width: '100%' }}>
            <header
              style={{
                textAlign: 'center',
                marginBottom: '2rem',
                maxWidth: '52rem',
                margin: '0 auto 2.4rem',
              }}
            >
              <span
                className="lewm-badge"
                style={{ borderColor: 'var(--lewm-cyan)', color: 'var(--lewm-cyan)' }}
              >
                Diagram ① · architecture
              </span>
              <h2
                className="lewm-editorial"
                style={{
                  marginTop: '0.7rem',
                  fontSize: 'clamp(1.6rem, 3vw, 2.4rem)',
                  letterSpacing: '-0.015em',
                }}
              >
                Sensor plane · wire · consumers · ExoChain.
              </h2>
              <p style={{ color: 'var(--lewm-mute)', margin: '0.5rem auto 0' }}>
                One direction of flow. The sensor network is primary; the world model is one
                additive subscriber. ExoChain is the attestation spine that lets standby and peer
                services catch up from replay alone.
              </p>
            </header>
            <FlowingSystemDiagram />
          </div>
        </section>

        {/* SCENE 3 — TL;DR pullquote */}
        <div style={{ position: 'relative' }}>
          <SceneAura variant="deep" showFlow />
          <Pullquote />
        </div>

        {/* SCENE 4 — The inversion */}
        <div style={{ position: 'relative' }}>
          <SceneAura variant="amber" />
          <InversionFlip />
        </div>

        {/* SCENE 5 — Sensor horizontal scroll lab */}
        <div style={{ position: 'relative' }}>
          <SceneAura variant="cyan" showFlow showStars={false} />
          <SensorHorizontalScroll />
        </div>

        {/* SCENE 6 — Deep dive into the pipeline */}
        <div style={{ position: 'relative' }}>
          <SceneAura variant="violet" />
          <DeepDive />
        </div>

        {/* SCENE 7 — SIGReg manifold + the latent contract */}
        <section
          aria-label="SIGReg latent manifold"
          style={{
            position: 'relative',
            padding: '12vh 1.5rem',
            display: 'flex',
            justifyContent: 'center',
          }}
        >
          <SceneAura variant="mixed" showStars />
          <div
            style={{
              position: 'relative',
              display: 'grid',
              gap: '3rem',
              gridTemplateColumns: 'minmax(300px, 1fr) minmax(280px, 1fr)',
              alignItems: 'center',
              maxWidth: '72rem',
              width: '100%',
            }}
          >
            <div>
              <span className="lewm-badge" style={{ borderColor: 'var(--lewm-cyan)', color: 'var(--lewm-cyan)' }}>
                ADR-050 · the latent contract
              </span>
              <h2
                className="lewm-editorial"
                style={{
                  marginTop: '0.7rem',
                  fontSize: 'clamp(1.6rem, 3vw, 2.3rem)',
                  letterSpacing: '-0.015em',
                }}
              >
                z<sub>t</sub> ∈ ℝ¹⁹² regularized to N(0, I).
              </h2>
              <p style={{ color: 'var(--lewm-ink)', margin: '0.6rem 0', lineHeight: 1.6 }}>
                SIGReg + VICReg pull every encoder’s output toward an isotropic Gaussian. That is
                what makes the wire <em>universal</em> — any subscriber can read from any sensor
                class without bespoke decoding.
              </p>
              <p style={{ color: 'var(--lewm-mute)', margin: 0, lineHeight: 1.6 }}>
                The contract is versioned, ExoChain-attested, and degrades — never crashes. If a
                new edge model drifts off the manifold, sigreg_health drops, and the four-condition
                rollback engages within 30 seconds.
              </p>
              <ul
                className="lewm-mono"
                style={{
                  listStyle: 'none',
                  padding: 0,
                  margin: '1.4rem 0 0',
                  color: 'var(--lewm-mute)',
                  fontSize: '0.85rem',
                  display: 'grid',
                  gap: '0.4rem',
                }}
              >
                <li>· VICReg variance term · per-dim σ ≥ 1</li>
                <li>· VICReg covariance · off-diagonal → 0</li>
                <li>· KL to N(0, I) · soft prior</li>
                <li>· temporal-straightening · z<sub>t</sub> → z<sub>t+1</sub> linear</li>
              </ul>
            </div>
            <div className="lewm-panel" style={{ padding: '1rem', borderColor: 'var(--lewm-cyan)' }}>
              <SigRegManifold />
            </div>
          </div>
        </section>

        {/* SCENE 8 — Surprise + health timeline */}
        <div style={{ position: 'relative' }}>
          <SceneAura variant="amber" showStars={false} showFlow />
          <SurpriseTimeline />
        </div>

        {/* SCENE 9 — Topology variants (deployment shapes) */}
        <div style={{ position: 'relative' }}>
          <SceneAura variant="violet" />
          <TopologyVariants />
        </div>

        {/* SCENE 10 — Per-component panels (existing pop-up book) */}
        <div style={{ position: 'relative' }}>
          <SceneAura variant="mixed" />
          <div style={{ padding: '4vh 0', position: 'relative' }}>
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
        </div>

        {/* SCENE 11 — Cross-view dissolve into the H-O-E-A cycle */}
        <div style={{ position: 'relative' }}>
          <SceneAura variant="mint" />
          <CrossViewDissolve />
        </div>

        {/* SCENE 12 — H-O-E-A loop */}
        <div style={{ position: 'relative' }}>
          <SceneAura variant="deep" showStars />
          <HoeaLoop />
        </div>

        {/* SCENE 13 — ADR index */}
        <div style={{ position: 'relative' }}>
          <SceneAura variant="cyan" />
          <AdrIndex />
        </div>

        {/* SCENE 14 — Closer */}
        <section
          aria-label="Closer"
          style={{
            position: 'relative',
            padding: '8vh 1.5rem 18vh',
            display: 'flex',
            justifyContent: 'center',
            textAlign: 'center',
          }}
        >
          <SceneAura variant="deep" showStars showFlow />
          <div style={{ position: 'relative', maxWidth: '52rem' }}>
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
              weftos.weavelogic.ai · feature/lewm-worldmodel · 2026-04-22
            </p>
          </div>
        </section>
      </main>
    </div>
  );
}
