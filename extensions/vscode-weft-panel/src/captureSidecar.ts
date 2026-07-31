// WEFT-282 — Capture sidecar / host bridge for the VSCode panel.
//
// Problem: VS Code webviews cannot grant `allow="microphone"` /
// `getUserMedia` (microsoft/vscode#303293). Voice input therefore
// cannot run inside the webview sandbox.
//
// Solution: capture on the **extension host** (Node process) and ship
// PCM / status to the webview over the existing postMessage channel.
// This is the "sidecar or host bridge" required by WEFT-282.
//
// Architecture (see docs/architecture/vscode-panel-capture-sidecar.md):
//
//   Webview  --capture-request-->  CaptureController (host)
//            <--capture-response--
//            <--capture-chunk-----  (optional push)
//                    |
//                    v
//            CaptureBackend
//              · mock        — synthetic PCM (tests / CI)
//              · process     — spawn WEFT_CAPTURE_SIDECAR / sox / arecord
//              · unavailable — graceful degradation (default)
//
// Pure TypeScript — no `vscode` import — so unit tests run under
// plain node:test / tsx without a host.

import { spawn, type ChildProcess } from "node:child_process";
import { Buffer } from "node:buffer";

// ── Wire discriminators ────────────────────────────────────────────

export const CAPTURE_REQUEST_TYPE = "capture-request" as const;
export const CAPTURE_RESPONSE_TYPE = "capture-response" as const;
export const CAPTURE_CHUNK_TYPE = "capture-chunk" as const;

/** Methods the webview may call on the host capture bridge. */
export const CAPTURE_METHODS = [
    "status",
    "list_devices",
    "start",
    "stop",
    "poll",
    "test_level",
    "grant_consent",
    "revoke_consent",
] as const;

export type CaptureMethod = (typeof CAPTURE_METHODS)[number];

export type CaptureMedia = "mic" | "camera";

export type CaptureBackendKind = "mock" | "process" | "unavailable";

// ── Consent scopes ─────────────────────────────────────────────────

/** WSP-aligned consent scopes for capture devices. */
export const MIC_CONSENT_SCOPE = "scope://mic";
export const CAMERA_CONSENT_SCOPE = "scope://camera";

export function isCaptureConsentScope(scope: string): boolean {
    const s = scope.trim().toLowerCase();
    return (
        s === MIC_CONSENT_SCOPE ||
        s === CAMERA_CONSENT_SCOPE ||
        s === "scope://microphone" ||
        s === "mic" ||
        s === "camera" ||
        s === "microphone"
    );
}

export function mediaFromConsentScope(scope: string): CaptureMedia | undefined {
    const s = scope.trim().toLowerCase();
    if (
        s === MIC_CONSENT_SCOPE ||
        s === "scope://microphone" ||
        s === "mic" ||
        s === "microphone"
    ) {
        return "mic";
    }
    if (s === CAMERA_CONSENT_SCOPE || s === "camera") {
        return "camera";
    }
    return undefined;
}

// ── Types ──────────────────────────────────────────────────────────

export interface CaptureDeviceInfo {
    id: string;
    name: string;
    media: CaptureMedia;
    is_default: boolean;
}

export interface CaptureMediaStatus {
    media: CaptureMedia;
    available: boolean;
    capturing: boolean;
    backend: CaptureBackendKind;
    reason?: string;
    sample_rate?: number;
    device?: string;
    notes?: string;
}

export interface CaptureStatus {
    /** Overall host bridge present (always true once module loads). */
    bridge: "host";
    /** Upstream issue that forces host-side capture. */
    upstream: "microsoft/vscode#303293";
    mic: CaptureMediaStatus;
    camera: CaptureMediaStatus;
    consents: {
        mic: boolean;
        camera: boolean;
    };
}

export interface CaptureSessionInfo {
    media: CaptureMedia;
    device: string;
    sample_rate: number;
    channels: number;
    backend: CaptureBackendKind;
    started_at: number;
}

export interface CapturePollResult {
    /** Mono PCM as signed 16-bit samples (host endian). */
    pcm_i16: number[];
    sample_rate: number;
    channels: number;
    /** Peak absolute level in [0, 1] for this chunk. */
    peak: number;
    capturing: boolean;
    media: CaptureMedia;
    /** Base64 of little-endian s16le bytes — whisper wire helper. */
    pcm_b64?: string;
    samples?: number;
}

export interface CaptureLevelReport {
    level: number;
    device: string;
    sample_rate: number;
    backend: CaptureBackendKind;
    media: CaptureMedia;
}

export type CaptureResult =
    | { ok: true; result: unknown }
    | { ok: false; error: string; error_code?: string };

export interface CaptureRequestMessage {
    type: typeof CAPTURE_REQUEST_TYPE;
    id: number | string;
    method: CaptureMethod | string;
    params?: unknown;
}

export interface CaptureResponseMessage {
    type: typeof CAPTURE_RESPONSE_TYPE;
    id: number | string;
    method: string;
    ok: boolean;
    result?: unknown;
    error?: string;
    error_code?: string;
}

export interface CaptureChunkMessage {
    type: typeof CAPTURE_CHUNK_TYPE;
    media: CaptureMedia;
    sample_rate: number;
    channels: number;
    peak: number;
    /** Optional binary payload as base64 s16le. */
    pcm_b64: string;
    seq: number;
    samples: number;
}

// ── Whisper wire helper ────────────────────────────────────────────

/**
 * Shape expected by clawft-service-whisper `PcmChunk` (data / pcm_b64).
 * Used by smoke paths that publish host-captured PCM onto substrate.
 */
export interface WhisperPcmChunk {
    data: string;
    sample_rate: number;
    channels: number;
    seq: number;
    samples: number;
    chunk_ms: number;
}

/** Encode mono i16 samples as little-endian base64. */
export function encodePcmI16Base64(pcm: readonly number[]): string {
    const buf = Buffer.allocUnsafe(pcm.length * 2);
    for (let i = 0; i < pcm.length; i++) {
        let s = pcm[i] ?? 0;
        if (s > 32767) s = 32767;
        if (s < -32768) s = -32768;
        buf.writeInt16LE(s, i * 2);
    }
    return buf.toString("base64");
}

/** Decode base64 s16le into number[] (for tests / process backends). */
export function decodePcmI16Base64(b64: string): number[] {
    const buf = Buffer.from(b64, "base64");
    const out: number[] = [];
    for (let i = 0; i + 1 < buf.length; i += 2) {
        out.push(buf.readInt16LE(i));
    }
    return out;
}

export function peakLevelI16(pcm: readonly number[]): number {
    let peak = 0;
    for (const s of pcm) {
        const a = Math.abs(s) / 32768;
        if (a > peak) peak = a;
    }
    return peak > 1 ? 1 : peak;
}

/** Build a whisper-compatible PCM chunk envelope from a poll result. */
export function toWhisperPcmChunk(
    poll: CapturePollResult,
    seq = 0,
): WhisperPcmChunk {
    const samples = poll.pcm_i16.length;
    const sr = poll.sample_rate || 16_000;
    const chunk_ms =
        samples > 0 && sr > 0 ? Math.round((samples * 1000) / sr) : 0;
    return {
        data: poll.pcm_b64 ?? encodePcmI16Base64(poll.pcm_i16),
        sample_rate: sr,
        channels: poll.channels || 1,
        seq,
        samples,
        chunk_ms,
    };
}

// ── Backend interface ──────────────────────────────────────────────

export interface CaptureBackend {
    readonly kind: CaptureBackendKind;
    /** Human-readable notes (privacy / platform). */
    notes(): string;
    listDevices(media: CaptureMedia): Promise<CaptureDeviceInfo[]>;
    /**
     * Start continuous capture. Returns session metadata or throws
     * a string error (unavailable / permission / device).
     */
    start(
        media: CaptureMedia,
        device?: string,
    ): Promise<CaptureSessionInfo>;
    stop(): Promise<boolean>;
    poll(): CapturePollResult;
    /** Short probe (~durationMs). Default implementations may synthetic. */
    testLevel(
        media: CaptureMedia,
        device?: string,
        durationMs?: number,
    ): Promise<CaptureLevelReport>;
    isCapturing(): boolean;
    currentSession(): CaptureSessionInfo | undefined;
}

// ── Unavailable backend ────────────────────────────────────────────

export class UnavailableCaptureBackend implements CaptureBackend {
    readonly kind: CaptureBackendKind = "unavailable";
    private readonly reason: string;

    constructor(
        reason = "No capture sidecar binary configured (set WEFT_CAPTURE_SIDECAR or WEFT_CAPTURE_BACKEND=mock)",
    ) {
        this.reason = reason;
    }

    notes(): string {
        return this.reason;
    }

    async listDevices(_media: CaptureMedia): Promise<CaptureDeviceInfo[]> {
        return [];
    }

    async start(
        media: CaptureMedia,
        _device?: string,
    ): Promise<CaptureSessionInfo> {
        throw unavailableError(media, this.reason);
    }

    async stop(): Promise<boolean> {
        return false;
    }

    poll(): CapturePollResult {
        return emptyPoll("mic");
    }

    async testLevel(
        media: CaptureMedia,
        _device?: string,
        _durationMs?: number,
    ): Promise<CaptureLevelReport> {
        throw unavailableError(media, this.reason);
    }

    isCapturing(): boolean {
        return false;
    }

    currentSession(): CaptureSessionInfo | undefined {
        return undefined;
    }
}

function unavailableError(media: CaptureMedia, reason: string): string {
    return `capture.${media} unavailable: ${reason}`;
}

function emptyPoll(media: CaptureMedia): CapturePollResult {
    return {
        pcm_i16: [],
        sample_rate: 0,
        channels: 1,
        peak: 0,
        capturing: false,
        media,
    };
}

// ── Mock backend (tests / CI / smoke without hardware) ─────────────

/**
 * Synthetic mono PCM generator. Produces a 440 Hz sine at 16 kHz so
 * peak levels and whisper wire helpers are exercisable without a mic.
 */
export class MockCaptureBackend implements CaptureBackend {
    readonly kind: CaptureBackendKind = "mock";
    private session: CaptureSessionInfo | undefined;
    private phase = 0;
    private readonly sampleRate: number;
    private readonly freqHz: number;

    constructor(opts?: { sampleRate?: number; freqHz?: number }) {
        this.sampleRate = opts?.sampleRate ?? 16_000;
        this.freqHz = opts?.freqHz ?? 440;
    }

    notes(): string {
        return "Mock capture backend (synthetic 440 Hz sine @ 16 kHz). Not real hardware.";
    }

    async listDevices(media: CaptureMedia): Promise<CaptureDeviceInfo[]> {
        if (media === "camera") {
            // Camera remains deliberately unavailable in MVP.
            return [];
        }
        return [
            {
                id: "mock-default",
                name: "Mock Default Microphone",
                media: "mic",
                is_default: true,
            },
        ];
    }

    async start(
        media: CaptureMedia,
        device?: string,
    ): Promise<CaptureSessionInfo> {
        if (media === "camera") {
            throw unavailableError(
                "camera",
                "Camera capture not implemented in M2 MVP (mic only)",
            );
        }
        this.session = {
            media: "mic",
            device: device ?? "Mock Default Microphone",
            sample_rate: this.sampleRate,
            channels: 1,
            backend: "mock",
            started_at: Date.now(),
        };
        this.phase = 0;
        return this.session;
    }

    async stop(): Promise<boolean> {
        const was = !!this.session;
        this.session = undefined;
        return was;
    }

    poll(): CapturePollResult {
        if (!this.session) {
            return emptyPoll("mic");
        }
        // ~100 ms of audio per poll.
        const n = Math.floor(this.sampleRate / 10);
        const pcm: number[] = new Array(n);
        const omega = (2 * Math.PI * this.freqHz) / this.sampleRate;
        for (let i = 0; i < n; i++) {
            const sample = Math.sin(this.phase) * 0.25;
            this.phase += omega;
            pcm[i] = Math.round(sample * 32767);
        }
        const peak = peakLevelI16(pcm);
        return {
            pcm_i16: pcm,
            sample_rate: this.sampleRate,
            channels: 1,
            peak,
            capturing: true,
            media: "mic",
            pcm_b64: encodePcmI16Base64(pcm),
            samples: n,
        };
    }

    async testLevel(
        media: CaptureMedia,
        device?: string,
        _durationMs?: number,
    ): Promise<CaptureLevelReport> {
        if (media === "camera") {
            throw unavailableError(
                "camera",
                "Camera capture not implemented in M2 MVP (mic only)",
            );
        }
        // One synthetic poll peak without leaving a long session open.
        const started = !this.session;
        if (started) {
            await this.start("mic", device);
        }
        const p = this.poll();
        if (started) {
            await this.stop();
        }
        return {
            level: p.peak,
            device: device ?? "Mock Default Microphone",
            sample_rate: this.sampleRate,
            backend: "mock",
            media: "mic",
        };
    }

    isCapturing(): boolean {
        return !!this.session;
    }

    currentSession(): CaptureSessionInfo | undefined {
        return this.session;
    }
}

// ── Process backend (external sidecar binary) ──────────────────────

/**
 * Spawns an external capture process that writes raw mono s16le PCM
 * to stdout. Configure with:
 *
 *   WEFT_CAPTURE_SIDECAR="/path/to/bin --args"
 *   # or absolute path; media/device appended as env when supported
 *
 * Expected output: continuous little-endian signed 16-bit mono PCM
 * at WEFT_CAPTURE_SAMPLE_RATE (default 16000).
 *
 * When the process exits or fails to spawn, start() rejects with a
 * graceful unavailable error — the panel stays usable without mic.
 */
export class ProcessCaptureBackend implements CaptureBackend {
    readonly kind: CaptureBackendKind = "process";
    private readonly command: string;
    private readonly sampleRate: number;
    private child: ChildProcess | undefined;
    private session: CaptureSessionInfo | undefined;
    private buffer: number[] = [];
    private readonly maxSamples: number;
    private stderrTail = "";

    constructor(command: string, sampleRate = 16_000) {
        this.command = command;
        this.sampleRate = sampleRate;
        this.maxSamples = sampleRate * 2; // ~2 s ring
    }

    notes(): string {
        return `Process capture sidecar: ${this.command}`;
    }

    async listDevices(media: CaptureMedia): Promise<CaptureDeviceInfo[]> {
        if (media === "camera") return [];
        return [
            {
                id: "process-default",
                name: `Sidecar (${this.command.split(/\s+/)[0] ?? "process"})`,
                media: "mic",
                is_default: true,
            },
        ];
    }

    async start(
        media: CaptureMedia,
        device?: string,
    ): Promise<CaptureSessionInfo> {
        if (media === "camera") {
            throw unavailableError(
                "camera",
                "Camera capture not implemented in M2 MVP (mic only)",
            );
        }
        await this.stop();

        const parts = splitCommand(this.command);
        if (parts.length === 0) {
            throw unavailableError("mic", "empty WEFT_CAPTURE_SIDECAR command");
        }
        const [bin, ...args] = parts;
        let child: ChildProcess;
        try {
            child = spawn(bin!, args, {
                stdio: ["ignore", "pipe", "pipe"],
                env: {
                    ...process.env,
                    WEFT_CAPTURE_MEDIA: "mic",
                    WEFT_CAPTURE_DEVICE: device ?? "",
                    WEFT_CAPTURE_SAMPLE_RATE: String(this.sampleRate),
                },
            });
        } catch (err) {
            throw unavailableError(
                "mic",
                `failed to spawn capture sidecar: ${String(err)}`,
            );
        }

        this.child = child;
        this.buffer = [];
        this.stderrTail = "";

        child.stdout?.on("data", (chunk: Buffer) => {
            for (let i = 0; i + 1 < chunk.length; i += 2) {
                this.buffer.push(chunk.readInt16LE(i));
            }
            if (this.buffer.length > this.maxSamples) {
                this.buffer.splice(0, this.buffer.length - this.maxSamples);
            }
        });
        child.stderr?.on("data", (chunk: Buffer) => {
            this.stderrTail = (this.stderrTail + chunk.toString("utf8")).slice(
                -500,
            );
        });
        child.on("error", (err: Error) => {
            this.stderrTail = String(err);
            this.child = undefined;
            this.session = undefined;
        });
        child.on("exit", () => {
            this.child = undefined;
            // Keep last session metadata until stop() so poll can report.
        });

        // Brief settle — if the process dies immediately, surface stderr.
        await delay(50);
        if (!this.child || this.child.killed || this.child.exitCode !== null) {
            const reason =
                this.stderrTail.trim() ||
                "capture sidecar exited immediately";
            this.child = undefined;
            throw unavailableError("mic", reason);
        }

        this.session = {
            media: "mic",
            device: device ?? "process-default",
            sample_rate: this.sampleRate,
            channels: 1,
            backend: "process",
            started_at: Date.now(),
        };
        return this.session;
    }

    async stop(): Promise<boolean> {
        const was = !!this.session || !!this.child;
        if (this.child) {
            try {
                this.child.kill("SIGTERM");
            } catch {
                /* ignore */
            }
            this.child = undefined;
        }
        this.session = undefined;
        this.buffer = [];
        return was;
    }

    poll(): CapturePollResult {
        if (!this.session) {
            return emptyPoll("mic");
        }
        const pcm = this.buffer.splice(0, this.buffer.length);
        const peak = peakLevelI16(pcm);
        return {
            pcm_i16: pcm,
            sample_rate: this.sampleRate,
            channels: 1,
            peak,
            capturing: !!this.child,
            media: "mic",
            pcm_b64: encodePcmI16Base64(pcm),
            samples: pcm.length,
        };
    }

    async testLevel(
        media: CaptureMedia,
        device?: string,
        durationMs = 300,
    ): Promise<CaptureLevelReport> {
        await this.start(media, device);
        await delay(Math.min(Math.max(durationMs, 50), 2000));
        const p = this.poll();
        await this.stop();
        return {
            level: p.peak,
            device: device ?? "process-default",
            sample_rate: this.sampleRate,
            backend: "process",
            media: "mic",
        };
    }

    isCapturing(): boolean {
        return !!this.session && !!this.child;
    }

    currentSession(): CaptureSessionInfo | undefined {
        return this.session;
    }
}

function splitCommand(cmd: string): string[] {
    // Minimal shell-ish split: respects double quotes.
    const out: string[] = [];
    const re = /"([^"]*)"|'([^']*)'|(\S+)/g;
    let m: RegExpExecArray | null;
    while ((m = re.exec(cmd)) !== null) {
        out.push(m[1] ?? m[2] ?? m[3] ?? "");
    }
    return out.filter((s) => s.length > 0);
}

function delay(ms: number): Promise<void> {
    return new Promise((r) => setTimeout(r, ms));
}

// ── Backend resolution ─────────────────────────────────────────────

export interface ResolveBackendOptions {
    /** Override env map (tests). Defaults to process.env. */
    env?: NodeJS.ProcessEnv;
    /** Inject a backend (tests). */
    force?: CaptureBackend;
}

/**
 * Resolve the capture backend.
 *
 * Priority:
 *   1. `force` inject (tests)
 *   2. WEFT_CAPTURE_BACKEND=mock|process|unavailable
 *   3. WEFT_CAPTURE_SIDECAR set → process
 *   4. unavailable (graceful default — panel still loads)
 */
export function resolveCaptureBackend(
    opts: ResolveBackendOptions = {},
): CaptureBackend {
    if (opts.force) return opts.force;
    const env = opts.env ?? process.env;
    const kind = (env.WEFT_CAPTURE_BACKEND ?? "").trim().toLowerCase();
    const sidecar = (env.WEFT_CAPTURE_SIDECAR ?? "").trim();
    const sr = parseInt(env.WEFT_CAPTURE_SAMPLE_RATE ?? "16000", 10);
    const sampleRate = Number.isFinite(sr) && sr > 0 ? sr : 16_000;

    if (kind === "mock") {
        return new MockCaptureBackend({ sampleRate });
    }
    if (kind === "unavailable") {
        return new UnavailableCaptureBackend();
    }
    if (kind === "process" || sidecar) {
        if (!sidecar) {
            return new UnavailableCaptureBackend(
                "WEFT_CAPTURE_BACKEND=process but WEFT_CAPTURE_SIDECAR is empty",
            );
        }
        return new ProcessCaptureBackend(sidecar, sampleRate);
    }
    // auto / unset → unavailable with a clear reason (no surprise mic open).
    return new UnavailableCaptureBackend(
        "Capture sidecar not configured. Set WEFT_CAPTURE_BACKEND=mock for synthetic PCM, " +
            "or WEFT_CAPTURE_SIDECAR=/path/to/capture-bin for host mic. " +
            "Webview getUserMedia is blocked (vscode#303293).",
    );
}

// ── Controller ─────────────────────────────────────────────────────

/**
 * Host-side capture controller. Owns consent state + one active backend
 * session. Dispatches `capture-request` methods.
 */
export class CaptureController {
    private backend: CaptureBackend;
    private micConsent = false;
    private cameraConsent = false;
    private seq = 0;

    constructor(backend?: CaptureBackend) {
        this.backend = backend ?? resolveCaptureBackend();
    }

    /** Swap backend (tests / runtime reconfigure). Stops active capture. */
    async setBackend(backend: CaptureBackend): Promise<void> {
        await this.backend.stop();
        this.backend = backend;
    }

    getBackend(): CaptureBackend {
        return this.backend;
    }

    grantConsent(scope: string): boolean {
        const media = mediaFromConsentScope(scope);
        if (!media) return false;
        if (media === "mic") this.micConsent = true;
        if (media === "camera") this.cameraConsent = true;
        return true;
    }

    revokeConsent(scope?: string): void {
        if (!scope) {
            this.micConsent = false;
            this.cameraConsent = false;
            return;
        }
        const media = mediaFromConsentScope(scope);
        if (media === "mic") this.micConsent = false;
        if (media === "camera") this.cameraConsent = false;
    }

    hasConsent(media: CaptureMedia): boolean {
        return media === "mic" ? this.micConsent : this.cameraConsent;
    }

    status(): CaptureStatus {
        const session = this.backend.currentSession();
        const capturing = this.backend.isCapturing();
        const notes = this.backend.notes();
        const micAvailable = this.backend.kind !== "unavailable";
        return {
            bridge: "host",
            upstream: "microsoft/vscode#303293",
            mic: {
                media: "mic",
                available: micAvailable,
                capturing: capturing && session?.media === "mic",
                backend: this.backend.kind,
                reason: micAvailable ? undefined : notes,
                sample_rate: session?.sample_rate,
                device: session?.device,
                notes,
            },
            camera: {
                media: "camera",
                available: false,
                capturing: false,
                backend: this.backend.kind,
                reason:
                    "Camera capture not implemented in M2 MVP (mic path only)",
                notes: "Camera reserved; graceful unavailable.",
            },
            consents: {
                mic: this.micConsent,
                camera: this.cameraConsent,
            },
        };
    }

    /**
     * Daemon-shaped `sensor.mic.status` payload so the existing
     * resource://sensor/mic/status mapping works without a daemon verb.
     */
    sensorMicStatus(): Record<string, unknown> {
        const s = this.status();
        return {
            available: s.mic.available,
            capturing: s.mic.capturing,
            backend: s.mic.backend,
            bridge: s.bridge,
            upstream: s.upstream,
            reason: s.mic.reason,
            sample_rate: s.mic.sample_rate ?? null,
            device: s.mic.device ?? null,
            consent: s.consents.mic,
            notes: s.mic.notes,
        };
    }

    async dispatch(
        method: string,
        params?: unknown,
    ): Promise<CaptureResult> {
        try {
            switch (method) {
                case "status":
                    return { ok: true, result: this.status() };
                case "list_devices": {
                    const p = asRecord(params);
                    const media = parseMedia(p.media) ?? "mic";
                    const devices = await this.backend.listDevices(media);
                    return { ok: true, result: { devices, media } };
                }
                case "start": {
                    const p = asRecord(params);
                    const media = parseMedia(p.media) ?? "mic";
                    if (!this.hasConsent(media)) {
                        return {
                            ok: false,
                            error: `consent required for ${media} (scope://${media}) — call consent.request first`,
                            error_code: "consent_required",
                        };
                    }
                    if (media === "camera") {
                        return {
                            ok: false,
                            error: unavailableError(
                                "camera",
                                "Camera capture not implemented in M2 MVP (mic only)",
                            ),
                            error_code: "unavailable",
                        };
                    }
                    const device =
                        typeof p.device === "string" ? p.device : undefined;
                    try {
                        const info = await this.backend.start(media, device);
                        this.seq = 0;
                        return { ok: true, result: info };
                    } catch (err) {
                        return {
                            ok: false,
                            error: String(err),
                            error_code: "unavailable",
                        };
                    }
                }
                case "stop": {
                    const was = await this.backend.stop();
                    return { ok: true, result: { stopped: was } };
                }
                case "poll": {
                    const poll = this.backend.poll();
                    if (poll.capturing && poll.pcm_i16.length > 0) {
                        this.seq += 1;
                    }
                    // Attach whisper-ready fields when samples present.
                    if (poll.pcm_i16.length > 0 && !poll.pcm_b64) {
                        poll.pcm_b64 = encodePcmI16Base64(poll.pcm_i16);
                        poll.samples = poll.pcm_i16.length;
                    }
                    return {
                        ok: true,
                        result: {
                            ...poll,
                            seq: this.seq,
                            whisper: poll.capturing
                                ? toWhisperPcmChunk(poll, this.seq)
                                : null,
                        },
                    };
                }
                case "test_level": {
                    const p = asRecord(params);
                    const media = parseMedia(p.media) ?? "mic";
                    if (!this.hasConsent(media)) {
                        return {
                            ok: false,
                            error: `consent required for ${media}`,
                            error_code: "consent_required",
                        };
                    }
                    const device =
                        typeof p.device === "string" ? p.device : undefined;
                    const durationMs =
                        typeof p.duration_ms === "number"
                            ? p.duration_ms
                            : typeof p.durationMs === "number"
                              ? p.durationMs
                              : 300;
                    try {
                        const report = await this.backend.testLevel(
                            media,
                            device,
                            durationMs,
                        );
                        return { ok: true, result: report };
                    } catch (err) {
                        return {
                            ok: false,
                            error: String(err),
                            error_code: "unavailable",
                        };
                    }
                }
                case "grant_consent": {
                    const p = asRecord(params);
                    const scope =
                        typeof p.scope === "string"
                            ? p.scope
                            : typeof p.media === "string"
                              ? `scope://${p.media}`
                              : "";
                    if (!scope || !isCaptureConsentScope(scope)) {
                        return {
                            ok: false,
                            error: "grant_consent requires scope scope://mic or scope://camera",
                            error_code: "invalid_params",
                        };
                    }
                    this.grantConsent(scope);
                    return {
                        ok: true,
                        result: {
                            granted: true,
                            scope,
                            consents: {
                                mic: this.micConsent,
                                camera: this.cameraConsent,
                            },
                        },
                    };
                }
                case "revoke_consent": {
                    const p = asRecord(params);
                    const scope =
                        typeof p.scope === "string" ? p.scope : undefined;
                    this.revokeConsent(scope);
                    if (scope) {
                        const media = mediaFromConsentScope(scope);
                        if (media && this.backend.currentSession()?.media === media) {
                            await this.backend.stop();
                        }
                    } else {
                        await this.backend.stop();
                    }
                    return {
                        ok: true,
                        result: {
                            revoked: true,
                            consents: {
                                mic: this.micConsent,
                                camera: this.cameraConsent,
                            },
                        },
                    };
                }
                default:
                    return {
                        ok: false,
                        error: `unknown capture method: ${method}`,
                        error_code: "method_not_found",
                    };
            }
        } catch (err) {
            return { ok: false, error: String(err), error_code: "internal" };
        }
    }

    /** Build a wire response envelope for postMessage. */
    buildResponse(
        id: number | string,
        method: string,
        result: CaptureResult,
    ): CaptureResponseMessage {
        if (result.ok) {
            return {
                type: CAPTURE_RESPONSE_TYPE,
                id,
                method,
                ok: true,
                result: result.result,
            };
        }
        return {
            type: CAPTURE_RESPONSE_TYPE,
            id,
            method,
            ok: false,
            error: result.error,
            error_code: result.error_code,
        };
    }

    /** Dispose: stop capture, drop consent. */
    async dispose(): Promise<void> {
        await this.backend.stop();
        this.micConsent = false;
        this.cameraConsent = false;
    }
}

// ── Parsing helpers ────────────────────────────────────────────────

function asRecord(v: unknown): Record<string, unknown> {
    if (v && typeof v === "object" && !Array.isArray(v)) {
        return v as Record<string, unknown>;
    }
    return {};
}

function parseMedia(v: unknown): CaptureMedia | undefined {
    if (v === "mic" || v === "microphone" || v === "audio") return "mic";
    if (v === "camera" || v === "video" || v === "cam") return "camera";
    return undefined;
}

/** Type guard for inbound webview messages. */
export function isCaptureRequest(msg: unknown): msg is CaptureRequestMessage {
    if (!msg || typeof msg !== "object") return false;
    const m = msg as Record<string, unknown>;
    return (
        m.type === CAPTURE_REQUEST_TYPE &&
        (typeof m.id === "string" || typeof m.id === "number") &&
        typeof m.method === "string"
    );
}
