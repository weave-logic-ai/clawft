// Unit tests for WEFT-282 capture sidecar / host bridge.
//
// Runnable with:
//   npx --yes tsx --test src/captureSidecar.test.ts
// from extensions/vscode-weft-panel (no VSCode host, no real mic).

import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
    CAPTURE_REQUEST_TYPE,
    CAPTURE_RESPONSE_TYPE,
    MIC_CONSENT_SCOPE,
    CAMERA_CONSENT_SCOPE,
    CaptureController,
    MockCaptureBackend,
    UnavailableCaptureBackend,
    decodePcmI16Base64,
    encodePcmI16Base64,
    isCaptureConsentScope,
    isCaptureRequest,
    mediaFromConsentScope,
    peakLevelI16,
    resolveCaptureBackend,
    toWhisperPcmChunk,
} from "./captureSidecar";

describe("consent scope helpers", () => {
    it("recognizes mic and camera scopes", () => {
        assert.equal(isCaptureConsentScope("scope://mic"), true);
        assert.equal(isCaptureConsentScope("scope://camera"), true);
        assert.equal(isCaptureConsentScope("scope://microphone"), true);
        assert.equal(isCaptureConsentScope("mic"), true);
        assert.equal(isCaptureConsentScope("scope://unrelated"), false);
    });

    it("maps scopes to media", () => {
        assert.equal(mediaFromConsentScope(MIC_CONSENT_SCOPE), "mic");
        assert.equal(mediaFromConsentScope(CAMERA_CONSENT_SCOPE), "camera");
        assert.equal(mediaFromConsentScope("scope://other"), undefined);
    });
});

describe("PCM helpers", () => {
    it("roundtrips i16 ↔ base64", () => {
        const samples = [0, 1000, -1000, 32767, -32768];
        const b64 = encodePcmI16Base64(samples);
        assert.equal(decodePcmI16Base64(b64).join(","), samples.join(","));
    });

    it("computes peak in [0,1]", () => {
        assert.equal(peakLevelI16([0, 0]), 0);
        assert.ok(peakLevelI16([16384]) > 0.4);
        assert.ok(peakLevelI16([16384]) < 0.6);
        assert.equal(peakLevelI16([32767]), 32767 / 32768);
    });

    it("builds whisper PcmChunk envelope", () => {
        const pcm = [100, -100, 200];
        const chunk = toWhisperPcmChunk(
            {
                pcm_i16: pcm,
                sample_rate: 16_000,
                channels: 1,
                peak: 0.1,
                capturing: true,
                media: "mic",
            },
            7,
        );
        assert.equal(chunk.sample_rate, 16_000);
        assert.equal(chunk.channels, 1);
        assert.equal(chunk.seq, 7);
        assert.equal(chunk.samples, 3);
        assert.ok(chunk.data.length > 0);
        assert.equal(decodePcmI16Base64(chunk.data).length, 3);
    });
});

describe("resolveCaptureBackend", () => {
    it("defaults to unavailable", () => {
        const b = resolveCaptureBackend({ env: {} });
        assert.equal(b.kind, "unavailable");
    });

    it("selects mock when env set", () => {
        const b = resolveCaptureBackend({
            env: { WEFT_CAPTURE_BACKEND: "mock" },
        });
        assert.equal(b.kind, "mock");
    });

    it("selects process when sidecar path set", () => {
        const b = resolveCaptureBackend({
            env: { WEFT_CAPTURE_SIDECAR: "/usr/bin/false" },
        });
        assert.equal(b.kind, "process");
    });

    it("honors force inject", () => {
        const mock = new MockCaptureBackend();
        const b = resolveCaptureBackend({ force: mock, env: {} });
        assert.equal(b, mock);
    });
});

describe("UnavailableCaptureBackend", () => {
    it("start fails gracefully", async () => {
        const b = new UnavailableCaptureBackend("no hardware");
        await assert.rejects(
            () => b.start("mic"),
            (err: unknown) => String(err).includes("unavailable"),
        );
        const poll = b.poll();
        assert.equal(poll.capturing, false);
        assert.equal(poll.pcm_i16.length, 0);
    });
});

describe("MockCaptureBackend", () => {
    it("lists a default mic device", async () => {
        const b = new MockCaptureBackend();
        const devices = await b.listDevices("mic");
        assert.equal(devices.length, 1);
        assert.equal(devices[0]!.is_default, true);
        assert.equal((await b.listDevices("camera")).length, 0);
    });

    it("produces non-empty PCM after start", async () => {
        const b = new MockCaptureBackend({ sampleRate: 16_000 });
        const info = await b.start("mic");
        assert.equal(info.backend, "mock");
        assert.equal(info.sample_rate, 16_000);
        const poll = b.poll();
        assert.equal(poll.capturing, true);
        assert.ok(poll.pcm_i16.length > 0);
        assert.ok(poll.peak > 0);
        assert.ok(poll.pcm_b64);
        await b.stop();
        assert.equal(b.isCapturing(), false);
    });

    it("rejects camera start", async () => {
        const b = new MockCaptureBackend();
        await assert.rejects(
            () => b.start("camera"),
            (err: unknown) => String(err).includes("camera"),
        );
    });

    it("testLevel returns a positive peak", async () => {
        const b = new MockCaptureBackend();
        const r = await b.testLevel("mic");
        assert.ok(r.level > 0);
        assert.equal(b.isCapturing(), false);
    });
});

describe("CaptureController", () => {
    it("status reports bridge + upstream issue", async () => {
        const c = new CaptureController(new MockCaptureBackend());
        const st = c.status();
        assert.equal(st.bridge, "host");
        assert.equal(st.upstream, "microsoft/vscode#303293");
        assert.equal(st.mic.available, true);
        assert.equal(st.camera.available, false);
        assert.equal(st.consents.mic, false);
    });

    it("start requires consent", async () => {
        const c = new CaptureController(new MockCaptureBackend());
        const r = await c.dispatch("start", { media: "mic" });
        assert.equal(r.ok, false);
        if (!r.ok) {
            assert.equal(r.error_code, "consent_required");
        }
    });

    it("start/poll/stop happy path after consent", async () => {
        const c = new CaptureController(new MockCaptureBackend());
        const g = await c.dispatch("grant_consent", {
            scope: MIC_CONSENT_SCOPE,
        });
        assert.equal(g.ok, true);

        const start = await c.dispatch("start", { media: "mic" });
        assert.equal(start.ok, true);

        const poll = await c.dispatch("poll");
        assert.equal(poll.ok, true);
        if (poll.ok) {
            const body = poll.result as {
                capturing: boolean;
                pcm_i16: number[];
                whisper: { data: string; sample_rate: number } | null;
            };
            assert.equal(body.capturing, true);
            assert.ok(body.pcm_i16.length > 0);
            assert.ok(body.whisper);
            assert.equal(body.whisper!.sample_rate, 16_000);
            assert.ok(body.whisper!.data.length > 0);
        }

        const stop = await c.dispatch("stop");
        assert.equal(stop.ok, true);
    });

    it("unavailable backend surfaces graceful error", async () => {
        const c = new CaptureController(
            new UnavailableCaptureBackend("missing binary"),
        );
        await c.dispatch("grant_consent", { scope: MIC_CONSENT_SCOPE });
        const start = await c.dispatch("start", { media: "mic" });
        assert.equal(start.ok, false);
        if (!start.ok) {
            assert.equal(start.error_code, "unavailable");
            assert.ok(start.error.includes("unavailable"));
        }
        const st = await c.dispatch("status");
        assert.equal(st.ok, true);
        if (st.ok) {
            const body = st.result as { mic: { available: boolean } };
            assert.equal(body.mic.available, false);
        }
    });

    it("camera start is graceful unavailable even on mock", async () => {
        const c = new CaptureController(new MockCaptureBackend());
        await c.dispatch("grant_consent", { scope: CAMERA_CONSENT_SCOPE });
        const start = await c.dispatch("start", { media: "camera" });
        assert.equal(start.ok, false);
        if (!start.ok) {
            assert.equal(start.error_code, "unavailable");
        }
    });

    it("revoke_consent stops capture", async () => {
        const c = new CaptureController(new MockCaptureBackend());
        await c.dispatch("grant_consent", { scope: MIC_CONSENT_SCOPE });
        await c.dispatch("start", { media: "mic" });
        assert.equal(c.getBackend().isCapturing(), true);
        await c.dispatch("revoke_consent", { scope: MIC_CONSENT_SCOPE });
        assert.equal(c.hasConsent("mic"), false);
        assert.equal(c.getBackend().isCapturing(), false);
    });

    it("sensorMicStatus is daemon-shaped", () => {
        const c = new CaptureController(new MockCaptureBackend());
        c.grantConsent(MIC_CONSENT_SCOPE);
        const s = c.sensorMicStatus();
        assert.equal(s.available, true);
        assert.equal(s.bridge, "host");
        assert.equal(s.consent, true);
        assert.equal(s.upstream, "microsoft/vscode#303293");
    });

    it("buildResponse wraps ok and error", async () => {
        const c = new CaptureController(new MockCaptureBackend());
        const ok = c.buildResponse(1, "status", {
            ok: true,
            result: { x: 1 },
        });
        assert.equal(ok.type, CAPTURE_RESPONSE_TYPE);
        assert.equal(ok.ok, true);
        assert.equal((ok.result as { x: number }).x, 1);

        const err = c.buildResponse("a", "start", {
            ok: false,
            error: "nope",
            error_code: "consent_required",
        });
        assert.equal(err.ok, false);
        assert.equal(err.error_code, "consent_required");
    });

    it("unknown method fails", async () => {
        const c = new CaptureController(new MockCaptureBackend());
        const r = await c.dispatch("explode");
        assert.equal(r.ok, false);
        if (!r.ok) assert.equal(r.error_code, "method_not_found");
    });

    it("dispose is safe", async () => {
        const c = new CaptureController(new MockCaptureBackend());
        await c.dispatch("grant_consent", { scope: MIC_CONSENT_SCOPE });
        await c.dispatch("start", { media: "mic" });
        await c.dispose();
        assert.equal(c.hasConsent("mic"), false);
        assert.equal(c.getBackend().isCapturing(), false);
    });
});

describe("isCaptureRequest", () => {
    it("accepts valid envelopes", () => {
        assert.equal(
            isCaptureRequest({
                type: CAPTURE_REQUEST_TYPE,
                id: 1,
                method: "status",
            }),
            true,
        );
    });

    it("rejects garbage", () => {
        assert.equal(isCaptureRequest(null), false);
        assert.equal(isCaptureRequest({ type: "rpc-request" }), false);
        assert.equal(
            isCaptureRequest({ type: CAPTURE_REQUEST_TYPE, id: 1 }),
            false,
        );
    });
});
