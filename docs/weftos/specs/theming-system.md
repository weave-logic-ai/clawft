# WeftOS Theming System Specification

**Version**: 0.2.0
**Date**: 2026-03-27
**Source**: Sprint 11 Symposium Track 4 (K8.6 deliverable, brought forward for early design)
**Status**: Formal specification -- implementable
**Depends on**: block-descriptor-schema.json (0.2.0), block-catalog.md (0.2.0), mentra-hud-constraints.md

---

## Overview

The WeftOS theming system is a **renderer-agnostic token layer** that sits between block descriptors and their visual output. A single theme definition produces consistent styling across all rendering targets: React (Tauri desktop), Terminal (xterm.js / Ink), Mentra HUD (monochrome OLED), Voice (TTS pace/tone hints), MCP (Markdown formatting), PDF, and Shell (plain text).

Because everything in WeftOS is a Lego block rendered from a JSON descriptor, theming is never embedded in component code. Instead, each renderer reads the active theme's tokens and maps them to target-specific primitives (CSS custom properties for web, ANSI escape codes for terminal, brightness levels for HUD).

### Design Principles

1. **Tokens, not styles.** Themes define semantic tokens (`colors.surface`, `colors.danger`). Renderers map tokens to platform primitives.
2. **Same descriptor, different look.** A block descriptor never contains color or font values. The theme layer injects them at render time.
3. **Inheritance with override.** Parent blocks can set a theme scope. Child blocks inherit it. Any block can override individual tokens.
4. **Multi-target by default.** Every theme definition includes target-specific overrides for terminal, HUD, and voice. A theme that only defines web colors is incomplete.
5. **User-extensible.** Users create, share, and install themes through the same discovery chain as skills: workspace > user > built-in.

### Relationship to Existing Design

- **Fumadocs ocean.css**: The current docs site uses Fumadocs's ocean preset (`@import 'fumadocs-ui/css/ocean.css'`). The `ocean-dark` built-in theme in this spec derives its dark mode palette from the same HSL values used in that preset. The docs site continues to use Fumadocs's own CSS; this theming system governs the Tauri GUI, console, and HUD renderers.
- **Brand identity (06-brand-identity.md)**: The brand spec defines three palettes (WeaveLogic slate/blue, WeftOS indigo, AI Assessor emerald). The theming system's `ocean-dark` theme maps to the WeftOS indigo palette. Other themes may use different palettes.
- **Block descriptor schema**: The schema gains an optional `theme` field on each `BlockElement` for per-block overrides (Section 5).

---

## 1. Theme Definition Format

A theme is a single JSON file conforming to the WeftOS Theme Schema.

```json
{
  "$schema": "https://weftos.weavelogic.dev/schemas/theme/0.2.0",
  "name": "ocean-dark",
  "version": "1.0.0",
  "displayName": "Ocean Dark",
  "description": "Deep ocean blue palette inspired by Fumadocs ocean preset. Default WeftOS theme.",
  "author": "WeaveLogic",
  "license": "Apache-2.0",
  "mode": "dark",

  "colors": {
    "background": "hsl(220, 60%, 8%)",
    "foreground": "hsl(220, 60%, 94.5%)",
    "surface": "hsla(220, 56%, 15%, 0.4)",
    "surfaceAlt": "hsl(220, 50%, 10%)",
    "border": "hsla(220, 50%, 50%, 0.2)",
    "borderStrong": "hsla(220, 50%, 50%, 0.4)",

    "primary": "hsl(205, 100%, 85%)",
    "primaryForeground": "hsl(0, 0%, 9%)",
    "secondary": "hsl(220, 50%, 20%)",
    "secondaryForeground": "hsl(220, 80%, 90%)",
    "accent": "hsl(220, 40%, 20%)",
    "accentForeground": "hsl(220, 80%, 90%)",

    "muted": "hsl(220, 50%, 10%)",
    "mutedForeground": "hsl(220, 30%, 65%)",

    "success": "hsl(152, 69%, 31%)",
    "successForeground": "hsl(0, 0%, 98%)",
    "warning": "hsl(38, 92%, 50%)",
    "warningForeground": "hsl(0, 0%, 9%)",
    "error": "hsl(0, 84%, 60%)",
    "errorForeground": "hsl(0, 0%, 98%)",
    "info": "hsl(205, 100%, 63.9%)",
    "infoForeground": "hsl(0, 0%, 9%)",

    "ring": "hsl(205, 100%, 85%)",
    "selection": "hsla(220, 100%, 60%, 0.3)",

    "semantic": {
      "metricNormal": "hsl(220, 60%, 94.5%)",
      "metricWarn": "hsl(38, 92%, 50%)",
      "metricCrit": "hsl(0, 84%, 60%)",
      "governancePermit": "hsl(152, 69%, 31%)",
      "governanceDeny": "hsl(0, 84%, 60%)",
      "chainEvent": "hsl(220, 80%, 90%)",
      "agentRunning": "hsl(152, 69%, 31%)",
      "agentIdle": "hsl(220, 30%, 65%)",
      "agentStopped": "hsl(0, 84%, 60%)",
      "codeBackground": "hsl(220, 30%, 6%)",
      "codeForeground": "hsl(220, 40%, 88%)"
    }
  },

  "typography": {
    "fontFamily": {
      "sans": "Inter, system-ui, -apple-system, sans-serif",
      "mono": "JetBrains Mono, Fira Code, Cascadia Code, monospace",
      "heading": "Inter, system-ui, -apple-system, sans-serif"
    },
    "fontSize": {
      "xs": "0.75rem",
      "sm": "0.875rem",
      "base": "1rem",
      "lg": "1.125rem",
      "xl": "1.25rem",
      "2xl": "1.5rem",
      "3xl": "2.25rem",
      "4xl": "3rem"
    },
    "fontWeight": {
      "normal": 400,
      "medium": 500,
      "semibold": 600,
      "bold": 700
    },
    "lineHeight": {
      "tight": 1.1,
      "snug": 1.3,
      "normal": 1.5,
      "relaxed": 1.6
    },
    "letterSpacing": {
      "tight": "-0.025em",
      "normal": "0em",
      "wide": "0.025em"
    }
  },

  "spacing": {
    "unit": 4,
    "scale": [0, 1, 2, 3, 4, 5, 6, 8, 10, 12, 16, 20, 24, 32, 40, 48, 64]
  },

  "borders": {
    "radius": {
      "none": "0px",
      "sm": "4px",
      "md": "8px",
      "lg": "12px",
      "xl": "16px",
      "full": "9999px"
    },
    "width": {
      "none": "0px",
      "thin": "1px",
      "medium": "2px",
      "thick": "3px"
    }
  },

  "effects": {
    "shadow": {
      "none": "none",
      "sm": "0 1px 2px rgba(0, 0, 0, 0.3)",
      "md": "0 4px 6px rgba(0, 0, 0, 0.3)",
      "lg": "0 10px 15px rgba(0, 0, 0, 0.3)",
      "card": "0 2px 8px rgba(0, 0, 0, 0.2)"
    },
    "glow": {
      "none": "none",
      "subtle": "0 0 8px hsla(205, 100%, 85%, 0.15)",
      "medium": "0 0 16px hsla(205, 100%, 85%, 0.25)",
      "strong": "0 0 24px hsla(205, 100%, 85%, 0.4)",
      "success": "0 0 12px hsla(152, 69%, 31%, 0.3)",
      "warning": "0 0 12px hsla(38, 92%, 50%, 0.3)",
      "error": "0 0 12px hsla(0, 84%, 60%, 0.3)"
    },
    "blur": {
      "none": "0px",
      "sm": "4px",
      "md": "8px",
      "lg": "16px"
    },
    "opacity": {
      "disabled": 0.5,
      "muted": 0.7,
      "full": 1.0
    }
  },

  "animation": {
    "duration": {
      "instant": "0ms",
      "fast": "100ms",
      "normal": "200ms",
      "slow": "400ms",
      "glacial": "800ms"
    },
    "easing": {
      "linear": "linear",
      "ease": "cubic-bezier(0.4, 0, 0.2, 1)",
      "easeIn": "cubic-bezier(0.4, 0, 1, 1)",
      "easeOut": "cubic-bezier(0, 0, 0.2, 1)",
      "easeInOut": "cubic-bezier(0.4, 0, 0.2, 1)",
      "spring": "cubic-bezier(0.34, 1.56, 0.64, 1)"
    }
  },

  "console": {
    "ansi": {
      "black": "#1a1b26",
      "red": "#f7768e",
      "green": "#9ece6a",
      "yellow": "#e0af68",
      "blue": "#7aa2f7",
      "magenta": "#bb9af7",
      "cyan": "#7dcfff",
      "white": "#c0caf5",
      "brightBlack": "#414868",
      "brightRed": "#f7768e",
      "brightGreen": "#9ece6a",
      "brightYellow": "#e0af68",
      "brightBlue": "#7aa2f7",
      "brightMagenta": "#bb9af7",
      "brightCyan": "#7dcfff",
      "brightWhite": "#c0caf5"
    },
    "background": "#1a1b26",
    "foreground": "#c0caf5",
    "cursor": "#c0caf5",
    "cursorAccent": "#1a1b26",
    "selectionBackground": "rgba(122, 162, 247, 0.3)",
    "prompt": {
      "format": "{user}@{host}:{path} {gitBranch} {kernelStatus}\n{symbol} ",
      "tokens": {
        "user": { "color": "green", "bold": true },
        "host": { "color": "blue", "bold": false },
        "path": { "color": "cyan", "bold": true },
        "gitBranch": { "color": "magenta", "bold": false, "prefix": "git:(", "suffix": ")" },
        "kernelStatus": { "color": "yellow", "bold": false, "prefix": "[", "suffix": "]" },
        "symbol": { "color": "green", "bold": true, "text": "$" }
      }
    },
    "syntaxHighlighting": {
      "keyword": "#bb9af7",
      "string": "#9ece6a",
      "number": "#ff9e64",
      "comment": "#565f89",
      "function": "#7aa2f7",
      "operator": "#89ddff",
      "type": "#2ac3de",
      "variable": "#c0caf5",
      "constant": "#ff9e64",
      "tag": "#f7768e",
      "attribute": "#7aa2f7",
      "punctuation": "#89ddff"
    },
    "panel": {
      "border": "single",
      "borderColor": "blue",
      "titleColor": "cyan",
      "titleBold": true,
      "padding": 1
    },
    "table": {
      "borderStyle": "rounded",
      "headerColor": "cyan",
      "headerBold": true,
      "rowAlternateBackground": true,
      "alternateColor": "brightBlack"
    }
  },

  "hud": {
    "foreground": "#00ff41",
    "background": "#000000",
    "warningForeground": "#ffff00",
    "errorForeground": "#ff0000",
    "mutedForeground": "#006620",
    "headerSeparator": "=",
    "footerSeparator": "-",
    "selectionIndicator": ">",
    "progressFilled": "=",
    "progressEmpty": " ",
    "progressBrackets": ["[", "]"]
  },

  "voice": {
    "pace": "normal",
    "pitch": "medium",
    "emphasisStyle": "stress",
    "pauseAfterHeading": "500ms",
    "pauseAfterParagraph": "300ms",
    "errorTone": "low",
    "successTone": "bright",
    "codeReadStyle": "spell"
  }
}
```

### Token Naming Convention

All tokens use camelCase. Nested tokens are addressed with dot notation in overrides: `colors.primary`, `console.ansi.red`, `effects.glow.subtle`.

### Required vs Optional Sections

| Section | Required | Fallback |
|---------|----------|----------|
| `colors` | Yes | -- |
| `typography` | Yes | -- |
| `spacing` | No | Default scale (4px unit, Tailwind-compatible) |
| `borders` | No | Default Tailwind radii and widths |
| `effects` | No | No shadows, no glows |
| `animation` | No | Default 200ms ease |
| `console` | No | Inherit from `colors` with auto-mapped ANSI palette |
| `hud` | No | Green-on-black monochrome derived from `colors.primary` |
| `voice` | No | Default pace/pitch |

---

## 2. Multi-Target Theme Resolution

Each renderer consumes the theme through a target-specific resolver. The resolver reads the theme JSON and produces output suitable for that renderer.

### 2.1 Web (React / Tauri GUI)

**Mechanism**: CSS custom properties injected at the `:root` level (or scoped to a block's DOM element for overrides).

**Resolution process**:

1. Theme loader reads the theme JSON.
2. Each token is converted to a CSS custom property:
   - `colors.background` becomes `--weftos-color-background`
   - `typography.fontSize.base` becomes `--weftos-font-size-base`
   - `effects.glow.subtle` becomes `--weftos-glow-subtle`
3. Properties are injected into a `<style>` element on `:root`.
4. React components reference tokens via `var(--weftos-color-background)`.

**Tailwind CSS 4 integration**:

Since Tailwind CSS 4 uses CSS custom properties natively via `@theme`, the theme tokens map directly:

```css
/* Generated by ThemeResolver<Web> from theme JSON */
@theme {
  --color-weftos-bg: var(--weftos-color-background);
  --color-weftos-fg: var(--weftos-color-foreground);
  --color-weftos-surface: var(--weftos-color-surface);
  --color-weftos-primary: var(--weftos-color-primary);
  --color-weftos-secondary: var(--weftos-color-secondary);
  --color-weftos-accent: var(--weftos-color-accent);
  --color-weftos-success: var(--weftos-color-success);
  --color-weftos-warning: var(--weftos-color-warning);
  --color-weftos-error: var(--weftos-color-error);
  --color-weftos-info: var(--weftos-color-info);
  --color-weftos-muted: var(--weftos-color-muted);
  --color-weftos-ring: var(--weftos-color-ring);

  --font-sans: var(--weftos-font-family-sans);
  --font-mono: var(--weftos-font-family-mono);

  --radius-sm: var(--weftos-border-radius-sm);
  --radius-md: var(--weftos-border-radius-md);
  --radius-lg: var(--weftos-border-radius-lg);
}
```

Then in component classes: `bg-weftos-bg text-weftos-fg border-weftos-surface`.

**Dark/light mode**: Themes declare `"mode": "dark"` or `"mode": "light"`. The web resolver sets the `data-theme` attribute on `<html>`. Components do not use Tailwind's `dark:` prefix for theme colors; they use the `--weftos-*` variables which switch entirely when the theme changes. The Fumadocs docs site remains independent (uses its own `dark:` classes).

### 2.2 Terminal (xterm.js in ConsolePan / Ink in CLI)

**Mechanism**: xterm.js `ITheme` object and Ink `<Box>` color props.

**Resolution process**:

1. The `console.ansi` palette maps directly to xterm.js `ITheme`:
   ```typescript
   const xtermTheme: ITheme = {
     background: theme.console.background,
     foreground: theme.console.foreground,
     cursor: theme.console.cursor,
     cursorAccent: theme.console.cursorAccent,
     selectionBackground: theme.console.selectionBackground,
     black: theme.console.ansi.black,
     red: theme.console.ansi.red,
     green: theme.console.ansi.green,
     yellow: theme.console.ansi.yellow,
     blue: theme.console.ansi.blue,
     magenta: theme.console.ansi.magenta,
     cyan: theme.console.ansi.cyan,
     white: theme.console.ansi.white,
     brightBlack: theme.console.ansi.brightBlack,
     brightRed: theme.console.ansi.brightRed,
     brightGreen: theme.console.ansi.brightGreen,
     brightYellow: theme.console.ansi.brightYellow,
     brightBlue: theme.console.ansi.brightBlue,
     brightMagenta: theme.console.ansi.brightMagenta,
     brightCyan: theme.console.ansi.brightCyan,
     brightWhite: theme.console.ansi.brightWhite
   };
   ```
2. For Ink (CLI renderer), colors map to Ink's chalk-based color system. ANSI names resolve to 256-color or true-color depending on terminal capability.
3. `console.panel` and `console.table` tokens control the `rich`-style formatting of bordered output panels and ASCII tables.

**Prompt rendering**:

The `console.prompt` section defines the prompt format. The prompt renderer tokenizes the format string, resolves each `{token}` against system state (user, host, git branch, kernel status), and applies the color/bold settings from `prompt.tokens`.

Example rendered prompt (ANSI escape sequences applied):
```
aepod@weftos:~/project git:(main) [running]
$
```

### 2.3 Console Rich Output (Panels, Tables, Borders)

When the console renders structured output (tables from `DataTable` blocks, panels from bordered output, chain events from `ChainViewer`), it uses the `console.panel` and `console.table` tokens.

**Panel border styles**: `"single"`, `"double"`, `"rounded"`, `"heavy"`, `"none"`. These map to Unicode box-drawing characters:

| Style | Top-left | Horizontal | Vertical | Top-right |
|-------|----------|------------|----------|-----------|
| single | `+` or `\u250c` | `\u2500` | `\u2502` | `\u2510` |
| double | `\u2554` | `\u2550` | `\u2551` | `\u2557` |
| rounded | `\u256d` | `\u2500` | `\u2502` | `\u256e` |
| heavy | `\u250f` | `\u2501` | `\u2503` | `\u2513` |
| none | (space) | (space) | (space) | (space) |

**Example console panel output** (ocean-dark theme):
```
+-------------------------------------------------+
| KERNEL STATUS                         [Running] |
+-------------------------------------------------+
| Uptime:   14h 22m                               |
| Services: 14 active                             |
| Chain:    block #4,271                          |
| Health:   100%                                   |
+-------------------------------------------------+
```

The panel border color comes from `console.panel.borderColor`, title color from `console.panel.titleColor`, and content foreground from `console.foreground`.

### 2.4 Mentra HUD (Monochrome)

**Mechanism**: The HUD renderer uses `hud.*` tokens to control the 3-level monochrome palette.

**Resolution process**:

1. All block colors collapse to three states:
   - **Normal**: `hud.foreground` (default green `#00ff41`)
   - **Warning**: `hud.warningForeground` (yellow/brighter)
   - **Error**: `hud.errorForeground` (red/inverted)
2. Muted text uses `hud.mutedForeground` (dimmer green).
3. Separators and progress bar characters come from `hud.headerSeparator`, `hud.progressFilled`, etc.
4. Typography is always monospace (hardware constraint). Font tokens are ignored. Only character count matters.
5. Animations are ignored. The HUD updates at 2-5 Hz per the latency budget.

**Color mapping from semantic tokens**:

| Semantic Condition | HUD Color |
|-------------------|-----------|
| `metric.value < threshold.warn` | `hud.foreground` (normal) |
| `metric.value >= threshold.warn` | `hud.warningForeground` |
| `metric.value >= threshold.crit` | `hud.errorForeground` |
| governance permit | `hud.foreground` |
| governance deny | `hud.errorForeground` |
| agent running | `hud.foreground` |
| agent idle | `hud.mutedForeground` |
| agent stopped | `hud.errorForeground` |

### 2.5 Voice (TTS)

**Mechanism**: The voice renderer uses `voice.*` tokens to generate SSML or TTS API hints.

**Resolution process**:

1. `voice.pace` maps to SSML `<prosody rate="...">`: `"slow"` = 80%, `"normal"` = 100%, `"fast"` = 120%.
2. `voice.pitch` maps to SSML `<prosody pitch="...">`: `"low"`, `"medium"`, `"high"`.
3. Headings get a pause after them (`voice.pauseAfterHeading`).
4. Error messages are spoken with `voice.errorTone` (lower pitch).
5. Success messages use `voice.successTone` (slightly higher pitch).
6. Code blocks follow `voice.codeReadStyle`: `"spell"` (character by character for short), `"summarize"` (describe the code), `"skip"` (omit entirely).

### 2.6 MCP (Markdown Formatting)

**Mechanism**: The MCP renderer produces Markdown text. Theming is minimal but controls formatting choices.

**Resolution process**:

1. Colors are not applicable (Markdown has no color).
2. The theme controls structural choices:
   - Metric blocks render as `**Label**: Value Unit`
   - Tables render as GFM tables
   - Panels render as blockquotes with a bold title
   - Code blocks use fenced code with the language hint from the `CodeEditor` block's `language` prop.

---

## 3. Theme JSON Schema

The complete JSON Schema for theme validation.

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://weftos.weavelogic.dev/schemas/theme/0.2.0",
  "title": "WeftOS Theme",
  "description": "Theme definition for multi-target rendering of WeftOS Lego blocks.",
  "type": "object",
  "required": ["name", "version", "mode", "colors", "typography"],
  "additionalProperties": false,
  "properties": {
    "$schema": { "type": "string" },
    "name": {
      "type": "string",
      "pattern": "^[a-z][a-z0-9-]*$",
      "description": "Machine-readable theme name. Lowercase with hyphens."
    },
    "version": {
      "type": "string",
      "pattern": "^\\d+\\.\\d+\\.\\d+$",
      "description": "Semver version of this theme."
    },
    "displayName": {
      "type": "string",
      "description": "Human-readable theme name."
    },
    "description": { "type": "string" },
    "author": { "type": "string" },
    "license": { "type": "string" },
    "mode": {
      "type": "string",
      "enum": ["dark", "light"],
      "description": "Whether this theme is a dark or light theme."
    },
    "colors": { "$ref": "#/$defs/Colors" },
    "typography": { "$ref": "#/$defs/Typography" },
    "spacing": { "$ref": "#/$defs/Spacing" },
    "borders": { "$ref": "#/$defs/Borders" },
    "effects": { "$ref": "#/$defs/Effects" },
    "animation": { "$ref": "#/$defs/Animation" },
    "console": { "$ref": "#/$defs/Console" },
    "hud": { "$ref": "#/$defs/Hud" },
    "voice": { "$ref": "#/$defs/Voice" }
  },
  "$defs": {
    "CSSColor": {
      "type": "string",
      "description": "Any valid CSS color value (hex, hsl, hsla, rgb, rgba, named)."
    },
    "Colors": {
      "type": "object",
      "required": ["background", "foreground", "surface", "primary", "secondary", "accent", "success", "warning", "error", "info", "border", "muted", "mutedForeground", "ring"],
      "additionalProperties": false,
      "properties": {
        "background": { "$ref": "#/$defs/CSSColor" },
        "foreground": { "$ref": "#/$defs/CSSColor" },
        "surface": { "$ref": "#/$defs/CSSColor" },
        "surfaceAlt": { "$ref": "#/$defs/CSSColor" },
        "border": { "$ref": "#/$defs/CSSColor" },
        "borderStrong": { "$ref": "#/$defs/CSSColor" },
        "primary": { "$ref": "#/$defs/CSSColor" },
        "primaryForeground": { "$ref": "#/$defs/CSSColor" },
        "secondary": { "$ref": "#/$defs/CSSColor" },
        "secondaryForeground": { "$ref": "#/$defs/CSSColor" },
        "accent": { "$ref": "#/$defs/CSSColor" },
        "accentForeground": { "$ref": "#/$defs/CSSColor" },
        "muted": { "$ref": "#/$defs/CSSColor" },
        "mutedForeground": { "$ref": "#/$defs/CSSColor" },
        "success": { "$ref": "#/$defs/CSSColor" },
        "successForeground": { "$ref": "#/$defs/CSSColor" },
        "warning": { "$ref": "#/$defs/CSSColor" },
        "warningForeground": { "$ref": "#/$defs/CSSColor" },
        "error": { "$ref": "#/$defs/CSSColor" },
        "errorForeground": { "$ref": "#/$defs/CSSColor" },
        "info": { "$ref": "#/$defs/CSSColor" },
        "infoForeground": { "$ref": "#/$defs/CSSColor" },
        "ring": { "$ref": "#/$defs/CSSColor" },
        "selection": { "$ref": "#/$defs/CSSColor" },
        "semantic": {
          "type": "object",
          "additionalProperties": { "$ref": "#/$defs/CSSColor" },
          "description": "Domain-specific semantic color aliases."
        }
      }
    },
    "Typography": {
      "type": "object",
      "required": ["fontFamily", "fontSize"],
      "additionalProperties": false,
      "properties": {
        "fontFamily": {
          "type": "object",
          "required": ["sans", "mono"],
          "additionalProperties": false,
          "properties": {
            "sans": { "type": "string" },
            "mono": { "type": "string" },
            "heading": { "type": "string" }
          }
        },
        "fontSize": {
          "type": "object",
          "additionalProperties": { "type": "string" }
        },
        "fontWeight": {
          "type": "object",
          "additionalProperties": { "type": "number" }
        },
        "lineHeight": {
          "type": "object",
          "additionalProperties": { "type": "number" }
        },
        "letterSpacing": {
          "type": "object",
          "additionalProperties": { "type": "string" }
        }
      }
    },
    "Spacing": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "unit": { "type": "number" },
        "scale": { "type": "array", "items": { "type": "number" } }
      }
    },
    "Borders": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "radius": { "type": "object", "additionalProperties": { "type": "string" } },
        "width": { "type": "object", "additionalProperties": { "type": "string" } }
      }
    },
    "Effects": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "shadow": { "type": "object", "additionalProperties": { "type": "string" } },
        "glow": { "type": "object", "additionalProperties": { "type": "string" } },
        "blur": { "type": "object", "additionalProperties": { "type": "string" } },
        "opacity": { "type": "object", "additionalProperties": { "type": "number" } }
      }
    },
    "Animation": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "duration": { "type": "object", "additionalProperties": { "type": "string" } },
        "easing": { "type": "object", "additionalProperties": { "type": "string" } }
      }
    },
    "Console": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "ansi": {
          "type": "object",
          "additionalProperties": { "type": "string" }
        },
        "background": { "type": "string" },
        "foreground": { "type": "string" },
        "cursor": { "type": "string" },
        "cursorAccent": { "type": "string" },
        "selectionBackground": { "type": "string" },
        "prompt": { "$ref": "#/$defs/ConsolePrompt" },
        "syntaxHighlighting": {
          "type": "object",
          "additionalProperties": { "type": "string" }
        },
        "panel": {
          "type": "object",
          "additionalProperties": false,
          "properties": {
            "border": { "type": "string", "enum": ["single", "double", "rounded", "heavy", "none"] },
            "borderColor": { "type": "string" },
            "titleColor": { "type": "string" },
            "titleBold": { "type": "boolean" },
            "padding": { "type": "number" }
          }
        },
        "table": {
          "type": "object",
          "additionalProperties": false,
          "properties": {
            "borderStyle": { "type": "string", "enum": ["single", "double", "rounded", "heavy", "none", "ascii"] },
            "headerColor": { "type": "string" },
            "headerBold": { "type": "boolean" },
            "rowAlternateBackground": { "type": "boolean" },
            "alternateColor": { "type": "string" }
          }
        }
      }
    },
    "ConsolePrompt": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "format": {
          "type": "string",
          "description": "Prompt format string with {token} placeholders."
        },
        "tokens": {
          "type": "object",
          "additionalProperties": {
            "type": "object",
            "properties": {
              "color": { "type": "string" },
              "bold": { "type": "boolean" },
              "italic": { "type": "boolean" },
              "prefix": { "type": "string" },
              "suffix": { "type": "string" },
              "text": { "type": "string" }
            }
          }
        }
      }
    },
    "Hud": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "foreground": { "type": "string" },
        "background": { "type": "string" },
        "warningForeground": { "type": "string" },
        "errorForeground": { "type": "string" },
        "mutedForeground": { "type": "string" },
        "headerSeparator": { "type": "string", "maxLength": 1 },
        "footerSeparator": { "type": "string", "maxLength": 1 },
        "selectionIndicator": { "type": "string", "maxLength": 2 },
        "progressFilled": { "type": "string", "maxLength": 1 },
        "progressEmpty": { "type": "string", "maxLength": 1 },
        "progressBrackets": {
          "type": "array",
          "items": { "type": "string", "maxLength": 1 },
          "minItems": 2,
          "maxItems": 2
        }
      }
    },
    "Voice": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "pace": { "type": "string", "enum": ["slow", "normal", "fast"] },
        "pitch": { "type": "string", "enum": ["low", "medium", "high"] },
        "emphasisStyle": { "type": "string", "enum": ["stress", "volume", "pitch"] },
        "pauseAfterHeading": { "type": "string" },
        "pauseAfterParagraph": { "type": "string" },
        "errorTone": { "type": "string" },
        "successTone": { "type": "string" },
        "codeReadStyle": { "type": "string", "enum": ["spell", "summarize", "skip"] }
      }
    }
  }
}
```

---

## 4. Block-Level Theming

Individual blocks can override theme tokens. This allows a specific Metric block to use red text without changing the global theme.

### 4.1 Descriptor Schema Extension

The `BlockElement` definition in `block-descriptor-schema.json` gains an optional `theme` field:

```json
{
  "type": "object",
  "required": ["type"],
  "additionalProperties": false,
  "properties": {
    "type": { ... },
    "children": { ... },
    "props": { ... },
    "on": { ... },
    "ports": { ... },
    "layout": { ... },
    "theme": {
      "$ref": "#/$defs/BlockThemeOverride"
    }
  }
}
```

The `BlockThemeOverride` definition:

```json
{
  "BlockThemeOverride": {
    "type": "object",
    "additionalProperties": false,
    "properties": {
      "override": {
        "type": "object",
        "description": "Map of dot-notation token paths to override values.",
        "additionalProperties": true
      },
      "inherit": {
        "type": "boolean",
        "default": true,
        "description": "Whether this block inherits the parent block's theme context. Default true."
      }
    }
  }
}
```

### 4.2 Override Examples

**Override a single color**:
```json
{
  "type": "Metric",
  "props": { "label": "CPU", "value": { "$state": "/kernel/metrics/cpu_percent" }, "unit": "%" },
  "theme": {
    "override": {
      "colors.foreground": "#ff4444"
    }
  }
}
```

**Override console panel style for an embedded ConsolePan**:
```json
{
  "type": "ConsolePan",
  "props": { "initialCommand": "chain.query --limit 5" },
  "theme": {
    "override": {
      "console.panel.border": "double",
      "console.panel.borderColor": "yellow",
      "effects.glow.subtle": "0 0 8px hsla(38, 92%, 50%, 0.2)"
    }
  }
}
```

**Use a completely different font for a code editor block**:
```json
{
  "type": "CodeEditor",
  "props": { "value": { "$state": "/journey/files/main_rs" }, "language": "rust", "readOnly": true },
  "theme": {
    "override": {
      "typography.fontFamily.mono": "Iosevka, monospace",
      "typography.fontSize.base": "0.8125rem"
    }
  }
}
```

### 4.3 Override Resolution

The renderer resolves theme tokens in this order (later wins):

1. **Built-in theme defaults** (the fallback theme compiled into the renderer)
2. **Active theme** (loaded from file)
3. **Parent block override** (if a layout block sets `theme.override`)
4. **Current block override** (this block's `theme.override`)

If `theme.inherit` is `false`, step 3 is skipped and the block uses only the active theme + its own overrides.

---

## 5. Theme Inheritance in Nested Blocks

Layout blocks (Column, Row, Grid, Tabs) can set a theme scope that applies to all their children.

### 5.1 Scoped Theming Example

A dashboard where the left sidebar has a darker surface and the main area uses the default theme:

```json
{
  "version": "0.2.0",
  "root": "dashboard",
  "elements": {
    "dashboard": {
      "type": "Row",
      "children": ["sidebar", "main"]
    },
    "sidebar": {
      "type": "Column",
      "children": ["tree", "agents"],
      "theme": {
        "override": {
          "colors.surface": "hsl(220, 50%, 6%)",
          "colors.border": "hsla(220, 50%, 50%, 0.1)"
        }
      }
    },
    "tree": {
      "type": "ResourceTree",
      "props": { "rootPath": "/src" }
    },
    "agents": {
      "type": "DataTable",
      "props": {
        "columns": [{"key": "pid", "label": "PID"}, {"key": "state", "label": "State"}],
        "rows": { "$state": "/kernel/processes" }
      }
    },
    "main": {
      "type": "Column",
      "children": ["metrics-row", "console"]
    },
    "metrics-row": {
      "type": "Row",
      "children": ["cpu", "mem", "chain"]
    },
    "cpu": { "type": "Metric", "props": { "label": "CPU", "value": { "$state": "/kernel/metrics/cpu_percent" }, "unit": "%" } },
    "mem": { "type": "Metric", "props": { "label": "Mem", "value": { "$state": "/kernel/metrics/mem_percent" }, "unit": "%" } },
    "chain": { "type": "Metric", "props": { "label": "Chain", "value": { "$state": "/kernel/chain/height" } } },
    "console": {
      "type": "ConsolePan",
      "props": {}
    }
  }
}
```

In this example, the `tree` and `agents` blocks inside `sidebar` inherit the darker surface. The `main` column and its children use the unmodified active theme.

### 5.2 Implementation: ThemeContext

On the web renderer, theme scoping is implemented via React Context:

```typescript
// Pseudocode -- not production code
const ThemeContext = createContext<ResolvedTheme>(defaultTheme);

function BlockRenderer({ element, parentTheme }: Props) {
  const resolved = useMemo(() => {
    if (!element.theme) return parentTheme;
    if (element.theme.inherit === false) {
      return applyOverrides(activeTheme, element.theme.override);
    }
    return applyOverrides(parentTheme, element.theme.override);
  }, [element.theme, parentTheme]);

  return (
    <ThemeContext.Provider value={resolved}>
      <div style={toCSSVars(resolved)}>
        <RegistryComponent type={element.type} props={element.props} />
      </div>
    </ThemeContext.Provider>
  );
}
```

For the terminal renderer, theme scoping is simulated by passing the resolved theme tokens through the render call stack. Each block receives its parent's resolved theme and can merge its own overrides.

---

## 6. User-Created Themes

### 6.1 Storage Location

```
~/.weftos/
  themes/
    my-custom-theme.json
    synthwave-84.json
  config.json           # References active theme
```

Workspace-level themes:
```
<project-root>/
  .weftos/
    themes/
      project-theme.json
```

### 6.2 Discovery Chain

Theme resolution follows the same priority as skills:

1. **Workspace**: `.weftos/themes/` in the current project root
2. **User**: `~/.weftos/themes/`
3. **Built-in**: Compiled into the WeftOS binary

If multiple themes have the same `name`, the workspace version wins.

### 6.3 Active Theme Selection

The active theme is stored in the user's preferences:

```json
// ~/.weftos/config.json
{
  "theme": "ocean-dark",
  "themeOverrides": {
    "typography.fontFamily.mono": "Iosevka, monospace"
  }
}
```

The `themeOverrides` field allows per-user customization without creating a full theme file. These overrides apply on top of the selected theme.

### 6.4 Console Theme Command

```
weftos theme list                        # List available themes
weftos theme active                      # Show current theme name
weftos theme set <name>                  # Switch active theme
weftos theme preview <name>              # Preview a theme (temporary, reverts on exit)
weftos theme create <name>               # Create a new theme from the current theme
weftos theme edit <name>                 # Open theme JSON in CodeEditor block
weftos theme export <name> [--output <path>]  # Export theme as JSON file
weftos theme import <path>               # Import a theme JSON file
```

### 6.5 Theme Marketplace Concept

Themes are shareable as JSON files. Future integration with the WeftOS skill/extension ecosystem:

- Themes can be published to a theme registry (same infrastructure as skill publishing).
- `weftos theme install <name>` downloads from the registry to `~/.weftos/themes/`.
- Theme metadata (`author`, `license`, `description`) is displayed in the theme browser.
- The Weaver can generate themes: "Create a theme that matches this brand's color palette" produces a valid theme JSON.

---

## 7. Built-In Themes

### 7.1 ocean-dark (Default)

The complete definition is shown in Section 1. Key characteristics:

- Deep blue-black backgrounds derived from the Fumadocs ocean preset
- Cool blue/indigo accent palette matching the WeftOS brand identity
- Tokyo Night-inspired terminal color scheme
- Subtle blue glow on focused/important elements
- Designed for extended coding sessions (low eye strain)

### 7.2 midnight

Deep black with neon accents. A hacker/cyberpunk aesthetic.

```json
{
  "name": "midnight",
  "version": "1.0.0",
  "displayName": "Midnight",
  "description": "Pure black with neon cyan and magenta accents. For those who prefer the void.",
  "author": "WeaveLogic",
  "license": "Apache-2.0",
  "mode": "dark",

  "colors": {
    "background": "hsl(0, 0%, 2%)",
    "foreground": "hsl(0, 0%, 90%)",
    "surface": "hsl(0, 0%, 6%)",
    "surfaceAlt": "hsl(0, 0%, 4%)",
    "border": "hsla(0, 0%, 100%, 0.08)",
    "borderStrong": "hsla(0, 0%, 100%, 0.16)",

    "primary": "hsl(180, 100%, 60%)",
    "primaryForeground": "hsl(0, 0%, 2%)",
    "secondary": "hsl(300, 80%, 60%)",
    "secondaryForeground": "hsl(0, 0%, 2%)",
    "accent": "hsl(180, 100%, 15%)",
    "accentForeground": "hsl(180, 100%, 80%)",

    "muted": "hsl(0, 0%, 10%)",
    "mutedForeground": "hsl(0, 0%, 50%)",

    "success": "hsl(120, 100%, 40%)",
    "successForeground": "hsl(0, 0%, 2%)",
    "warning": "hsl(45, 100%, 50%)",
    "warningForeground": "hsl(0, 0%, 2%)",
    "error": "hsl(0, 100%, 55%)",
    "errorForeground": "hsl(0, 0%, 98%)",
    "info": "hsl(180, 100%, 60%)",
    "infoForeground": "hsl(0, 0%, 2%)",

    "ring": "hsl(180, 100%, 60%)",
    "selection": "hsla(180, 100%, 60%, 0.2)",

    "semantic": {
      "metricNormal": "hsl(0, 0%, 90%)",
      "metricWarn": "hsl(45, 100%, 50%)",
      "metricCrit": "hsl(0, 100%, 55%)",
      "governancePermit": "hsl(120, 100%, 40%)",
      "governanceDeny": "hsl(0, 100%, 55%)",
      "chainEvent": "hsl(180, 100%, 80%)",
      "agentRunning": "hsl(120, 100%, 40%)",
      "agentIdle": "hsl(0, 0%, 50%)",
      "agentStopped": "hsl(0, 100%, 55%)",
      "codeBackground": "hsl(0, 0%, 3%)",
      "codeForeground": "hsl(0, 0%, 85%)"
    }
  },

  "typography": {
    "fontFamily": {
      "sans": "Inter, system-ui, sans-serif",
      "mono": "JetBrains Mono, Fira Code, monospace",
      "heading": "Inter, system-ui, sans-serif"
    },
    "fontSize": {
      "xs": "0.75rem", "sm": "0.875rem", "base": "1rem",
      "lg": "1.125rem", "xl": "1.25rem", "2xl": "1.5rem",
      "3xl": "2.25rem", "4xl": "3rem"
    },
    "fontWeight": { "normal": 400, "medium": 500, "semibold": 600, "bold": 700 },
    "lineHeight": { "tight": 1.1, "snug": 1.3, "normal": 1.5, "relaxed": 1.6 }
  },

  "effects": {
    "shadow": {
      "none": "none",
      "sm": "0 1px 2px rgba(0, 0, 0, 0.5)",
      "md": "0 4px 6px rgba(0, 0, 0, 0.5)",
      "lg": "0 10px 15px rgba(0, 0, 0, 0.5)",
      "card": "0 2px 8px rgba(0, 0, 0, 0.4)"
    },
    "glow": {
      "none": "none",
      "subtle": "0 0 8px hsla(180, 100%, 60%, 0.15)",
      "medium": "0 0 16px hsla(180, 100%, 60%, 0.25)",
      "strong": "0 0 24px hsla(180, 100%, 60%, 0.4)",
      "success": "0 0 12px hsla(120, 100%, 40%, 0.3)",
      "warning": "0 0 12px hsla(45, 100%, 50%, 0.3)",
      "error": "0 0 12px hsla(0, 100%, 55%, 0.3)"
    }
  },

  "console": {
    "ansi": {
      "black": "#050505",
      "red": "#ff3333",
      "green": "#00ff00",
      "yellow": "#ffff00",
      "blue": "#00aaff",
      "magenta": "#ff00ff",
      "cyan": "#00ffff",
      "white": "#e6e6e6",
      "brightBlack": "#333333",
      "brightRed": "#ff6666",
      "brightGreen": "#66ff66",
      "brightYellow": "#ffff66",
      "brightBlue": "#66ccff",
      "brightMagenta": "#ff66ff",
      "brightCyan": "#66ffff",
      "brightWhite": "#ffffff"
    },
    "background": "#050505",
    "foreground": "#e6e6e6",
    "cursor": "#00ffff",
    "cursorAccent": "#050505",
    "selectionBackground": "rgba(0, 255, 255, 0.2)",
    "prompt": {
      "format": "{symbol} {path} {gitBranch} {kernelStatus} ",
      "tokens": {
        "symbol": { "color": "cyan", "bold": true, "text": ">" },
        "path": { "color": "magenta", "bold": false },
        "gitBranch": { "color": "green", "bold": false, "prefix": "[", "suffix": "]" },
        "kernelStatus": { "color": "yellow", "bold": false, "prefix": "(", "suffix": ")" }
      }
    },
    "panel": {
      "border": "single",
      "borderColor": "cyan",
      "titleColor": "magenta",
      "titleBold": true,
      "padding": 1
    },
    "table": {
      "borderStyle": "single",
      "headerColor": "cyan",
      "headerBold": true,
      "rowAlternateBackground": false,
      "alternateColor": "brightBlack"
    }
  },

  "hud": {
    "foreground": "#00ffff",
    "background": "#000000",
    "warningForeground": "#ffff00",
    "errorForeground": "#ff0000",
    "mutedForeground": "#006666",
    "headerSeparator": "=",
    "footerSeparator": "-",
    "selectionIndicator": ">",
    "progressFilled": "#",
    "progressEmpty": ".",
    "progressBrackets": ["[", "]"]
  }
}
```

### 7.3 paper-light

Clean light theme for documentation, reports, and daytime use.

```json
{
  "name": "paper-light",
  "version": "1.0.0",
  "displayName": "Paper Light",
  "description": "Clean light theme optimized for readability and documentation.",
  "author": "WeaveLogic",
  "license": "Apache-2.0",
  "mode": "light",

  "colors": {
    "background": "hsl(0, 0%, 100%)",
    "foreground": "hsl(220, 20%, 15%)",
    "surface": "hsl(220, 20%, 97%)",
    "surfaceAlt": "hsl(220, 20%, 94%)",
    "border": "hsl(220, 20%, 85%)",
    "borderStrong": "hsl(220, 20%, 70%)",

    "primary": "hsl(220, 70%, 45%)",
    "primaryForeground": "hsl(0, 0%, 100%)",
    "secondary": "hsl(220, 20%, 92%)",
    "secondaryForeground": "hsl(220, 20%, 25%)",
    "accent": "hsl(220, 30%, 90%)",
    "accentForeground": "hsl(220, 20%, 25%)",

    "muted": "hsl(220, 20%, 94%)",
    "mutedForeground": "hsl(220, 10%, 50%)",

    "success": "hsl(152, 60%, 36%)",
    "successForeground": "hsl(0, 0%, 100%)",
    "warning": "hsl(38, 92%, 45%)",
    "warningForeground": "hsl(0, 0%, 100%)",
    "error": "hsl(0, 72%, 50%)",
    "errorForeground": "hsl(0, 0%, 100%)",
    "info": "hsl(210, 70%, 50%)",
    "infoForeground": "hsl(0, 0%, 100%)",

    "ring": "hsl(220, 70%, 45%)",
    "selection": "hsla(220, 70%, 45%, 0.15)",

    "semantic": {
      "metricNormal": "hsl(220, 20%, 15%)",
      "metricWarn": "hsl(38, 92%, 45%)",
      "metricCrit": "hsl(0, 72%, 50%)",
      "governancePermit": "hsl(152, 60%, 36%)",
      "governanceDeny": "hsl(0, 72%, 50%)",
      "chainEvent": "hsl(220, 20%, 25%)",
      "agentRunning": "hsl(152, 60%, 36%)",
      "agentIdle": "hsl(220, 10%, 50%)",
      "agentStopped": "hsl(0, 72%, 50%)",
      "codeBackground": "hsl(220, 20%, 96%)",
      "codeForeground": "hsl(220, 20%, 20%)"
    }
  },

  "typography": {
    "fontFamily": {
      "sans": "Inter, system-ui, sans-serif",
      "mono": "JetBrains Mono, Fira Code, monospace",
      "heading": "Inter, system-ui, sans-serif"
    },
    "fontSize": {
      "xs": "0.75rem", "sm": "0.875rem", "base": "1rem",
      "lg": "1.125rem", "xl": "1.25rem", "2xl": "1.5rem",
      "3xl": "2.25rem", "4xl": "3rem"
    },
    "fontWeight": { "normal": 400, "medium": 500, "semibold": 600, "bold": 700 },
    "lineHeight": { "tight": 1.1, "snug": 1.3, "normal": 1.5, "relaxed": 1.6 }
  },

  "effects": {
    "shadow": {
      "none": "none",
      "sm": "0 1px 2px rgba(0, 0, 0, 0.06)",
      "md": "0 4px 6px rgba(0, 0, 0, 0.08)",
      "lg": "0 10px 15px rgba(0, 0, 0, 0.1)",
      "card": "0 1px 3px rgba(0, 0, 0, 0.06)"
    },
    "glow": {
      "none": "none",
      "subtle": "0 0 4px hsla(220, 70%, 45%, 0.1)",
      "medium": "0 0 8px hsla(220, 70%, 45%, 0.15)",
      "strong": "0 0 16px hsla(220, 70%, 45%, 0.2)",
      "success": "0 0 8px hsla(152, 60%, 36%, 0.15)",
      "warning": "0 0 8px hsla(38, 92%, 45%, 0.15)",
      "error": "0 0 8px hsla(0, 72%, 50%, 0.15)"
    }
  },

  "console": {
    "ansi": {
      "black": "#1e1e2e",
      "red": "#d20f39",
      "green": "#40a02b",
      "yellow": "#df8e1d",
      "blue": "#1e66f5",
      "magenta": "#8839ef",
      "cyan": "#179299",
      "white": "#4c4f69",
      "brightBlack": "#6c6f85",
      "brightRed": "#d20f39",
      "brightGreen": "#40a02b",
      "brightYellow": "#df8e1d",
      "brightBlue": "#1e66f5",
      "brightMagenta": "#8839ef",
      "brightCyan": "#179299",
      "brightWhite": "#ccd0da"
    },
    "background": "#eff1f5",
    "foreground": "#4c4f69",
    "cursor": "#4c4f69",
    "cursorAccent": "#eff1f5",
    "selectionBackground": "rgba(30, 102, 245, 0.15)",
    "prompt": {
      "format": "{user}@{host}:{path} {gitBranch}\n{symbol} ",
      "tokens": {
        "user": { "color": "green", "bold": true },
        "host": { "color": "blue", "bold": false },
        "path": { "color": "cyan", "bold": true },
        "gitBranch": { "color": "magenta", "bold": false, "prefix": "git:(", "suffix": ")" },
        "symbol": { "color": "blue", "bold": true, "text": "$" }
      }
    },
    "panel": {
      "border": "rounded",
      "borderColor": "blue",
      "titleColor": "blue",
      "titleBold": true,
      "padding": 1
    },
    "table": {
      "borderStyle": "rounded",
      "headerColor": "blue",
      "headerBold": true,
      "rowAlternateBackground": true,
      "alternateColor": "brightBlack"
    }
  },

  "hud": {
    "foreground": "#333333",
    "background": "#ffffff",
    "warningForeground": "#996600",
    "errorForeground": "#cc0000",
    "mutedForeground": "#999999",
    "headerSeparator": "=",
    "footerSeparator": "-",
    "selectionIndicator": ">",
    "progressFilled": "=",
    "progressEmpty": " ",
    "progressBrackets": ["[", "]"]
  }
}
```

### 7.4 high-contrast

Accessibility-focused theme. Default for Mentra HUD. Meets WCAG AAA contrast ratios.

```json
{
  "name": "high-contrast",
  "version": "1.0.0",
  "displayName": "High Contrast",
  "description": "Maximum contrast for accessibility. Default theme for Mentra HUD.",
  "author": "WeaveLogic",
  "license": "Apache-2.0",
  "mode": "dark",

  "colors": {
    "background": "hsl(0, 0%, 0%)",
    "foreground": "hsl(0, 0%, 100%)",
    "surface": "hsl(0, 0%, 5%)",
    "surfaceAlt": "hsl(0, 0%, 8%)",
    "border": "hsl(0, 0%, 100%)",
    "borderStrong": "hsl(0, 0%, 100%)",

    "primary": "hsl(60, 100%, 50%)",
    "primaryForeground": "hsl(0, 0%, 0%)",
    "secondary": "hsl(0, 0%, 20%)",
    "secondaryForeground": "hsl(0, 0%, 100%)",
    "accent": "hsl(180, 100%, 50%)",
    "accentForeground": "hsl(0, 0%, 0%)",

    "muted": "hsl(0, 0%, 15%)",
    "mutedForeground": "hsl(0, 0%, 75%)",

    "success": "hsl(120, 100%, 50%)",
    "successForeground": "hsl(0, 0%, 0%)",
    "warning": "hsl(60, 100%, 50%)",
    "warningForeground": "hsl(0, 0%, 0%)",
    "error": "hsl(0, 100%, 50%)",
    "errorForeground": "hsl(0, 0%, 100%)",
    "info": "hsl(180, 100%, 50%)",
    "infoForeground": "hsl(0, 0%, 0%)",

    "ring": "hsl(60, 100%, 50%)",
    "selection": "hsla(60, 100%, 50%, 0.3)",

    "semantic": {
      "metricNormal": "hsl(0, 0%, 100%)",
      "metricWarn": "hsl(60, 100%, 50%)",
      "metricCrit": "hsl(0, 100%, 50%)",
      "governancePermit": "hsl(120, 100%, 50%)",
      "governanceDeny": "hsl(0, 100%, 50%)",
      "chainEvent": "hsl(180, 100%, 50%)",
      "agentRunning": "hsl(120, 100%, 50%)",
      "agentIdle": "hsl(0, 0%, 75%)",
      "agentStopped": "hsl(0, 100%, 50%)",
      "codeBackground": "hsl(0, 0%, 3%)",
      "codeForeground": "hsl(0, 0%, 100%)"
    }
  },

  "typography": {
    "fontFamily": {
      "sans": "Inter, system-ui, sans-serif",
      "mono": "JetBrains Mono, Fira Code, monospace",
      "heading": "Inter, system-ui, sans-serif"
    },
    "fontSize": {
      "xs": "0.875rem", "sm": "1rem", "base": "1.125rem",
      "lg": "1.25rem", "xl": "1.5rem", "2xl": "1.75rem",
      "3xl": "2.5rem", "4xl": "3.25rem"
    },
    "fontWeight": { "normal": 400, "medium": 500, "semibold": 600, "bold": 700 },
    "lineHeight": { "tight": 1.2, "snug": 1.4, "normal": 1.6, "relaxed": 1.8 }
  },

  "borders": {
    "radius": {
      "none": "0px", "sm": "2px", "md": "4px",
      "lg": "6px", "xl": "8px", "full": "9999px"
    },
    "width": {
      "none": "0px", "thin": "2px", "medium": "3px", "thick": "4px"
    }
  },

  "effects": {
    "shadow": {
      "none": "none", "sm": "none", "md": "none", "lg": "none", "card": "none"
    },
    "glow": {
      "none": "none",
      "subtle": "0 0 4px hsla(60, 100%, 50%, 0.3)",
      "medium": "0 0 8px hsla(60, 100%, 50%, 0.4)",
      "strong": "0 0 12px hsla(60, 100%, 50%, 0.5)",
      "success": "0 0 8px hsla(120, 100%, 50%, 0.4)",
      "warning": "0 0 8px hsla(60, 100%, 50%, 0.4)",
      "error": "0 0 8px hsla(0, 100%, 50%, 0.4)"
    }
  },

  "animation": {
    "duration": {
      "instant": "0ms", "fast": "0ms", "normal": "0ms", "slow": "0ms", "glacial": "0ms"
    },
    "easing": {
      "linear": "linear", "ease": "linear", "easeIn": "linear",
      "easeOut": "linear", "easeInOut": "linear", "spring": "linear"
    }
  },

  "console": {
    "ansi": {
      "black": "#000000",
      "red": "#ff0000",
      "green": "#00ff00",
      "yellow": "#ffff00",
      "blue": "#0088ff",
      "magenta": "#ff00ff",
      "cyan": "#00ffff",
      "white": "#ffffff",
      "brightBlack": "#666666",
      "brightRed": "#ff3333",
      "brightGreen": "#33ff33",
      "brightYellow": "#ffff33",
      "brightBlue": "#3399ff",
      "brightMagenta": "#ff33ff",
      "brightCyan": "#33ffff",
      "brightWhite": "#ffffff"
    },
    "background": "#000000",
    "foreground": "#ffffff",
    "cursor": "#ffff00",
    "cursorAccent": "#000000",
    "selectionBackground": "rgba(255, 255, 0, 0.3)",
    "prompt": {
      "format": "{symbol} {path} ",
      "tokens": {
        "symbol": { "color": "yellow", "bold": true, "text": ">" },
        "path": { "color": "white", "bold": true }
      }
    },
    "panel": {
      "border": "heavy",
      "borderColor": "white",
      "titleColor": "yellow",
      "titleBold": true,
      "padding": 1
    },
    "table": {
      "borderStyle": "heavy",
      "headerColor": "yellow",
      "headerBold": true,
      "rowAlternateBackground": false,
      "alternateColor": "brightBlack"
    }
  },

  "hud": {
    "foreground": "#ffffff",
    "background": "#000000",
    "warningForeground": "#ffff00",
    "errorForeground": "#ff0000",
    "mutedForeground": "#999999",
    "headerSeparator": "=",
    "footerSeparator": "=",
    "selectionIndicator": ">>",
    "progressFilled": "#",
    "progressEmpty": "-",
    "progressBrackets": ["[", "]"]
  },

  "voice": {
    "pace": "slow",
    "pitch": "medium",
    "emphasisStyle": "volume",
    "pauseAfterHeading": "700ms",
    "pauseAfterParagraph": "500ms",
    "errorTone": "low",
    "successTone": "bright",
    "codeReadStyle": "summarize"
  }
}
```

Key accessibility features:
- All animations disabled (duration 0ms, linear easing) for users with vestibular disorders.
- Font sizes bumped up by one step (base is 1.125rem instead of 1rem).
- Border widths doubled for visibility.
- No subtle shadows; glow effects use higher opacity for clear focus indication.
- Heavy border style for panels and tables.
- Pure black/white with saturated primaries (yellow, cyan, green, red) for maximum contrast.

---

## 8. Console Prompt Theming

The WeftOS console prompt is itself a themed element. The `console.prompt` section in the theme definition controls its appearance.

### 8.1 Prompt Format Tokens

| Token | Source | Description |
|-------|--------|-------------|
| `{user}` | OS username or WeftOS session user | Current user identity |
| `{host}` | Hostname or WeftOS instance name | Machine/instance identifier |
| `{path}` | Current working directory (shortened) | File system location |
| `{gitBranch}` | Git branch name (if in a git repo) | Version control context |
| `{kernelStatus}` | Kernel state: running, degraded, stopped | Kernel health indicator |
| `{kernelVersion}` | WeftOS kernel version string | Version display |
| `{chainHeight}` | Current ExoChain block height | Chain state |
| `{agentCount}` | Number of active agent processes | Process summary |
| `{peerCount}` | Number of connected mesh peers | Network state |
| `{symbol}` | Static prompt symbol (configurable) | Command entry indicator |
| `{time}` | Current time (HH:MM) | Timestamp |
| `{exitCode}` | Exit code of last command (0 = green, nonzero = red) | Error state |

### 8.2 Prompt Token Styling

Each token in `console.prompt.tokens` accepts:

| Property | Type | Description |
|----------|------|-------------|
| `color` | ANSI color name | Text color (maps to `console.ansi.*`) |
| `bold` | boolean | Bold weight |
| `italic` | boolean | Italic style |
| `prefix` | string | Static text before the resolved value |
| `suffix` | string | Static text after the resolved value |
| `text` | string | Override: use this text instead of the resolved value |
| `hideIf` | string | Hide this token if condition is met: `"empty"`, `"zero"`, `"offline"` |

### 8.3 Prompt Examples by Theme

**ocean-dark**:
```
aepod@weftos:~/project git:(feature/sprint-11) [running]
$
```

**midnight**:
```
> ~/project [feature/sprint-11] (running)
```

**paper-light**:
```
aepod@weftos:~/project git:(main)
$
```

**high-contrast**:
```
> ~/project
```

### 8.4 Weaver-Generated Console Prompts

The Weaver can generate prompt themes as part of workspace customization. A user request like "make my prompt show chain height and agent count" produces:

```json
{
  "console": {
    "prompt": {
      "format": "{symbol} {path} {chainHeight} {agentCount} ",
      "tokens": {
        "symbol": { "color": "green", "bold": true, "text": "$" },
        "path": { "color": "cyan", "bold": true },
        "chainHeight": { "color": "yellow", "bold": false, "prefix": "chain:", "hideIf": "zero" },
        "agentCount": { "color": "magenta", "bold": false, "prefix": "agents:", "hideIf": "zero" }
      }
    }
  }
}
```

This is a partial theme that merges into the active theme as an override.

### 8.5 Syntax Highlighting in Console Output

The `console.syntaxHighlighting` tokens control how command output is colored when the output contains code or structured data. The renderer detects output type via `DisplayHint` and applies the appropriate highlighting.

| Context | Highlighting Applied |
|---------|---------------------|
| Shell command output (plain text) | No highlighting |
| `DisplayHint::Code(language)` | Language-specific highlighting using `syntaxHighlighting` tokens |
| `DisplayHint::Table` | Header row uses `console.table.headerColor`, borders use `console.panel.borderColor` |
| `DisplayHint::JSON` | JSON syntax highlighting (strings, numbers, keys, brackets) |
| `DisplayHint::Diff` | Added lines in `green`, removed lines in `red`, context in `white` |
| `DisplayHint::Error` | Error text in `red`, stack trace in `brightBlack` |

---

## 9. Implementation Plan

### 9.1 File Structure

```
gui/src/
  theme/
    types.ts              # ThemeDefinition, ResolvedTheme, ThemeOverride types
    schema.ts             # Zod schema for theme validation
    resolver-web.ts       # CSS custom property generation
    resolver-terminal.ts  # xterm.js ITheme + ANSI escape generation
    resolver-hud.ts       # Monochrome palette extraction
    resolver-voice.ts     # SSML hint generation
    context.tsx           # React ThemeContext provider
    loader.ts             # Theme file discovery and loading
    defaults/
      ocean-dark.json     # Built-in theme
      midnight.json       # Built-in theme
      paper-light.json    # Built-in theme
      high-contrast.json  # Built-in theme

docs/weftos/specs/
  theming-system.md       # This specification
  theme-schema.json       # JSON Schema (extracted from Section 3)
```

### 9.2 Block Descriptor Schema Changes

Add to `block-descriptor-schema.json` in the `BlockElement` properties:

```json
"theme": {
  "type": "object",
  "additionalProperties": false,
  "properties": {
    "override": {
      "type": "object",
      "description": "Dot-notation token path to override value map.",
      "additionalProperties": true
    },
    "inherit": {
      "type": "boolean",
      "default": true,
      "description": "Whether this block inherits parent theme context."
    }
  }
}
```

### 9.3 Zustand Integration

The `usePreferencesStore` (defined in Track 4, TD-5) stores the active theme name and user overrides:

```typescript
interface PreferencesStore {
  activeTheme: string;          // Theme name, e.g., "ocean-dark"
  themeOverrides: Record<string, unknown>;  // User-level overrides
  resolvedTheme: ResolvedTheme; // Computed from activeTheme + overrides
  setTheme: (name: string) => void;
  setOverride: (path: string, value: unknown) => void;
}
```

When `activeTheme` or `themeOverrides` change, the store recomputes `resolvedTheme` and the web resolver regenerates CSS custom properties.

### 9.4 Tailwind CSS 4 Integration

The theme tokens are injected as CSS custom properties that Tailwind references. The integration requires a generated CSS file that maps `--weftos-*` vars to Tailwind `@theme` tokens.

File: `gui/src/theme/tailwind-bridge.css` (auto-generated by `resolver-web.ts` on theme change)

```css
@theme {
  --color-weftos-bg: var(--weftos-color-background);
  --color-weftos-fg: var(--weftos-color-foreground);
  --color-weftos-surface: var(--weftos-color-surface);
  --color-weftos-primary: var(--weftos-color-primary);
  --color-weftos-accent: var(--weftos-color-accent);
  --color-weftos-muted: var(--weftos-color-muted);
  --color-weftos-success: var(--weftos-color-success);
  --color-weftos-warning: var(--weftos-color-warning);
  --color-weftos-error: var(--weftos-color-error);
  --color-weftos-info: var(--weftos-color-info);
  --color-weftos-ring: var(--weftos-color-ring);
  --color-weftos-border: var(--weftos-color-border);
  --font-weftos-sans: var(--weftos-font-family-sans);
  --font-weftos-mono: var(--weftos-font-family-mono);
  --radius-weftos-sm: var(--weftos-border-radius-sm);
  --radius-weftos-md: var(--weftos-border-radius-md);
  --radius-weftos-lg: var(--weftos-border-radius-lg);
}
```

Components then use: `className="bg-weftos-bg text-weftos-fg border-weftos-border rounded-weftos-md"`.

### 9.5 Sprint Scope

**Sprint 12 (K8.1 scope)**:
- `types.ts` and `schema.ts` (theme type definitions and Zod validation)
- `ocean-dark.json` (single built-in theme)
- `resolver-web.ts` (CSS custom property injection)
- `context.tsx` (React ThemeContext)
- `usePreferencesStore` with `activeTheme` field
- Block descriptor schema update (add `theme` field)

**Sprint 13 (K8.2 scope)**:
- `resolver-terminal.ts` (xterm.js ITheme mapping for ConsolePan)
- Console prompt rendering with theme tokens

**Sprint 17-18 (K8.5 scope)**:
- `midnight.json`, `paper-light.json`, `high-contrast.json` (additional themes)
- Console syntax highlighting integration
- Console panel and table theming
- `loader.ts` (file-based theme discovery)
- `weftos theme` command set

**Sprint 19-20 (K8.6 scope)**:
- `resolver-hud.ts` (Mentra HUD monochrome mapping)
- `resolver-voice.ts` (TTS hints)
- Theme marketplace integration
- User-created theme workflow
- Weaver theme generation

### 9.6 Migration from Current CSS

The current `global.css` imports Fumadocs presets:

```css
@import 'tailwindcss';
@import 'fumadocs-ui/css/preset.css';
@import 'fumadocs-ui/css/ocean.css';
```

This remains unchanged for the docs site. The Tauri GUI application has its own CSS entry point that imports:

```css
@import 'tailwindcss';
@import '../theme/tailwind-bridge.css';
```

The Fumadocs docs site and the Tauri GUI are separate applications. They share the brand palette (via the brand identity spec) but use different theming mechanisms. The docs site uses Fumadocs's built-in theming; the GUI uses the WeftOS theming system described here.

---

## 10. Research Notes: Hermes Agent Styling Patterns

The Hermes agent project (hermes-agent on GitHub) uses Python's `rich` library for terminal output. While Hermes is not a dependency of WeftOS, its patterns informed several design decisions in this spec.

### Hermes Patterns Observed

1. **Rich Console with Panels**: Hermes uses `rich.panel.Panel` for bordered output sections, `rich.table.Table` for structured data, and `rich.console.Console` for styled text output. WeftOS maps these to the `console.panel` and `console.table` theme tokens.

2. **ANSI Color Palette**: Hermes relies on Rich's default theme which maps semantic names (like `"info"`, `"warning"`, `"error"`) to ANSI colors. WeftOS adopts the same pattern with the `console.ansi` palette and `colors.semantic` mapping.

3. **No External Theme Configuration**: Hermes does not expose `~/.hermes/theme.json` or equivalent. All styling is hardcoded in the Python source via Rich style strings. WeftOS explicitly solves this gap by making theming a first-class, user-configurable system.

4. **Workspace GUI Projects**: Community projects like hermes-workspace and mission-control use standard web frameworks (React, Tailwind) with their own CSS. They do not inherit terminal styling from the agent. WeftOS avoids this disconnect by having a single theme definition that covers both terminal and web rendering.

### Design Decisions Influenced by Hermes

| Hermes Pattern | WeftOS Decision |
|---------------|----------------|
| Rich panels with Unicode borders | `console.panel.border` supports single, double, rounded, heavy, none |
| Rich table formatting | `console.table` token group with header color, border style, alternating rows |
| Hardcoded color styles | All colors are tokens in the theme JSON, never hardcoded |
| No theme file | Full theme file system with discovery chain and user overrides |
| Terminal-only styling | Multi-target: same theme covers web, terminal, HUD, voice, MCP |
