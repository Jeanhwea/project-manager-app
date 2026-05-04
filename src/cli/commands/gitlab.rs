//! GitLab command definitions

use clap::{Subcommand, ValueEnum};

/// GitLab integration commands
#[derive(Subcommand)]
#[command(visible_alias = "gl")]
#[command(about = "GitLab integration commands")]
pub enum GitlabCommands {
    /// Login to a GitLab server and save credentials
    #[command(about = "Login to a GitLab server and save credentials")]
    Login {
        /// GitLab server URL (will prompt if not provided)
        #[arg(
            long,
            short,
            help = "GitLab server URL (e.g. https://gitlab.com, http://192.168.0.110/gitlab/)"
        )]
        server: Option<String>,

        /// GitLab Personal Access Token (required, will prompt if not provided)
        #[arg(long, short = 't', help = "GitLab Personal Access Token (required)")]
        token: Option<String>,

        /// Default clone protocol
        #[arg(
            long,
            short = 'p',
            value_enum,
            default_value = "ssh",
            help = "Default clone protocol: ssh or https"
        )]
        protocol: CloneProtocol,
    },

    /// Clone all repositories from a GitLab group
    #[command(visible_alias = "cl")]
    #[command(about = "Clone all repositories from a GitLab group")]
    Clone {
        /// GitLab group path (e.g. "my-org/team" or numeric ID)
        #[arg(help = "GitLab group path (e.g. \"my-org/team\" or numeric ID)")]
        group: String,

        /// GitLab server URL (uses saved config if not specified)
        #[arg(
            long,
            short,
            help = "GitLab server URL (uses saved config if not specified)"
        )]
        server: Option<String>,

        /// GitLab private token (overrides saved config)
        #[arg(
            long,
            short = 't',
            help = "GitLab private token (overrides saved config)"
        )]
        token: Option<String>,

        /// Clone protocol (overrides saved config)
        #[arg(
            long,
            short = 'p',
            value_enum,
            help = "Clone protocol: ssh or https (uses saved config if not specified)"
        )]
        protocol: Option<CloneProtocol>,

        /// Output directory for cloned repositories
        #[arg(
            long,
            short = 'o',
            default_value = ".",
            help = "Output directory for cloned repositories"
        )]
        output: String,

        /// Include archived projects
        #[arg(long, default_value = "false", help = "Include archived projects")]
        include_archived: bool,

        /// Clone submodules recursively
        #[arg(long, default_value = "false", help = "Clone submodules recursively")]
        recursive: bool,

        /// Dry run: show what would be changed without making any modifications
        #[arg(
            long,
            default_value = "false",
            help = "Dry run: show what would be changed without making any modifications"
        )]
        dry_run: bool,
    },
}

/// Clone protocol enumeration
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CloneProtocol {
    Ssh,
    Https,
}
