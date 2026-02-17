//! Tool implementations for clawft.
//!
//! Provides file, shell, and memory tools that implement the
//! `Tool` trait from clawft-core.
//!
//! # Tools
//!
//! - **File tools** ([`file_tools`]): `read_file`, `write_file`, `edit_file`, `list_directory`
//! - **Shell tool** ([`shell_tool`]): `exec_shell`
//! - **Memory tools** ([`memory_tool`]): `memory_read`, `memory_write`
//!
//! All file and directory operations enforce workspace path containment
//! to prevent directory traversal attacks.

pub mod file_tools;
pub mod memory_tool;
pub mod message_tool;
pub mod shell_tool;
pub mod spawn_tool;
pub mod web_fetch;
pub mod web_search;

use std::path::PathBuf;
use std::sync::Arc;

use clawft_core::tools::registry::ToolRegistry;
use clawft_platform::Platform;

/// Register all built-in tools with the given registry.
///
/// Creates instances of every tool in this crate and registers them
/// with `registry`. File tools are sandboxed to `workspace_dir`.
///
/// # Arguments
///
/// * `registry` - The tool registry to populate.
/// * `platform` - Platform abstraction for filesystem and process operations.
/// * `workspace_dir` - Root directory that file tools are restricted to.
pub fn register_all<P: Platform + 'static>(
    registry: &mut ToolRegistry,
    platform: Arc<P>,
    workspace_dir: PathBuf,
) {
    registry.register(Arc::new(file_tools::ReadFileTool::new(
        platform.clone(),
        workspace_dir.clone(),
    )));
    registry.register(Arc::new(file_tools::WriteFileTool::new(
        platform.clone(),
        workspace_dir.clone(),
    )));
    registry.register(Arc::new(file_tools::EditFileTool::new(
        platform.clone(),
        workspace_dir.clone(),
    )));
    registry.register(Arc::new(file_tools::ListDirectoryTool::new(
        platform.clone(),
        workspace_dir.clone(),
    )));
    registry.register(Arc::new(shell_tool::ShellExecTool::new(
        workspace_dir.clone(),
    )));
    registry.register(Arc::new(memory_tool::MemoryReadTool::new(
        platform.clone(),
    )));
    registry.register(Arc::new(memory_tool::MemoryWriteTool::new(
        platform.clone(),
    )));
    registry.register(Arc::new(web_search::WebSearchTool::new(
        platform.clone(),
        None,
    )));
    registry.register(Arc::new(web_fetch::WebFetchTool::new(
        platform.clone(),
    )));
    registry.register(Arc::new(spawn_tool::SpawnTool::new(
        platform,
        workspace_dir,
    )));
}
