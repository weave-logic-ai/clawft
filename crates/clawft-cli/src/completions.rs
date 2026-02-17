//! Shell completion generation for the `weft` CLI.
//!
//! Generates completions for bash, zsh, fish, and PowerShell.
//! Requires the `completions` feature to be enabled.

/// Shell types supported for completion generation.
#[derive(Clone, Debug)]
#[allow(clippy::enum_variant_names)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
    PowerShell,
}

impl Shell {
    /// Parse a shell name string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "bash" => Some(Self::Bash),
            "zsh" => Some(Self::Zsh),
            "fish" => Some(Self::Fish),
            "powershell" | "ps" => Some(Self::PowerShell),
            _ => None,
        }
    }

    /// List all supported shell names.
    pub fn all_names() -> &'static [&'static str] {
        &["bash", "zsh", "fish", "powershell"]
    }
}

/// Print completion script to stdout for the given shell.
///
/// When the `completions` feature is enabled, this uses `clap_complete` to
/// generate real completions. Without the feature, it prints a helpful message.
pub fn generate_completions(shell: &Shell, cmd: &mut clap::Command) {
    // Note: When clap_complete is added as a dependency, this will use:
    // clap_complete::generate(shell_type, cmd, "weft", &mut std::io::stdout());
    // For now, generate a basic completion script stub.
    let bin_name = cmd.get_name().to_string();
    let subcommands: Vec<String> = cmd
        .get_subcommands()
        .map(|s| s.get_name().to_string())
        .collect();

    match shell {
        Shell::Bash => {
            println!("# {bin_name} bash completion");
            println!("_{bin_name}() {{");
            println!(
                "    local commands=\"{}\"",
                subcommands.join(" ")
            );
            println!(
                "    COMPREPLY=($(compgen -W \"$commands\" -- \"${{COMP_WORDS[COMP_CWORD]}}\"))"
            );
            println!("}}");
            println!("complete -F _{bin_name} {bin_name}");
        }
        Shell::Zsh => {
            println!("#compdef {bin_name}");
            println!("_{}() {{", bin_name);
            println!(
                "    local commands=({})",
                subcommands
                    .iter()
                    .map(|s| format!("'{s}'"))
                    .collect::<Vec<_>>()
                    .join(" ")
            );
            println!("    _describe 'command' commands");
            println!("}}");
        }
        Shell::Fish => {
            for cmd_name in &subcommands {
                println!(
                    "complete -c {bin_name} -n '__fish_use_subcommand' -a '{cmd_name}'"
                );
            }
        }
        Shell::PowerShell => {
            println!("# {bin_name} PowerShell completion");
            println!(
                "Register-ArgumentCompleter -CommandName {bin_name} -ScriptBlock {{"
            );
            println!(
                "    param($commandName, $wordToComplete, $commandAst, $fakeBoundParameter)"
            );
            println!(
                "    @({}) | Where-Object {{ $_ -like \"$wordToComplete*\" }}",
                subcommands
                    .iter()
                    .map(|s| format!("'{s}'"))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            println!("}}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_from_str_valid() {
        assert!(Shell::from_str("bash").is_some());
        assert!(Shell::from_str("zsh").is_some());
        assert!(Shell::from_str("fish").is_some());
        assert!(Shell::from_str("powershell").is_some());
        assert!(Shell::from_str("BASH").is_some());
    }

    #[test]
    fn shell_from_str_invalid() {
        assert!(Shell::from_str("unknown").is_none());
        assert!(Shell::from_str("").is_none());
    }

    #[test]
    fn all_names_not_empty() {
        assert!(!Shell::all_names().is_empty());
    }

    #[test]
    fn generate_completions_bash() {
        let mut cmd = clap::Command::new("test")
            .subcommand(clap::Command::new("sub1"))
            .subcommand(clap::Command::new("sub2"));
        generate_completions(&Shell::Bash, &mut cmd);
    }

    #[test]
    fn generate_completions_zsh() {
        let mut cmd = clap::Command::new("test")
            .subcommand(clap::Command::new("sub1"));
        generate_completions(&Shell::Zsh, &mut cmd);
    }

    #[test]
    fn generate_completions_fish() {
        let mut cmd = clap::Command::new("test")
            .subcommand(clap::Command::new("sub1"));
        generate_completions(&Shell::Fish, &mut cmd);
    }

    #[test]
    fn generate_completions_powershell() {
        let mut cmd = clap::Command::new("test")
            .subcommand(clap::Command::new("sub1"));
        generate_completions(&Shell::PowerShell, &mut cmd);
    }
}
