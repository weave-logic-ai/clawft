//! Rust source-to-MDX converter for WeftOS API documentation.
//!
//! Parses `//!` (crate/module) and `///` (item) doc comments plus public
//! item signatures from Rust source files, then emits Fumadocs-compatible
//! MDX pages.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

// ── CLI ─────────────────────────────────────────────────────────────────

fn usage() -> ! {
    eprintln!(
        "Usage: rustdoc-mdx --crate-dir <path> --output <dir>\n\n\
         Reads Rust source files from <path>/src/ and writes MDX API docs\n\
         into <dir>/<crate-name>.mdx.\n\n\
         Options:\n  \
           --crate-dir <path>   Root of the crate (must contain src/)\n  \
           --output <dir>       Output directory for MDX files\n  \
           --index-only         Only regenerate the index.mdx file"
    );
    process::exit(1);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let mut crate_dir: Option<PathBuf> = None;
    let mut output_dir: Option<PathBuf> = None;
    let mut index_only = false;
    let mut i = 0;

    while i < args.len() {
        match args[i].as_str() {
            "--crate-dir" => {
                i += 1;
                crate_dir = Some(PathBuf::from(args.get(i).unwrap_or_else(|| {
                    eprintln!("error: --crate-dir requires a value");
                    process::exit(1);
                })));
            }
            "--output" => {
                i += 1;
                output_dir = Some(PathBuf::from(args.get(i).unwrap_or_else(|| {
                    eprintln!("error: --output requires a value");
                    process::exit(1);
                })));
            }
            "--index-only" => {
                index_only = true;
            }
            "--help" | "-h" => usage(),
            other => {
                eprintln!("error: unknown argument: {other}");
                usage();
            }
        }
        i += 1;
    }

    let output = output_dir.unwrap_or_else(|| {
        eprintln!("error: --output is required");
        process::exit(1);
    });
    fs::create_dir_all(&output).expect("failed to create output directory");

    if index_only {
        generate_index(&output);
        return;
    }

    let crate_path = crate_dir.unwrap_or_else(|| {
        eprintln!("error: --crate-dir is required (unless --index-only)");
        process::exit(1);
    });

    let crate_name = crate_name_from_path(&crate_path);
    let src_dir = crate_path.join("src");

    if !src_dir.is_dir() {
        eprintln!("error: {}/src/ does not exist", crate_path.display());
        process::exit(1);
    }

    let crate_mod = parse_crate(&src_dir);
    let mdx = render_mdx(&crate_name, &crate_mod);

    let out_file = output.join(format!("{crate_name}.mdx"));
    fs::write(&out_file, mdx).expect("failed to write MDX file");
    eprintln!("wrote {}", out_file.display());
}

// ── Data model ──────────────────────────────────────────────────────────

#[derive(Debug, Default)]
struct CrateDoc {
    /// Crate-level doc comment (`//!` lines from lib.rs).
    module_doc: String,
    /// Public items grouped by kind.
    items: Vec<Item>,
    /// Sub-modules with their own items (from separate .rs files).
    submodules: BTreeMap<String, ModuleDoc>,
}

#[derive(Debug, Default)]
struct ModuleDoc {
    doc: String,
    items: Vec<Item>,
}

#[derive(Debug)]
struct Item {
    kind: ItemKind,
    signature: String,
    name: String,
    doc: String,
    fields: Vec<Field>,
    variants: Vec<Variant>,
}

#[derive(Debug)]
struct Field {
    name: String,
    ty: String,
    doc: String,
}

#[derive(Debug)]
struct Variant {
    name: String,
    doc: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ItemKind {
    Function,
    Struct,
    Enum,
    Trait,
    TypeAlias,
    Constant,
}

impl ItemKind {
    fn heading(self) -> &'static str {
        match self {
            Self::Function => "Functions",
            Self::Struct => "Structs",
            Self::Enum => "Enums",
            Self::Trait => "Traits",
            Self::TypeAlias => "Type Aliases",
            Self::Constant => "Constants",
        }
    }
}

// ── Parsing ─────────────────────────────────────────────────────────────

fn crate_name_from_path(path: &Path) -> String {
    // Read Cargo.toml to extract the actual package name.
    let cargo_toml = path.join("Cargo.toml");
    if let Ok(contents) = fs::read_to_string(&cargo_toml) {
        for line in contents.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("name") {
                if let Some(val) = trimmed.split('=').nth(1) {
                    let name = val.trim().trim_matches('"');
                    if !name.is_empty() {
                        return name.to_string();
                    }
                }
            }
        }
    }
    // Fallback: directory name.
    path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string()
}

fn parse_crate(src_dir: &Path) -> CrateDoc {
    let mut crate_doc = CrateDoc::default();

    // Parse lib.rs (or main.rs as fallback).
    let root_file = if src_dir.join("lib.rs").exists() {
        src_dir.join("lib.rs")
    } else if src_dir.join("main.rs").exists() {
        src_dir.join("main.rs")
    } else {
        return crate_doc;
    };

    let source = fs::read_to_string(&root_file).unwrap_or_default();
    let (module_doc, items) = parse_source(&source);
    crate_doc.module_doc = module_doc;
    crate_doc.items = items;

    // Parse submodule files (*.rs in src/, excluding lib.rs/main.rs).
    if let Ok(entries) = fs::read_dir(src_dir) {
        let mut files: Vec<_> = entries
            .filter_map(|e| e.ok())
            .filter(|e| {
                let name = e.file_name();
                let n = name.to_string_lossy();
                n.ends_with(".rs") && n != "lib.rs" && n != "main.rs"
            })
            .collect();
        files.sort_by_key(|e| e.file_name());

        for entry in files {
            let fname = entry.file_name();
            let mod_name = fname.to_string_lossy().trim_end_matches(".rs").to_string();
            if let Ok(src) = fs::read_to_string(entry.path()) {
                let (doc, items) = parse_source(&src);
                if !doc.is_empty() || !items.is_empty() {
                    crate_doc
                        .submodules
                        .insert(mod_name, ModuleDoc { doc, items });
                }
            }
        }
    }

    // Parse submodule directories (src/foo/mod.rs).
    if let Ok(entries) = fs::read_dir(src_dir) {
        let mut dirs: Vec<_> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .collect();
        dirs.sort_by_key(|e| e.file_name());

        for entry in dirs {
            let dir_name = entry.file_name().to_string_lossy().to_string();
            let mod_file = entry.path().join("mod.rs");
            if mod_file.exists() {
                if let Ok(src) = fs::read_to_string(&mod_file) {
                    let (doc, items) = parse_source(&src);
                    if !doc.is_empty() || !items.is_empty() {
                        let entry = crate_doc
                            .submodules
                            .entry(dir_name)
                            .or_default();
                        entry.doc = doc;
                        entry.items.extend(items);
                    }
                }
            }
        }
    }

    crate_doc
}

/// Parse a single Rust source file into module-level docs and public items.
fn parse_source(source: &str) -> (String, Vec<Item>) {
    let lines: Vec<&str> = source.lines().collect();
    let mut module_doc = String::new();
    let mut items = Vec::new();
    let mut idx = 0;

    // Collect module-level doc comments (`//!`).
    while idx < lines.len() {
        let trimmed = lines[idx].trim();
        if trimmed.starts_with("//!") {
            let content = trimmed.strip_prefix("//!").unwrap_or("");
            let content = content.strip_prefix(' ').unwrap_or(content);
            module_doc.push_str(content);
            module_doc.push('\n');
        } else if trimmed.is_empty() && module_doc.is_empty() {
            // Skip leading blank lines.
        } else if !trimmed.starts_with("//!") {
            break;
        }
        idx += 1;
    }

    // Scan for public items with preceding `///` doc comments.
    while idx < lines.len() {
        let trimmed = lines[idx].trim();

        // Collect `///` doc comment block.
        if trimmed.starts_with("///") && !trimmed.starts_with("////") {
            let doc_start = idx;
            let mut doc = String::new();
            while idx < lines.len() && lines[idx].trim().starts_with("///") {
                let line = lines[idx].trim();
                let content = line.strip_prefix("///").unwrap_or("");
                let content = content.strip_prefix(' ').unwrap_or(content);
                doc.push_str(content);
                doc.push('\n');
                idx += 1;
            }

            // Skip attribute lines (#[...]).
            while idx < lines.len() {
                let t = lines[idx].trim();
                if t.starts_with("#[") || t.starts_with("#![") || t.is_empty() {
                    idx += 1;
                } else {
                    break;
                }
            }

            // Check if this is a public item.
            if idx < lines.len() {
                let item_line = lines[idx].trim();
                if let Some(item) = try_parse_item(item_line, &doc, &lines, idx) {
                    items.push(item);
                }
            }
            let _ = doc_start; // used for context, consumed above
        }
        idx += 1;
    }

    (module_doc.trim_end().to_string(), items)
}

fn try_parse_item(line: &str, doc: &str, lines: &[&str], start: usize) -> Option<Item> {
    if !line.starts_with("pub ") && !line.starts_with("pub(crate)") {
        return None;
    }

    // Strip visibility qualifiers to get the item keyword.
    let after_pub = if line.starts_with("pub(crate)") {
        line.strip_prefix("pub(crate)")?.trim()
    } else {
        line.strip_prefix("pub")?.trim()
    };

    let (kind, rest) = if after_pub.starts_with("fn ") || after_pub.starts_with("async fn ") {
        (ItemKind::Function, after_pub)
    } else if after_pub.starts_with("struct ") {
        (ItemKind::Struct, after_pub.strip_prefix("struct ")?)
    } else if after_pub.starts_with("enum ") {
        (ItemKind::Enum, after_pub.strip_prefix("enum ")?)
    } else if after_pub.starts_with("trait ") {
        (ItemKind::Trait, after_pub.strip_prefix("trait ")?)
    } else if after_pub.starts_with("type ") {
        (ItemKind::TypeAlias, after_pub)
    } else if after_pub.starts_with("const ") {
        (ItemKind::Constant, after_pub)
    } else {
        return None;
    };

    // Skip pub(crate) items — we only document the fully public API.
    if line.starts_with("pub(crate)") {
        return None;
    }

    let name = extract_name(rest, kind);
    let signature = build_signature(line, lines, start);
    let fields = if kind == ItemKind::Struct {
        extract_fields(lines, start)
    } else {
        Vec::new()
    };
    let variants = if kind == ItemKind::Enum {
        extract_variants(lines, start)
    } else {
        Vec::new()
    };

    Some(Item {
        kind,
        signature,
        name,
        doc: doc.trim_end().to_string(),
        fields,
        variants,
    })
}

fn extract_name(rest: &str, kind: ItemKind) -> String {
    let s = match kind {
        ItemKind::Function => {
            // "fn foo(" or "async fn foo("
            let after_fn = if rest.starts_with("async fn ") {
                &rest[9..]
            } else if rest.starts_with("fn ") {
                &rest[3..]
            } else {
                rest
            };
            after_fn
                .split(|c: char| !c.is_alphanumeric() && c != '_')
                .next()
                .unwrap_or(rest)
        }
        _ => rest
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .next()
            .unwrap_or(rest),
    };
    s.to_string()
}

/// Build the full signature, handling multi-line signatures.
fn build_signature(first_line: &str, lines: &[&str], start: usize) -> String {
    let mut sig = first_line.to_string();

    // If the line doesn't end with `{`, `;`, or `)`, collect continuation lines.
    let terminators = ['{', ';', ')'];
    if !terminators.iter().any(|t| sig.trim_end().ends_with(*t)) {
        let mut j = start + 1;
        while j < lines.len() {
            let l = lines[j].trim();
            sig.push(' ');
            sig.push_str(l);
            if terminators.iter().any(|t| l.ends_with(*t)) || l.is_empty() {
                break;
            }
            j += 1;
        }
    }

    // Trim trailing `{` and whitespace for display.
    let sig = sig.trim_end().trim_end_matches('{').trim_end();
    sig.to_string()
}

/// Extract fields from a struct definition with doc comments.
fn extract_fields(lines: &[&str], start: usize) -> Vec<Field> {
    let mut fields = Vec::new();
    let mut idx = start;

    // Find opening brace.
    while idx < lines.len() && !lines[idx].contains('{') {
        idx += 1;
    }
    if idx >= lines.len() {
        return fields;
    }
    idx += 1; // skip `{` line

    let mut current_doc = String::new();
    while idx < lines.len() {
        let trimmed = lines[idx].trim();
        if trimmed == "}" || trimmed.starts_with("}") {
            break;
        }
        if trimmed.starts_with("///") {
            let content = trimmed.strip_prefix("///").unwrap_or("");
            let content = content.strip_prefix(' ').unwrap_or(content);
            if !current_doc.is_empty() {
                current_doc.push(' ');
            }
            current_doc.push_str(content);
        } else if trimmed.starts_with("#[") || trimmed.is_empty() {
            // Skip attributes and blank lines.
        } else if trimmed.starts_with("pub ") {
            // Parse: `pub field_name: Type,`
            let field_part = trimmed
                .strip_prefix("pub ")
                .unwrap_or(trimmed)
                .trim_end_matches(',');
            if let Some((name, ty)) = field_part.split_once(':') {
                fields.push(Field {
                    name: name.trim().to_string(),
                    ty: ty.trim().to_string(),
                    doc: std::mem::take(&mut current_doc),
                });
            }
        } else {
            // Non-pub field or other line — reset doc.
            current_doc.clear();
        }
        idx += 1;
    }
    fields
}

/// Extract variants from an enum definition with doc comments.
fn extract_variants(lines: &[&str], start: usize) -> Vec<Variant> {
    let mut variants = Vec::new();
    let mut idx = start;

    // Find opening brace.
    while idx < lines.len() && !lines[idx].contains('{') {
        idx += 1;
    }
    if idx >= lines.len() {
        return variants;
    }
    idx += 1;

    let mut current_doc = String::new();
    let mut brace_depth: i32 = 0;

    while idx < lines.len() {
        let trimmed = lines[idx].trim();

        // Track nested braces to skip struct variant bodies.
        if brace_depth > 0 {
            brace_depth += trimmed.matches('{').count() as i32;
            brace_depth -= trimmed.matches('}').count() as i32;
            idx += 1;
            continue;
        }

        if trimmed == "}" {
            break;
        }
        if trimmed.starts_with("///") {
            let content = trimmed.strip_prefix("///").unwrap_or("");
            let content = content.strip_prefix(' ').unwrap_or(content);
            if !current_doc.is_empty() {
                current_doc.push(' ');
            }
            current_doc.push_str(content);
        } else if trimmed.starts_with("#[") || trimmed.is_empty() {
            // Skip attributes and blank lines.
        } else {
            // This should be a variant line.
            let variant_name = trimmed
                .split(|c: char| !c.is_alphanumeric() && c != '_')
                .next()
                .unwrap_or("")
                .to_string();
            if !variant_name.is_empty() {
                variants.push(Variant {
                    name: variant_name,
                    doc: std::mem::take(&mut current_doc),
                });
            }

            // Track opening braces on this line.
            let opens = trimmed.matches('{').count() as i32;
            let closes = trimmed.matches('}').count() as i32;
            brace_depth += opens - closes;
        }
        idx += 1;
    }
    variants
}

// ── MDX rendering ───────────────────────────────────────────────────────

fn render_mdx(crate_name: &str, crate_doc: &CrateDoc) -> String {
    let mut out = String::with_capacity(8192);

    // Extract the first non-heading paragraph of the module doc as the description.
    let description = crate_doc
        .module_doc
        .lines()
        .skip_while(|l| l.trim().is_empty() || l.trim().starts_with('#'))
        .take_while(|l| !l.trim().is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let description = description.trim();

    // Frontmatter.
    writeln!(out, "---").unwrap();
    writeln!(out, "title: \"{crate_name}\"").unwrap();
    if !description.is_empty() {
        // Escape quotes in description for YAML.
        let escaped = description.replace('"', "\\\"");
        writeln!(out, "description: \"{escaped}\"").unwrap();
    }
    writeln!(out, "---").unwrap();
    writeln!(out).unwrap();

    // Module-level documentation.
    if !crate_doc.module_doc.is_empty() {
        writeln!(out, "{}", crate_doc.module_doc).unwrap();
        writeln!(out).unwrap();
    }

    // Render top-level items grouped by kind.
    render_items(&mut out, &crate_doc.items);

    // Render submodules.
    for (mod_name, mod_doc) in &crate_doc.submodules {
        writeln!(out, "---\n").unwrap();
        writeln!(out, "## Module `{mod_name}`\n").unwrap();
        if !mod_doc.doc.is_empty() {
            writeln!(out, "{}\n", mod_doc.doc).unwrap();
        }
        render_items(&mut out, &mod_doc.items);
    }

    out
}

fn render_items(out: &mut String, items: &[Item]) {
    // Group items by kind, preserving the kind ordering.
    let mut by_kind: BTreeMap<ItemKind, Vec<&Item>> = BTreeMap::new();
    for item in items {
        by_kind.entry(item.kind).or_default().push(item);
    }

    for (kind, group) in &by_kind {
        writeln!(out, "## {}\n", kind.heading()).unwrap();
        for item in group {
            // Signature as heading.
            writeln!(out, "### `{}`\n", item.name).unwrap();

            // Signature in code block if it's non-trivial.
            if item.signature.len() > item.name.len() + 10 {
                writeln!(out, "```rust\n{}\n```\n", item.signature).unwrap();
            }

            // Doc comment body.
            if !item.doc.is_empty() {
                writeln!(out, "{}\n", item.doc).unwrap();
            }

            // Struct fields.
            if !item.fields.is_empty() {
                writeln!(out, "#### Fields\n").unwrap();
                for f in &item.fields {
                    if f.doc.is_empty() {
                        writeln!(out, "- `{}: {}`", f.name, f.ty).unwrap();
                    } else {
                        writeln!(out, "- `{}: {}` -- {}", f.name, f.ty, f.doc).unwrap();
                    }
                }
                writeln!(out).unwrap();
            }

            // Enum variants.
            if !item.variants.is_empty() {
                writeln!(out, "#### Variants\n").unwrap();
                for v in &item.variants {
                    if v.doc.is_empty() {
                        writeln!(out, "- `{}`", v.name).unwrap();
                    } else {
                        writeln!(out, "- `{}` -- {}", v.name, v.doc).unwrap();
                    }
                }
                writeln!(out).unwrap();
            }
        }
    }
}

// ── Index generation ────────────────────────────────────────────────────

fn generate_index(output_dir: &Path) {
    let mut entries: Vec<String> = Vec::new();

    if let Ok(dir) = fs::read_dir(output_dir) {
        let mut files: Vec<_> = dir
            .filter_map(|e| e.ok())
            .filter(|e| {
                let name = e.file_name();
                let n = name.to_string_lossy();
                n.ends_with(".mdx") && n != "index.mdx" && n != "meta.json"
            })
            .collect();
        files.sort_by_key(|e| e.file_name());

        for entry in files {
            let fname = entry.file_name();
            let stem = fname
                .to_string_lossy()
                .trim_end_matches(".mdx")
                .to_string();
            entries.push(stem);
        }
    }

    let mut out = String::new();
    writeln!(out, "---").unwrap();
    writeln!(out, "title: \"API Reference\"").unwrap();
    writeln!(out, "description: \"Auto-generated API reference for WeftOS crates.\"").unwrap();
    writeln!(out, "---").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "# API Reference\n").unwrap();
    writeln!(
        out,
        "Auto-generated from Rust source files. See each crate page for detailed type and function documentation.\n"
    )
    .unwrap();
    writeln!(out, "## Crates\n").unwrap();

    for name in &entries {
        writeln!(out, "- [`{name}`](/docs/api/{name})").unwrap();
    }
    writeln!(out).unwrap();

    let index_path = output_dir.join("index.mdx");
    fs::write(&index_path, out).expect("failed to write index.mdx");
    eprintln!("wrote {}", index_path.display());
}
