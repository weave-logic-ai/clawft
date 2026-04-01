/**
 * ThemeSwitcher — dropdown component for switching the active WeftOS theme.
 * Persists the selection to localStorage (browser) or via Tauri invoke (desktop).
 */

import { useCallback } from 'react';
import { useTheme } from '../themes/ThemeProvider';

const STORAGE_KEY = 'weftos-theme';

function isTauri(): boolean {
  return typeof window !== 'undefined' && !!window.__TAURI_INTERNALS__;
}

async function persistThemeName(name: string): Promise<void> {
  if (isTauri()) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('save_config', { key: 'theme', value: name });
      return;
    } catch {
      // Fall through to localStorage if Tauri command unavailable
    }
  }
  try {
    localStorage.setItem(STORAGE_KEY, name);
  } catch {
    // localStorage may be unavailable in some contexts
  }
}

export function ThemeSwitcher() {
  const { themeName, setTheme, availableThemes } = useTheme();

  const handleChange = useCallback(
    (e: React.ChangeEvent<HTMLSelectElement>) => {
      const name = e.target.value;
      setTheme(name);
      persistThemeName(name);
    },
    [setTheme],
  );

  return (
    <div className="flex items-center gap-2">
      <label
        htmlFor="theme-switcher"
        className="text-xs text-gray-400 whitespace-nowrap"
      >
        Theme
      </label>
      <select
        id="theme-switcher"
        value={themeName}
        onChange={handleChange}
        className="bg-gray-800 border border-gray-700 text-gray-200 text-xs rounded px-2 py-1 focus:outline-none focus:ring-1 focus:ring-indigo-500 focus:border-indigo-500"
      >
        {availableThemes.map((name) => (
          <option key={name} value={name}>
            {formatThemeName(name)}
          </option>
        ))}
      </select>
    </div>
  );
}

function formatThemeName(name: string): string {
  return name
    .split('-')
    .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
    .join(' ');
}
