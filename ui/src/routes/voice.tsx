import { useEffect } from "react";
import { useVoiceStore } from "../stores/voice-store";
import { api } from "../lib/api-client";
import { VoiceSettings } from "../components/voice/settings";
import { PushToTalk } from "../components/voice/push-to-talk";
import { Card, CardHeader, CardTitle, CardContent } from "../components/ui/card";
import { Badge } from "../components/ui/badge";

const stateVariant: Record<
  string,
  "success" | "default" | "secondary" | "destructive"
> = {
  idle: "secondary",
  listening: "success",
  processing: "default",
  speaking: "default",
};

export function VoicePage() {
  const { state, settings } = useVoiceStore();

  useEffect(() => {
    api.voice.status().then((data) => {
      const store = useVoiceStore.getState();
      store.setState(data.state);
      store.setTalkMode(data.talkModeActive);
      store.updateSettings(data.settings);
    }).catch(() => {
      // MSW or network error; keep defaults
    });
  }, []);

  return (
    <div className="flex-1 overflow-auto p-6 space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900 dark:text-gray-100">
            Voice
          </h1>
          <p className="text-sm text-gray-500 dark:text-gray-400">
            Configure voice input, output, and talk mode
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Badge variant={stateVariant[state]}>{state}</Badge>
          <Badge variant={settings.enabled ? "success" : "secondary"}>
            {settings.enabled ? "Enabled" : "Disabled"}
          </Badge>
        </div>
      </div>

      <div className="grid grid-cols-1 gap-6 lg:grid-cols-3">
        {/* Settings panel (takes 2 cols on large screens) */}
        <div className="lg:col-span-2">
          <VoiceSettings />
        </div>

        {/* Push-to-talk panel */}
        <div className="space-y-6">
          <Card>
            <CardHeader>
              <CardTitle>Push-to-Talk</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="flex justify-center py-4">
                <PushToTalk />
              </div>
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  );
}
