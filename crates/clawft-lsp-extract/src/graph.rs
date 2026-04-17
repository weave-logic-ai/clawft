//! LSP extraction graph — the output of querying a language server.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A knowledge graph extracted from LSP server responses.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LspGraph {
    pub nodes: Vec<LspNode>,
    pub edges: Vec<LspEdge>,
    pub language: String,
    pub root_uri: String,
    pub stats: LspStats,
}

/// A node extracted from LSP symbols.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspNode {
    pub id: String,
    pub name: String,
    pub kind: LspNodeKind,
    pub file: String,
    pub line: u32,
    pub end_line: u32,
    pub detail: Option<String>,
    pub container: Option<String>,
    pub is_public: bool,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Edge between two LSP nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspEdge {
    pub source: String,
    pub target: String,
    pub kind: LspEdgeKind,
    pub file: Option<String>,
    pub line: Option<u32>,
}

/// Node kinds mapped from LSP SymbolKind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LspNodeKind {
    File,
    Module,
    Namespace,
    Package,
    Class,
    Method,
    Property,
    Field,
    Constructor,
    Enum,
    Interface,
    Function,
    Variable,
    Constant,
    String,
    Number,
    Boolean,
    Array,
    Object,
    Key,
    Null,
    EnumMember,
    Struct,
    Event,
    Operator,
    TypeParameter,
    Macro,
    Unknown,
}

impl LspNodeKind {
    /// Convert from LSP SymbolKind integer.
    pub fn from_lsp(kind: u32) -> Self {
        match kind {
            1 => Self::File,
            2 => Self::Module,
            3 => Self::Namespace,
            4 => Self::Package,
            5 => Self::Class,
            6 => Self::Method,
            7 => Self::Property,
            8 => Self::Field,
            9 => Self::Constructor,
            10 => Self::Enum,
            11 => Self::Interface,
            12 => Self::Function,
            13 => Self::Variable,
            14 => Self::Constant,
            15 => Self::String,
            16 => Self::Number,
            17 => Self::Boolean,
            18 => Self::Array,
            19 => Self::Object,
            20 => Self::Key,
            21 => Self::Null,
            22 => Self::EnumMember,
            23 => Self::Struct,
            24 => Self::Event,
            25 => Self::Operator,
            26 => Self::TypeParameter,
            _ => Self::Unknown,
        }
    }
}

/// Edge kinds derived from LSP queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LspEdgeKind {
    Contains,
    Calls,
    CalledBy,
    References,
    Implements,
    Extends,
    Overrides,
    TypeOf,
    ImportedBy,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LspStats {
    pub files_processed: usize,
    pub symbols_extracted: usize,
    pub references_resolved: usize,
    pub calls_mapped: usize,
    pub duration_ms: u64,
}

impl LspGraph {
    pub fn new(language: &str, root_uri: &str) -> Self {
        Self {
            language: language.into(),
            root_uri: root_uri.into(),
            ..Default::default()
        }
    }
}
