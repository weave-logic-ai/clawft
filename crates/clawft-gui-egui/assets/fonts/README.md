# Panel / shell font assets

| File | Purpose |
|------|---------|
| `DejaVuSans-WeftSymbols.ttf` | Icon glyph fallback (sidebar chips). Full-time, all targets. |
| `UbuntuLight-WeftLatin.ttf` | Wasm proportional UI face (WEFT-577 subset). |
| `Hack-WeftLatin.ttf` | Wasm monospace UI face (WEFT-577 subset). |

## Regenerating the Latin subsets

Sources are the faces shipped by `epaint_default_fonts` 0.34 (same as
egui's stock pack). Subset with [fonttools](https://github.com/fonttools/fonttools):

```bash
SRC_U=~/.cargo/registry/src/*/epaint_default_fonts-0.34.*/fonts/Ubuntu-Light.ttf
SRC_H=~/.cargo/registry/src/*/epaint_default_fonts-0.34.*/fonts/Hack-Regular.ttf
OUT=crates/clawft-gui-egui/assets/fonts
UNICODES='U+0020-007E,U+00A0-00FF,U+0100-017F,U+2000-206F,U+2190-21FF,U+2200-22FF,U+2500-257F,U+25A0-25FF,U+2600-26FF'

pyftsubset $SRC_U --unicodes="$UNICODES" --layout-features='kern,liga' \
  --no-hinting --desubroutinize --output-file=$OUT/UbuntuLight-WeftLatin.ttf
pyftsubset $SRC_H --unicodes="$UNICODES" --layout-features='kern,liga' \
  --no-hinting --desubroutinize --output-file=$OUT/Hack-WeftLatin.ttf
```

Native builds still use egui `default_fonts` (full faces + emoji).
