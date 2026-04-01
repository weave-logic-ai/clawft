import {
  createContext,
  useContext,
  useState,
  useEffect,
  useCallback,
  useMemo,
  type ReactNode,
} from "react";
import type { WeftOSTheme } from "./types.ts";
import { loadBuiltinTheme, BUILTIN_THEME_NAMES } from "./loader.ts";

const THEME_STORAGE_KEY = "weftos-theme";

/** Retrieve persisted theme name from localStorage or Tauri config. */
function getSavedThemeName(): string | null {
  try {
    return localStorage.getItem(THEME_STORAGE_KEY);
  } catch {
    return null;
  }
}

/* ------------------------------------------------------------------ */
/* CSS variable mapping                                                */
/* ------------------------------------------------------------------ */

function setColorVars(
  vars: Record<string, string>,
  colors: WeftOSTheme["colors"],
): void {
  for (const [k, v] of Object.entries(colors)) {
    if (k === "semantic") continue;
    if (typeof v === "string") {
      vars[`--weftos-color-${camelToDash(k)}`] = v;
    }
  }
  if (colors.semantic) {
    for (const [k, v] of Object.entries(colors.semantic)) {
      vars[`--weftos-color-semantic-${camelToDash(k)}`] = v;
    }
  }
}

function setTypographyVars(
  vars: Record<string, string>,
  typo: WeftOSTheme["typography"],
): void {
  for (const [k, v] of Object.entries(typo.fontFamily)) {
    vars[`--weftos-font-family-${k}`] = v;
  }
  for (const [k, v] of Object.entries(typo.fontSize)) {
    vars[`--weftos-font-size-${k}`] = v;
  }
  if (typo.fontWeight) {
    for (const [k, v] of Object.entries(typo.fontWeight)) {
      vars[`--weftos-font-weight-${k}`] = String(v);
    }
  }
  if (typo.lineHeight) {
    for (const [k, v] of Object.entries(typo.lineHeight)) {
      vars[`--weftos-line-height-${k}`] = String(v);
    }
  }
  if (typo.letterSpacing) {
    for (const [k, v] of Object.entries(typo.letterSpacing)) {
      vars[`--weftos-letter-spacing-${k}`] = v;
    }
  }
}

function setBorderVars(
  vars: Record<string, string>,
  borders: NonNullable<WeftOSTheme["borders"]>,
): void {
  for (const [k, v] of Object.entries(borders.radius)) {
    vars[`--weftos-border-radius-${k}`] = v;
  }
  for (const [k, v] of Object.entries(borders.width)) {
    vars[`--weftos-border-width-${k}`] = v;
  }
}

function setEffectVars(
  vars: Record<string, string>,
  effects: NonNullable<WeftOSTheme["effects"]>,
): void {
  if (effects.shadow) {
    for (const [k, v] of Object.entries(effects.shadow)) {
      vars[`--weftos-shadow-${k}`] = v;
    }
  }
  if (effects.glow) {
    for (const [k, v] of Object.entries(effects.glow)) {
      vars[`--weftos-glow-${k}`] = v;
    }
  }
  if (effects.blur) {
    for (const [k, v] of Object.entries(effects.blur)) {
      vars[`--weftos-blur-${k}`] = v;
    }
  }
  if (effects.opacity) {
    for (const [k, v] of Object.entries(effects.opacity)) {
      vars[`--weftos-opacity-${k}`] = String(v);
    }
  }
}

function setAnimationVars(
  vars: Record<string, string>,
  anim: NonNullable<WeftOSTheme["animation"]>,
): void {
  for (const [k, v] of Object.entries(anim.duration)) {
    vars[`--weftos-duration-${k}`] = v;
  }
  for (const [k, v] of Object.entries(anim.easing)) {
    vars[`--weftos-easing-${camelToDash(k)}`] = v;
  }
}

/** Build a flat map of all CSS custom properties from a theme. */
export function themeToCSSVars(theme: WeftOSTheme): Record<string, string> {
  const vars: Record<string, string> = {};
  setColorVars(vars, theme.colors);
  setTypographyVars(vars, theme.typography);
  if (theme.borders) setBorderVars(vars, theme.borders);
  if (theme.effects) setEffectVars(vars, theme.effects);
  if (theme.animation) setAnimationVars(vars, theme.animation);
  return vars;
}

function camelToDash(s: string): string {
  return s.replace(/[A-Z]/g, (m) => `-${m.toLowerCase()}`);
}

/* ------------------------------------------------------------------ */
/* React Context                                                       */
/* ------------------------------------------------------------------ */

interface ThemeContextValue {
  theme: WeftOSTheme;
  themeName: string;
  setTheme: (nameOrTheme: string | WeftOSTheme) => void;
  availableThemes: string[];
}

const ThemeContext = createContext<ThemeContextValue | null>(null);

export function useTheme(): ThemeContextValue {
  const ctx = useContext(ThemeContext);
  if (!ctx) {
    throw new Error("useTheme must be used within a <ThemeProvider>");
  }
  return ctx;
}

/* ------------------------------------------------------------------ */
/* Provider                                                            */
/* ------------------------------------------------------------------ */

interface ThemeProviderProps {
  children: ReactNode;
  defaultTheme?: string;
}

export function ThemeProvider({
  children,
  defaultTheme = "ocean-dark",
}: ThemeProviderProps) {
  const [theme, setThemeState] = useState<WeftOSTheme>(() => {
    const saved = getSavedThemeName();
    if (saved) {
      const loaded = loadBuiltinTheme(saved);
      if (loaded) return loaded;
    }
    return loadBuiltinTheme(defaultTheme) ?? loadBuiltinTheme("ocean-dark")!;
  });

  const setTheme = useCallback((nameOrTheme: string | WeftOSTheme) => {
    if (typeof nameOrTheme === "string") {
      const loaded = loadBuiltinTheme(nameOrTheme);
      if (loaded) setThemeState(loaded);
    } else {
      setThemeState(nameOrTheme);
    }
  }, []);

  // Apply CSS custom properties to :root whenever theme changes
  useEffect(() => {
    const vars = themeToCSSVars(theme);
    const root = document.documentElement;
    for (const [prop, value] of Object.entries(vars)) {
      root.style.setProperty(prop, value);
    }
    root.setAttribute("data-theme", theme.mode);
    root.setAttribute("data-theme-name", theme.name);

    return () => {
      for (const prop of Object.keys(vars)) {
        root.style.removeProperty(prop);
      }
    };
  }, [theme]);

  const value = useMemo<ThemeContextValue>(
    () => ({
      theme,
      themeName: theme.name,
      setTheme,
      availableThemes: BUILTIN_THEME_NAMES,
    }),
    [theme, setTheme],
  );

  return (
    <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>
  );
}
