//! build-kb — Generate an RVF knowledge-base file from WeftOS MDX docs.
//!
//! Walks the docs directory, chunks each MDX file by heading, generates
//! deterministic hash-based embeddings (SHA-256 -> 384 floats), and writes
//! a binary `.rvf` segment file for the browser playground RAG pipeline.
//!
//! The output contains:
//! - One Meta segment (0x07) with CBOR-encoded corpus manifest
//! - One Vec segment (0x01) per document chunk with CBOR-encoded payload

use std::path::{Path, PathBuf};

use chrono::Utc;
use clap::Parser;
use regex::Regex;
use rvf_types::{SegmentFlags, SegmentType};
use serde::Serialize;
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

// -- CLI --------------------------------------------------------------------

#[derive(Parser)]
#[command(name = "build-kb", about = "Build an RVF knowledge base from MDX docs")]
struct Cli {
    /// Path to the docs content directory (e.g. docs/src/content/docs).
    #[arg(long)]
    docs_dir: PathBuf,

    /// Output path for the .rvf file.
    #[arg(long)]
    output: PathBuf,
}

// -- RVF payload types (CBOR-serialized) ------------------------------------

#[derive(Serialize)]
struct ManifestPayload {
    agent_id: String,
    namespace: String,
    segment_count: usize,
    dimension: u32,
    embedder_name: String,
    created_at: String,
    version: u32,
}

#[derive(Serialize)]
struct VecPayload {
    id: String,
    text: String,
    embedding: Vec<f32>,
    metadata: serde_json::Value,
    tags: Vec<String>,
    namespace: String,
    dimension: u32,
    embedder_name: String,
}

// -- Embedding --------------------------------------------------------------

const EMBEDDING_DIM: usize = 384;

/// Deterministic hash-based embedding (placeholder for real model).
///
/// SHA-256 of the input text is expanded to 384 floats in [-1, 1],
/// then L2-normalised. The shape matches all-MiniLM-L6-v2 so downstream
/// consumers need no changes when we swap in ONNX inference later.
fn hash_embed(text: &str) -> Vec<f32> {
    let hash = Sha256::digest(text.as_bytes());
    let mut embedding = Vec::with_capacity(EMBEDDING_DIM);
    for i in 0..EMBEDDING_DIM {
        let byte = hash[i % 32];
        let val = (byte as f32 / 127.5) - 1.0;
        let pos_factor = ((i as f32) * 0.01).sin();
        embedding.push(val * 0.5 + pos_factor * 0.5);
    }
    // L2 normalise.
    let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for v in &mut embedding {
            *v /= norm;
        }
    }
    embedding
}

// -- Frontmatter parsing ----------------------------------------------------

#[derive(Debug, Default)]
struct Frontmatter {
    title: String,
    #[allow(dead_code)]
    description: String,
}

/// Extract YAML frontmatter delimited by `---` lines.
fn parse_frontmatter(content: &str) -> (Frontmatter, &str) {
    let mut fm = Frontmatter::default();

    if !content.starts_with("---") {
        return (fm, content);
    }

    // Find closing `---`.
    let after_open = &content[3..];
    let close = match after_open.find("\n---") {
        Some(pos) => pos,
        None => return (fm, content),
    };

    let yaml_block = &after_open[..close];
    let body_start = 3 + close + 4; // skip opening `---` + yaml + `\n---`
    let body = if body_start < content.len() {
        &content[body_start..]
    } else {
        ""
    };

    // Simple key: "value" parser (avoids pulling in a YAML crate).
    for line in yaml_block.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("title:") {
            fm.title = unquote(rest.trim());
        } else if let Some(rest) = line.strip_prefix("description:") {
            fm.description = unquote(rest.trim());
        }
    }

    (fm, body)
}

fn unquote(s: &str) -> String {
    let s = s.trim();
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

// -- MDX stripping ----------------------------------------------------------

/// Remove MDX-specific syntax: import statements and JSX component tags.
fn strip_mdx(text: &str) -> String {
    let import_re = Regex::new(r"(?m)^import\s+.*$").unwrap();
    let jsx_re = Regex::new(r"</?[A-Z][A-Za-z0-9]*[^>]*/?>\s*").unwrap();

    let text = import_re.replace_all(text, "");
    let text = jsx_re.replace_all(&text, "");
    text.to_string()
}

// -- Chunking ---------------------------------------------------------------

#[derive(Debug)]
struct Chunk {
    heading: String,
    text: String,
}

/// Split markdown body into chunks by H2/H3 headings.
fn chunk_by_headings(body: &str) -> Vec<Chunk> {
    let heading_re = Regex::new(r"(?m)^(#{2,3})\s+(.+)$").unwrap();

    let mut chunks: Vec<Chunk> = Vec::new();
    let mut last_heading = String::new();
    let mut last_start: usize = 0;
    let mut first = true;

    for m in heading_re.find_iter(body) {
        let caps = heading_re.captures(&body[m.start()..]).unwrap();
        let heading_text = caps.get(2).unwrap().as_str().trim().to_string();

        if first {
            // Content before the first heading (intro).
            let intro = body[..m.start()].trim();
            if !intro.is_empty() {
                chunks.push(Chunk {
                    heading: String::new(),
                    text: intro.to_string(),
                });
            }
            first = false;
        } else {
            let section_text = body[last_start..m.start()].trim();
            if !section_text.is_empty() {
                chunks.push(Chunk {
                    heading: last_heading.clone(),
                    text: section_text.to_string(),
                });
            }
        }

        last_heading = heading_text;
        last_start = m.start();
    }

    // Remaining content after the last heading.
    let tail = body[last_start..].trim();
    if !tail.is_empty() {
        chunks.push(Chunk {
            heading: last_heading,
            text: tail.to_string(),
        });
    }

    // If no headings were found, treat entire body as one chunk.
    if chunks.is_empty() && !body.trim().is_empty() {
        chunks.push(Chunk {
            heading: String::new(),
            text: body.trim().to_string(),
        });
    }

    chunks
}

const MAX_CHUNK_CHARS: usize = 2000;

/// Split oversized chunks at paragraph boundaries.
fn split_large_chunks(chunks: Vec<Chunk>) -> Vec<Chunk> {
    let mut result = Vec::new();
    for chunk in chunks {
        if chunk.text.len() <= MAX_CHUNK_CHARS {
            result.push(chunk);
        } else {
            // Split at double-newline (paragraph) boundaries.
            let paragraphs: Vec<&str> = chunk.text.split("\n\n").collect();
            let mut current = String::new();
            let mut sub_idx = 0;
            for para in &paragraphs {
                if !current.is_empty() && current.len() + para.len() + 2 > MAX_CHUNK_CHARS {
                    result.push(Chunk {
                        heading: if sub_idx == 0 {
                            chunk.heading.clone()
                        } else {
                            format!("{} (cont.)", chunk.heading)
                        },
                        text: current.clone(),
                    });
                    current.clear();
                    sub_idx += 1;
                }
                if !current.is_empty() {
                    current.push_str("\n\n");
                }
                current.push_str(para);
            }
            if !current.is_empty() {
                result.push(Chunk {
                    heading: if sub_idx == 0 {
                        chunk.heading.clone()
                    } else {
                        format!("{} (cont.)", chunk.heading)
                    },
                    text: current,
                });
            }
        }
    }
    result
}

// -- Path helpers -----------------------------------------------------------

/// Derive URL source path from file path.
///
/// `content/docs/clawft/providers.mdx` -> `/docs/clawft/providers`
fn source_path(file_path: &Path, docs_dir: &Path) -> String {
    let rel = file_path.strip_prefix(docs_dir).unwrap_or(file_path);
    let without_ext = rel.with_extension("");
    let mut path_str = without_ext.to_string_lossy().replace('\\', "/");
    // Remove trailing /index.
    if path_str.ends_with("/index") {
        path_str = path_str[..path_str.len() - 6].to_string();
    }
    format!("/docs/{path_str}")
}

/// Derive a slug from a file path for use in segment IDs.
///
/// `content/docs/clawft/providers.mdx` -> `clawft-providers`
fn page_slug(file_path: &Path, docs_dir: &Path) -> String {
    let rel = file_path.strip_prefix(docs_dir).unwrap_or(file_path);
    let without_ext = rel.with_extension("");
    let mut slug = without_ext
        .to_string_lossy()
        .replace('\\', "/")
        .replace('/', "-");
    if slug.ends_with("-index") {
        slug = slug[..slug.len() - 6].to_string();
    }
    slug
}

/// Auto-generate tags from path segments.
fn tags_from_path(file_path: &Path, docs_dir: &Path) -> Vec<String> {
    let rel = file_path.strip_prefix(docs_dir).unwrap_or(file_path);
    rel.with_extension("")
        .iter()
        .map(|c| c.to_string_lossy().to_string())
        .filter(|s| s != "index")
        .collect()
}

/// Infer a category from the first path segment.
fn category_from_path(file_path: &Path, docs_dir: &Path) -> String {
    let rel = file_path.strip_prefix(docs_dir).unwrap_or(file_path);
    rel.iter()
        .next()
        .map(|c| c.to_string_lossy().to_string())
        .unwrap_or_else(|| "general".to_string())
}

// -- CBOR encoding helper ---------------------------------------------------

fn cbor_encode<T: Serialize>(value: &T) -> Vec<u8> {
    let mut buf = Vec::new();
    ciborium::into_writer(value, &mut buf).expect("CBOR serialization should not fail");
    buf
}

// -- Main -------------------------------------------------------------------

const BASE_URL: &str = "https://weftos.weavelogic.ai";
const EMBEDDER_NAME: &str = "hash-sha256";

fn main() {
    let cli = Cli::parse();

    if !cli.docs_dir.is_dir() {
        eprintln!(
            "Error: docs directory does not exist: {}",
            cli.docs_dir.display()
        );
        std::process::exit(1);
    }

    let now = Utc::now();
    let mut vec_segments: Vec<Vec<u8>> = Vec::new();
    let mut total_text_bytes: usize = 0;
    let mut file_count: usize = 0;
    let mut chunk_count: usize = 0;

    let code_fence_re = Regex::new(r"```").unwrap();

    for entry in WalkDir::new(&cli.docs_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .is_some_and(|ext| ext == "mdx")
                && e.file_type().is_file()
        })
    {
        let path = entry.path();
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Warning: could not read {}: {e}", path.display());
                continue;
            }
        };

        file_count += 1;

        let (fm, body) = parse_frontmatter(&content);
        let cleaned = strip_mdx(body);
        let raw_chunks = chunk_by_headings(&cleaned);
        let chunks = split_large_chunks(raw_chunks);

        let slug = page_slug(path, &cli.docs_dir);
        let src = source_path(path, &cli.docs_dir);
        let doc_url = format!("{BASE_URL}{src}");
        let tags = tags_from_path(path, &cli.docs_dir);
        let category = category_from_path(path, &cli.docs_dir);
        let total_chunks = chunks.len();

        for (i, chunk) in chunks.iter().enumerate() {
            let id = format!("{slug}-{i}");
            let has_code = code_fence_re.find_iter(&chunk.text).count() >= 2;
            let section = if chunk.heading.is_empty() {
                fm.title.clone()
            } else {
                chunk.heading.clone()
            };

            let metadata = serde_json::json!({
                "source": src,
                "title": fm.title,
                "section": section,
                "doc_url": doc_url,
                "category": category,
                "chunk_index": i,
                "total_chunks": total_chunks,
                "has_code": has_code,
            });

            let embedding = hash_embed(&chunk.text);
            total_text_bytes += chunk.text.len();

            let payload = VecPayload {
                id,
                text: chunk.text.clone(),
                embedding,
                metadata,
                tags: tags.clone(),
                namespace: "docs".to_string(),
                dimension: EMBEDDING_DIM as u32,
                embedder_name: EMBEDDER_NAME.to_string(),
            };

            let cbor_buf = cbor_encode(&payload);

            // segment_id starts at 1 (0 is reserved for manifest)
            chunk_count += 1;
            let seg_bytes = weftos_rvf_wire::write_segment(
                SegmentType::Vec as u8,
                &cbor_buf,
                SegmentFlags::empty(),
                chunk_count as u64,
            );
            vec_segments.push(seg_bytes);
        }
    }

    // Build manifest segment (segment_id = 0, SegmentType::Meta = 0x07).
    let manifest = ManifestPayload {
        agent_id: "tour-guide".to_string(),
        namespace: "docs".to_string(),
        segment_count: chunk_count,
        dimension: EMBEDDING_DIM as u32,
        embedder_name: EMBEDDER_NAME.to_string(),
        created_at: now.to_rfc3339(),
        version: 1,
    };
    let manifest_cbor = cbor_encode(&manifest);
    let manifest_seg = weftos_rvf_wire::write_segment(
        SegmentType::Meta as u8,
        &manifest_cbor,
        SegmentFlags::empty(),
        0, // segment_id 0 for manifest
    );

    // Assemble final output: manifest + all vec segments.
    let mut output = Vec::with_capacity(
        manifest_seg.len() + vec_segments.iter().map(|s| s.len()).sum::<usize>(),
    );
    output.extend_from_slice(&manifest_seg);
    for seg in &vec_segments {
        output.extend_from_slice(seg);
    }

    // Ensure output directory exists.
    if let Some(parent) = cli.output.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).unwrap_or_else(|e| {
                eprintln!("Error: could not create output directory: {e}");
                std::process::exit(1);
            });
        }
    }

    std::fs::write(&cli.output, &output).unwrap_or_else(|e| {
        eprintln!("Error: could not write output file: {e}");
        std::process::exit(1);
    });

    let output_size = output.len();
    println!("Knowledge base built successfully (binary RVF).");
    println!("  Files processed:  {file_count}");
    println!("  Segments created: {chunk_count}");
    println!("  Total text:       {:.1} KB", total_text_bytes as f64 / 1024.0);
    println!(
        "  Output file:      {} ({:.1} KB)",
        cli.output.display(),
        output_size as f64 / 1024.0
    );
}
