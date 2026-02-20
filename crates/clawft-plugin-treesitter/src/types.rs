//! Types for tree-sitter code analysis operations.

use serde::{Deserialize, Serialize};

/// Supported programming languages for analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Rust,
    TypeScript,
    Python,
    JavaScript,
}

impl Language {
    /// Parse a language string into a known language enum.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "rust" | "rs" => Some(Self::Rust),
            "typescript" | "ts" => Some(Self::TypeScript),
            "python" | "py" => Some(Self::Python),
            "javascript" | "js" => Some(Self::JavaScript),
            _ => None,
        }
    }

    /// File extensions associated with this language.
    pub fn extensions(&self) -> &[&str] {
        match self {
            Self::Rust => &["rs"],
            Self::TypeScript => &["ts", "tsx"],
            Self::Python => &["py"],
            Self::JavaScript => &["js", "jsx", "mjs"],
        }
    }
}

/// Configuration for tree-sitter analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeSitterConfig {
    /// Maximum file size in bytes to analyze (default: 1 MB).
    #[serde(default = "default_max_file_size")]
    pub max_file_size: usize,

    /// Maximum depth for AST output to prevent overly large responses.
    #[serde(default = "default_max_depth")]
    pub max_ast_depth: usize,
}

fn default_max_file_size() -> usize {
    1_048_576
}

fn default_max_depth() -> usize {
    10
}

impl Default for TreeSitterConfig {
    fn default() -> Self {
        Self {
            max_file_size: default_max_file_size(),
            max_ast_depth: default_max_depth(),
        }
    }
}

/// A symbol found in the source code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    /// Symbol name (e.g., function name, class name).
    pub name: String,
    /// Symbol kind (e.g., "function", "struct", "class", "method").
    pub kind: String,
    /// Start line (0-based).
    pub start_line: usize,
    /// End line (0-based).
    pub end_line: usize,
    /// Start column (0-based).
    pub start_col: usize,
}

/// Complexity metrics for a source file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityMetrics {
    /// Total cyclomatic complexity of the file.
    pub total_complexity: usize,
    /// Number of functions found.
    pub function_count: usize,
    /// Per-function complexity breakdown.
    pub functions: Vec<FunctionComplexity>,
}

/// Complexity for a single function.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionComplexity {
    /// Function name.
    pub name: String,
    /// Cyclomatic complexity.
    pub complexity: usize,
    /// Start line (0-based).
    pub start_line: usize,
}

/// A simplified AST node for serialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstNode {
    /// Node kind (e.g., "function_item", "if_expression").
    pub kind: String,
    /// Start line (0-based).
    pub start_line: usize,
    /// End line (0-based).
    pub end_line: usize,
    /// Whether this node is named (not anonymous syntax).
    pub is_named: bool,
    /// Children nodes (truncated at max_depth).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<AstNode>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_parse() {
        assert_eq!(Language::parse("rust"), Some(Language::Rust));
        assert_eq!(Language::parse("rs"), Some(Language::Rust));
        assert_eq!(Language::parse("TypeScript"), Some(Language::TypeScript));
        assert_eq!(Language::parse("ts"), Some(Language::TypeScript));
        assert_eq!(Language::parse("python"), Some(Language::Python));
        assert_eq!(Language::parse("py"), Some(Language::Python));
        assert_eq!(Language::parse("javascript"), Some(Language::JavaScript));
        assert_eq!(Language::parse("js"), Some(Language::JavaScript));
        assert_eq!(Language::parse("unknown"), None);
    }

    #[test]
    fn language_extensions() {
        assert!(Language::Rust.extensions().contains(&"rs"));
        assert!(Language::TypeScript.extensions().contains(&"ts"));
        assert!(Language::Python.extensions().contains(&"py"));
        assert!(Language::JavaScript.extensions().contains(&"js"));
    }

    #[test]
    fn language_serde() {
        let json = serde_json::to_string(&Language::Rust).unwrap();
        assert_eq!(json, r#""rust""#);
        let restored: Language = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, Language::Rust);
    }

    #[test]
    fn config_default() {
        let config = TreeSitterConfig::default();
        assert_eq!(config.max_file_size, 1_048_576);
        assert_eq!(config.max_ast_depth, 10);
    }
}
