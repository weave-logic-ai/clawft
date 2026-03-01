/**
 * Voice-to-agent chat helper.
 *
 * Sends a message via the REST API, then waits for the agent's response
 * on the WebSocket `sessions:{key}` topic. The REST endpoint only returns
 * the user's own message as an echo; the real agent reply arrives async
 * via the broadcaster.
 */

import { api } from "./api-client";
import { wsClient } from "./ws-client";

const VOICE_SESSION = "voice";
const RESPONSE_TIMEOUT_MS = 30_000;

/** Ensure we're subscribed to the voice session topic. */
let subscribed = false;
function ensureSubscribed() {
  if (!subscribed) {
    wsClient.subscribe(`sessions:${VOICE_SESSION}`);
    subscribed = true;
  }
}

/**
 * Send a voice transcript to the agent and wait for the assistant response.
 *
 * Returns the assistant's reply text, or throws on timeout/error.
 */
export async function sendVoiceMessage(text: string): Promise<string> {
  ensureSubscribed();

  return new Promise<string>((resolve, reject) => {
    const timeout = setTimeout(() => {
      cleanup();
      reject(new Error("Response timed out"));
    }, RESPONSE_TIMEOUT_MS);

    const cleanup = wsClient.on("event", (raw: unknown) => {
      const msg = raw as {
        type: string;
        topic?: string;
        data?: {
          type?: string;
          role?: string;
          content?: string;
        };
      };

      if (
        msg.topic === `sessions:${VOICE_SESSION}` &&
        msg.data?.type === "message" &&
        msg.data?.role === "assistant"
      ) {
        clearTimeout(timeout);
        cleanup();
        resolve(msg.data.content || "");
      }
    });

    // Fire the message — we don't care about the echo response.
    api.chat.send(VOICE_SESSION, text).catch((err) => {
      clearTimeout(timeout);
      cleanup();
      reject(err);
    });
  });
}
