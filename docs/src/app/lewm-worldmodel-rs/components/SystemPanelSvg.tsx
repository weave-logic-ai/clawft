type Props = { compact?: boolean };

export default function SystemPanelSvg({ compact = false }: Props) {
  return (
    <svg
      viewBox="0 0 1200 760"
      role="img"
      aria-label="Weftos × LeWM system architecture: sensor plane emits SIGReg-manifold latents to a mesh observation wire subscribed by local ECC consumers and an optional cluster world model, all attested by ExoChain."
      style={{
        width: '100%',
        height: 'auto',
        maxHeight: compact ? '58vh' : '80vh',
        display: 'block',
      }}
    >
      <defs>
        <marker
          id="lewm-arrow"
          viewBox="0 -5 10 10"
          refX="10"
          refY="0"
          markerWidth="7"
          markerHeight="7"
          orient="auto"
        >
          <path d="M0,-4L10,0L0,4" fill="currentColor" />
        </marker>
        <pattern id="lewm-iso" width="24" height="24" patternUnits="userSpaceOnUse">
          <path d="M0 24 L24 0" stroke="var(--lewm-grid)" strokeWidth="0.5" fill="none" />
        </pattern>
      </defs>

      {/* BG grid */}
      <rect width="1200" height="760" fill="url(#lewm-iso)" opacity="0.35" />

      {/* PANEL ❶ — Sensor Plane */}
      <g>
        <rect x="40" y="60" width="340" height="260" rx="6"
          fill="var(--lewm-surface)" fillOpacity="0.95"
          stroke="var(--lewm-cyan)" strokeOpacity="0.7" strokeWidth="1" />
        <text x="62" y="90" fontFamily="var(--font-mono-lewm), monospace" fontSize="13"
          fill="var(--lewm-cyan)">❶  SENSOR PLANE</text>
        <text x="62" y="108" fontFamily="var(--font-mono-lewm), monospace" fontSize="10"
          fill="var(--lewm-mute)">RGB-D · IMU · Proprio · LiDAR</text>

        {/* Three pipeline steps */}
        {['COLLECT', 'AGGREGATE', 'ENCODE ★'].map((label, i) => (
          <g key={label}>
            <rect x={62 + i * 92} y={150} width="80" height="54" rx="3"
              fill="var(--lewm-surface-2)" stroke="var(--lewm-line)" strokeWidth="1" />
            <text x={62 + i * 92 + 40} y={172} fontFamily="var(--font-mono-lewm), monospace"
              fontSize="9" fill="var(--lewm-ink)" textAnchor="middle">{label}</text>
            <text x={62 + i * 92 + 40} y={188} fontFamily="var(--font-mono-lewm), monospace"
              fontSize="8" fill="var(--lewm-mute)" textAnchor="middle">
              {i === 0 ? 'gate' : i === 1 ? 'fusion' : 'SIGReg'}
            </text>
            {i < 2 && (
              <path d={`M${62 + i * 92 + 80},177 L${62 + (i + 1) * 92 - 2},177`}
                stroke="var(--lewm-cyan)" strokeWidth="1.25" markerEnd="url(#lewm-arrow)"
                color="var(--lewm-cyan)" fill="none" />
            )}
          </g>
        ))}

        <text x="62" y="236" fontFamily="var(--font-mono-lewm), monospace" fontSize="9"
          fill="var(--lewm-mute)">z_t ∈ ℝ¹⁹² · N(0, I) · per-frame Ed25519</text>
        <text x="62" y="260" fontFamily="var(--font-mono-lewm), monospace" fontSize="9"
          fill="var(--lewm-mute)">ADR-056 Pipeline Service · ADR-057 edge intelligence</text>

        {/* Emit arrow down */}
        <path d="M210,300 L210,380" stroke="var(--lewm-cyan)" strokeWidth="1.5"
          color="var(--lewm-cyan)" markerEnd="url(#lewm-arrow)" fill="none" />
        <text x="218" y="342" fontFamily="var(--font-mono-lewm), monospace" fontSize="9"
          fill="var(--lewm-cyan)">emit · 10 Hz</text>
      </g>

      {/* PANEL ❷ — Observation Wire (horizontal trunk) */}
      <g>
        <rect x="40" y="380" width="1120" height="150" rx="6"
          fill="var(--lewm-surface)" fillOpacity="0.95"
          stroke="var(--lewm-line)" strokeWidth="1" />
        <text x="62" y="410" fontFamily="var(--font-mono-lewm), monospace" fontSize="13"
          fill="var(--lewm-ink)">❷  OBSERVATION WIRE  ·  mesh.sensor.v1.*</text>

        {/* encoded — primary */}
        <line x1="62" y1="445" x2="1138" y2="445"
          stroke="var(--lewm-cyan)" strokeWidth="2" markerEnd="url(#lewm-arrow)"
          color="var(--lewm-cyan)" />
        <text x="70" y="438" fontFamily="var(--font-mono-lewm), monospace" fontSize="9"
          fill="var(--lewm-cyan)">encoded · primary · 10 Hz · CBOR + Ed25519</text>

        {/* consensus — conditional */}
        <line x1="1138" y1="475" x2="62" y2="475"
          stroke="var(--lewm-violet)" strokeWidth="1.25" strokeDasharray="5 4"
          markerEnd="url(#lewm-arrow)" color="var(--lewm-violet)" />
        <text x="70" y="468" fontFamily="var(--font-mono-lewm), monospace" fontSize="9"
          fill="var(--lewm-violet)">consensus · conditional · z_cluster, ẑ_{`{t+1}`}</text>

        {/* control — bidir */}
        <line x1="62" y1="505" x2="1138" y2="505"
          stroke="var(--lewm-amber)" strokeWidth="1" strokeDasharray="2 3" />
        <line x1="1138" y1="505" x2="62" y2="505"
          stroke="var(--lewm-amber)" strokeWidth="1" strokeDasharray="2 3" opacity="0" />
        <text x="70" y="500" fontFamily="var(--font-mono-lewm), monospace" fontSize="9"
          fill="var(--lewm-amber)">control · drift · surprise · plans (multi-pub)</text>

        <text x="1120" y="520" fontFamily="var(--font-mono-lewm), monospace" fontSize="8"
          fill="var(--lewm-mute)" textAnchor="end">ExoChain-indexed · ADR-053</text>
      </g>

      {/* PANEL ❸ — Local Consumers */}
      <g>
        <rect x="40" y="560" width="540" height="180" rx="6"
          fill="var(--lewm-surface)" fillOpacity="0.95"
          stroke="var(--lewm-mint)" strokeOpacity="0.7" strokeWidth="1" />
        <text x="62" y="590" fontFamily="var(--font-mono-lewm), monospace" fontSize="13"
          fill="var(--lewm-mint)">❸  LOCAL CONSUMERS  ·  authoritative</text>

        <text x="62" y="620" fontFamily="var(--font-mono-lewm), monospace" fontSize="10"
          fill="var(--lewm-ink)">hnsw_eml · ECC Causal Graph · CMVG forest</text>
        <text x="62" y="640" fontFamily="var(--font-mono-lewm), monospace" fontSize="9"
          fill="var(--lewm-mute)">StructureTag::LatentWorldModel · LatentHandle in metadata</text>

        {/* H-O-E-A strip */}
        {['HYPOTHESIZE', 'OBSERVE', 'EVALUATE', 'ADJUST'].map((s, i) => (
          <g key={s}>
            <rect x={62 + i * 118} y={670} width="104" height="30" rx="3"
              fill="var(--lewm-surface-2)" stroke="var(--lewm-mint)" strokeOpacity="0.5" />
            <text x={62 + i * 118 + 52} y={688} fontFamily="var(--font-mono-lewm), monospace"
              fontSize="8" fill="var(--lewm-mint)" textAnchor="middle">{s}</text>
          </g>
        ))}
        <text x="62" y="720" fontFamily="var(--font-mono-lewm), monospace" fontSize="8"
          fill="var(--lewm-mute)">DEMOCRITUS · 1 kHz servo · 1 ms tick (ADR-047)</text>
      </g>

      {/* PANEL ❹ — WorldModelService */}
      <g>
        <rect x="620" y="560" width="540" height="180" rx="6"
          fill="var(--lewm-surface)" fillOpacity="0.95"
          stroke="var(--lewm-violet)" strokeOpacity="0.7" strokeWidth="1"
          strokeDasharray="4 3" />
        <text x="642" y="590" fontFamily="var(--font-mono-lewm), monospace" fontSize="13"
          fill="var(--lewm-violet)">❹  WORLDMODELSERVICE  ·  optional · additive</text>

        <text x="642" y="620" fontFamily="var(--font-mono-lewm), monospace" fontSize="10"
          fill="var(--lewm-ink)">Fusion Transformer · Predictor pred_φ · LatentPlanner</text>
        <text x="642" y="640" fontFamily="var(--font-mono-lewm), monospace" fontSize="9"
          fill="var(--lewm-mute)">Exposes LatticeApi (ADR-052) · ServiceUnavailable ⇒ local fallback</text>

        <text x="642" y="690" fontFamily="var(--font-mono-lewm), monospace" fontSize="9"
          fill="var(--lewm-violet)">Single · Raft-standby · Peer-to-peer (ADR-054)</text>
        <text x="642" y="720" fontFamily="var(--font-mono-lewm), monospace" fontSize="9"
          fill="var(--lewm-violet)">CEM default · MPPI warm-start · gradient-shooting</text>
      </g>

      {/* Subscribe arrows from wire down */}
      <path d="M300,530 L300,560" stroke="var(--lewm-mint)" strokeWidth="1.25"
        color="var(--lewm-mint)" markerEnd="url(#lewm-arrow)" fill="none" />
      <path d="M900,530 L900,560" stroke="var(--lewm-violet)" strokeWidth="1.25"
        color="var(--lewm-violet)" markerEnd="url(#lewm-arrow)" fill="none" />

      {/* LatticeApi feedback */}
      <path d="M620,650 C580,650 570,640 570,630" stroke="var(--lewm-violet)" strokeWidth="1"
        strokeDasharray="3 3" fill="none" color="var(--lewm-violet)" markerEnd="url(#lewm-arrow)" />
    </svg>
  );
}
