import { useState } from "react";
import { useVoiceStore } from "../../stores/voice-store";
import { api } from "../../lib/api-client";
import { Card, CardHeader, CardTitle, CardContent } from "../ui/card";
import { Button } from "../ui/button";
import { Badge } from "../ui/badge";
import { cn } from "../../lib/utils";

const languages = [
  { code: "en", label: "English" },
  { code: "es", label: "Spanish" },
  { code: "fr", label: "French" },
  { code: "de", label: "German" },
  { code: "ja", label: "Japanese" },
  { code: "zh", label: "Chinese" },
  { code: "ko", label: "Korean" },
] as const;

function Toggle({
  checked,
  onChange,
  label,
  description,
}: {
  checked: boolean;
  onChange: (checked: boolean) => void;
  label: string;
  description?: string;
}) {
  return (
    <label className="flex items-center justify-between py-3 cursor-pointer">
      <div>
        <span className="text-sm font-medium text-gray-900 dark:text-gray-100">
          {label}
        </span>
        {description && (
          <p className="text-xs text-gray-500 dark:text-gray-400">
            {description}
          </p>
        )}
      </div>
      <button
        type="button"
        role="switch"
        aria-checked={checked}
        onClick={() => onChange(!checked)}
        className={cn(
          "relative inline-flex h-6 w-11 shrink-0 rounded-full border-2 border-transparent transition-colors",
          "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500 focus-visible:ring-offset-2",
          checked ? "bg-blue-600" : "bg-gray-200 dark:bg-gray-600",
        )}
      >
        <span
          className={cn(
            "pointer-events-none inline-block h-5 w-5 rounded-full bg-white shadow-lg ring-0 transition-transform",
            checked ? "translate-x-5" : "translate-x-0",
          )}
        />
      </button>
    </label>
  );
}

export function VoiceSettings() {
  const { settings, updateSettings } = useVoiceStore();
  const [micTestResult, setMicTestResult] = useState<{
    success: boolean;
    level: number;
  } | null>(null);
  const [speakerTestResult, setSpeakerTestResult] = useState<{
    success: boolean;
  } | null>(null);
  const [testing, setTesting] = useState<"mic" | "speaker" | null>(null);

  const handleSettingChange = (key: string, value: boolean | string) => {
    updateSettings({ [key]: value });
    api.voice.updateSettings({ [key]: value }).catch(() => {
      // revert on failure silently
    });
  };

  const handleTestMic = async () => {
    setTesting("mic");
    setMicTestResult(null);
    try {
      const result = await api.voice.testMic();
      setMicTestResult(result);
    } catch {
      setMicTestResult({ success: false, level: 0 });
    } finally {
      setTesting(null);
    }
  };

  const handleTestSpeaker = async () => {
    setTesting("speaker");
    setSpeakerTestResult(null);
    try {
      const result = await api.voice.testSpeaker();
      setSpeakerTestResult(result);
    } catch {
      setSpeakerTestResult({ success: false });
    } finally {
      setTesting(null);
    }
  };

  return (
    <div className="space-y-6">
      {/* Main toggles */}
      <Card>
        <CardHeader>
          <CardTitle>Voice Configuration</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="divide-y divide-gray-100 dark:divide-gray-700">
            <Toggle
              checked={settings.enabled}
              onChange={(v) => handleSettingChange("enabled", v)}
              label="Voice Enabled"
              description="Enable voice input and output for this workspace"
            />
            <Toggle
              checked={settings.wakeWordEnabled}
              onChange={(v) => handleSettingChange("wakeWordEnabled", v)}
              label="Wake Word"
              description="Activate voice with a wake word instead of manual trigger"
            />
            <Toggle
              checked={settings.echoCancel}
              onChange={(v) => handleSettingChange("echoCancel", v)}
              label="Echo Cancellation"
              description="Remove speaker output from microphone input"
            />
            <Toggle
              checked={settings.noiseSuppression}
              onChange={(v) => handleSettingChange("noiseSuppression", v)}
              label="Noise Suppression"
              description="Filter background noise from voice input"
            />
            <Toggle
              checked={settings.pushToTalk}
              onChange={(v) => handleSettingChange("pushToTalk", v)}
              label="Push-to-Talk"
              description="Hold a button to record instead of continuous listening"
            />
          </div>
        </CardContent>
      </Card>

      {/* Language */}
      <Card>
        <CardHeader>
          <CardTitle>Language</CardTitle>
        </CardHeader>
        <CardContent>
          <select
            value={settings.language}
            onChange={(e) => handleSettingChange("language", e.target.value)}
            className={cn(
              "w-full rounded-md border border-gray-300 bg-white px-3 py-2 text-sm",
              "text-gray-900 shadow-sm transition-colors",
              "focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500",
              "dark:border-gray-600 dark:bg-gray-700 dark:text-gray-100",
            )}
          >
            {languages.map((lang) => (
              <option key={lang.code} value={lang.code}>
                {lang.label}
              </option>
            ))}
          </select>
        </CardContent>
      </Card>

      {/* Audio tests */}
      <Card>
        <CardHeader>
          <CardTitle>Audio Test</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="space-y-4">
            {/* Microphone test */}
            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm font-medium text-gray-900 dark:text-gray-100">
                  Microphone
                </p>
                {micTestResult && (
                  <p className="text-xs text-gray-500 dark:text-gray-400">
                    {micTestResult.success
                      ? `Level: ${Math.round(micTestResult.level * 100)}%`
                      : "Test failed"}
                  </p>
                )}
              </div>
              <div className="flex items-center gap-2">
                {micTestResult && (
                  <Badge
                    variant={micTestResult.success ? "success" : "destructive"}
                  >
                    {micTestResult.success ? "OK" : "Failed"}
                  </Badge>
                )}
                <Button
                  variant="outline"
                  size="sm"
                  onClick={handleTestMic}
                  disabled={testing !== null}
                >
                  {testing === "mic" ? "Testing..." : "Test Microphone"}
                </Button>
              </div>
            </div>

            {/* Speaker test */}
            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm font-medium text-gray-900 dark:text-gray-100">
                  Speaker
                </p>
                {speakerTestResult && (
                  <p className="text-xs text-gray-500 dark:text-gray-400">
                    {speakerTestResult.success
                      ? "Playback successful"
                      : "Test failed"}
                  </p>
                )}
              </div>
              <div className="flex items-center gap-2">
                {speakerTestResult && (
                  <Badge
                    variant={
                      speakerTestResult.success ? "success" : "destructive"
                    }
                  >
                    {speakerTestResult.success ? "OK" : "Failed"}
                  </Badge>
                )}
                <Button
                  variant="outline"
                  size="sm"
                  onClick={handleTestSpeaker}
                  disabled={testing !== null}
                >
                  {testing === "speaker" ? "Testing..." : "Test Speaker"}
                </Button>
              </div>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Audio quality status */}
      <Card>
        <CardHeader>
          <CardTitle>Audio Quality</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="space-y-2 text-sm">
            <div className="flex justify-between">
              <span className="text-gray-600 dark:text-gray-400">
                Echo Cancellation
              </span>
              <Badge variant={settings.echoCancel ? "success" : "secondary"}>
                {settings.echoCancel ? "Active" : "Off"}
              </Badge>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-600 dark:text-gray-400">
                Noise Suppression
              </span>
              <Badge
                variant={settings.noiseSuppression ? "success" : "secondary"}
              >
                {settings.noiseSuppression ? "Active" : "Off"}
              </Badge>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-600 dark:text-gray-400">
                Sample Rate
              </span>
              <span className="font-medium text-gray-900 dark:text-gray-100">
                48 kHz
              </span>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-600 dark:text-gray-400">Codec</span>
              <span className="font-medium text-gray-900 dark:text-gray-100">
                Opus
              </span>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
