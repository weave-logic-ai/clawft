//! Export formats for the knowledge graph.

pub mod json;
pub mod obsidian;
pub mod svg;
pub mod vowl;
pub mod wiki;

use std::path::Path;

use crate::GraphifyError;
use crate::model::KnowledgeGraph;

/// Supported export formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// JSON (`node_link_data` compatible).
    Json,
    /// GraphML XML format.
    GraphMl,
    /// Neo4j Cypher text.
    Cypher,
    /// Interactive HTML visualization (requires `html-export` feature).
    Html,
    /// Obsidian vault + canvas.
    Obsidian,
    /// Positioned SVG (Barnes–Hut / tree layout → static SVG).
    Svg,
    /// Wikipedia-style markdown wiki.
    Wiki,
    /// VOWL JSON for WebVOWL / topology navigator.
    Vowl,
}

impl ExportFormat {
    /// Parse a format string into an `ExportFormat`.
    pub fn from_str_loose(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "json" => Some(Self::Json),
            "graphml" => Some(Self::GraphMl),
            "cypher" => Some(Self::Cypher),
            "html" => Some(Self::Html),
            "obsidian" => Some(Self::Obsidian),
            "svg" => Some(Self::Svg),
            "wiki" => Some(Self::Wiki),
            "vowl" => Some(Self::Vowl),
            _ => None,
        }
    }

    /// File extension for this format.
    pub fn extension(&self) -> &str {
        match self {
            Self::Json => "json",
            Self::GraphMl => "graphml",
            Self::Cypher => "cypher",
            Self::Html => "html",
            Self::Obsidian => "md",
            Self::Svg => "svg",
            Self::Wiki => "md",
            Self::Vowl => "json",
        }
    }
}

/// Export a knowledge graph to the given format and output path.
pub fn export(
    kg: &KnowledgeGraph,
    format: ExportFormat,
    output: &Path,
) -> Result<(), GraphifyError> {
    match format {
        ExportFormat::Json => json::to_json(kg, output),
        ExportFormat::Obsidian => {
            obsidian::to_obsidian_vault(kg, output)?;
            Ok(())
        }
        ExportFormat::Wiki => {
            wiki::to_wiki(kg, output, &[], None)?;
            Ok(())
        }
        ExportFormat::Svg => svg::to_svg(kg, output),
        _ => Err(GraphifyError::ExportError(format!(
            "Export format {:?} not yet implemented",
            format,
        ))),
    }
}
