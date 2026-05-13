use crate::control::command::Command;
use crate::domain::editor::{
    BumpType, EditorRegistry, add_lockfile_operations, compute_edited_content,
    resolve_config_files,
};
use crate::domain::git::{
    ReleaseGitState, collect_context, resolve_git_root, validate_git_state,
};
use crate::error::Result;
use crate::model::git::GitContext;
use crate::model::plan::{EditOperation, ExecutionPlan, GitOperation, MessageOperation};
use crate::utils::path::canonicalize_path;
use std::path::Path;
use std::path::PathBuf;

#[derive(Debug, clap::Args)]
pub struct ReleaseArgs {
    #[arg(
        value_enum,
        default_value = "patch",
        help = "Bump type: major, minor, patch"
    )]
    pub bump_type: BumpType,
    #[arg(help = "Files to update version (auto-detect if not specified)")]
    pub files: Vec<String>,
    #[arg(
        long,
        short = 'n',
        default_value = "false",
        help = "Stay in current directory"
    )]
    pub no_root: bool,
    #[arg(
        long,
        short,
        default_value = "false",
        help = "Force release even if not on master"
    )]
    pub force: bool,
    #[arg(long, default_value = "false", help = "Skip pushing tags and branches")]
    pub skip_push: bool,
    #[arg(long, default_value = "false", help = "Dry run")]
    pub dry_run: bool,
    #[arg(long, short = 'm', help = "Custom commit message")]
    pub message: Option<String>,
    #[arg(long, help = "Pre-release suffix (e.g. \"alpha\" -> v1.0.0-alpha)")]
    pub pre_release: Option<String>,
}

#[derive(Debug)]
pub(crate) struct ReleaseContext {
    git_ctx: GitContext,
    state: ReleaseGitState,
    config_files: Vec<String>,
    registry: EditorRegistry,
}

impl Command for ReleaseArgs {
    type Context = ReleaseContext;

    fn context(&self) -> Result<ReleaseContext> {
        let work_dir = if self.no_root {
            std::env::current_dir()?
        } else {
            resolve_git_root()?
        };

        let resolved_files = resolve_file_paths(&self.files, &work_dir);

        let git_ctx = collect_context(&work_dir)?;
        let state = validate_git_state(
            &work_dir,
            self.force,
            &self.bump_type,
            &self.pre_release,
            &self.message,
            &git_ctx,
        )?;
        let registry = EditorRegistry::default_with_editors();
        let config_files = resolve_config_files(&registry, &resolved_files)?;

        Ok(ReleaseContext {
            git_ctx,
            state,
            config_files,
            registry,
        })
    }

    fn plan(&self, ctx: &ReleaseContext) -> Result<ExecutionPlan> {
        let mut plan = build_execution_plan(
            self,
            &ctx.config_files,
            &ctx.state,
            &ctx.git_ctx,
            &ctx.registry,
        );
        plan.dry_run = self.dry_run;
        Ok(plan)
    }
}

pub fn run(args: ReleaseArgs) -> Result<()> {
    Command::run(&args)
}

fn resolve_file_paths(files: &[String], base_dir: &Path) -> Vec<String> {
    files
        .iter()
        .map(|f| {
            let path = Path::new(f);
            if path.is_absolute() {
                f.clone().replace('\\', "/")
            } else if path.starts_with(".") || f.contains('/') || f.contains('\\') {
                canonicalize_path(base_dir.join(f))
                    .map(|p| p.to_string_lossy().replace('\\', "/"))
                    .unwrap_or_else(|_| f.clone().replace('\\', "/"))
            } else {
                canonicalize_path(f)
                    .map(|p| p.to_string_lossy().replace('\\', "/"))
                    .unwrap_or_else(|_| f.clone().replace('\\', "/"))
            }
        })
        .collect()
}

fn build_execution_plan(
    args: &ReleaseArgs,
    config_files: &[String],
    state: &ReleaseGitState,
    ctx: &GitContext,
    registry: &EditorRegistry,
) -> ExecutionPlan {
    let mut plan = ExecutionPlan::new();

    plan.add(MessageOperation::Section {
        title: "修改计划".to_string(),
    });

    for file_path in config_files {
        let editor = registry.detect_editor(Path::new(file_path)).unwrap();
        if let Ok((original, edited)) = compute_edited_content(editor, &state.new_tag, file_path)
        {
            let old_lines: Vec<&str> = original.lines().collect();
            let new_lines: Vec<&str> = edited.lines().collect();
            plan.add(EditOperation::WriteFile {
                path: file_path.clone(),
                content: edited,
                description: format!("edit {}", file_path),
            });

            for (line_num, (old_line, new_line)) in
                (1..).zip(old_lines.iter().zip(new_lines.iter()))
            {
                if old_line != new_line {
                    plan.add(MessageOperation::Diff {
                        file: file_path.clone(),
                        line_num,
                        old_content: old_line.to_string(),
                        new_content: new_line.to_string(),
                    });
                }
            }

            add_lockfile_operations(&mut plan, file_path);

            plan.add(GitOperation::Add {
                path: file_path.clone(),
                working_dir: PathBuf::from("."),
            });
        }
    }

    plan.add(GitOperation::Commit {
        message: state.commit_message.clone(),
        working_dir: PathBuf::from("."),
    });

    // Handle the case where tag already exists
    plan.add(GitOperation::CreateTag {
        tag: state.new_tag.clone(),
        working_dir: PathBuf::from("."),
    });

    if !args.skip_push {
        for remote in ctx.remote_names() {
            plan.add(GitOperation::PushTag {
                remote: remote.to_string(),
                tag: state.new_tag.clone(),
                working_dir: PathBuf::from("."),
            });
            plan.add(GitOperation::PushBranch {
                remote: remote.to_string(),
                branch: state.current_branch.clone(),
                working_dir: PathBuf::from("."),
            });
        }
    }

    plan
}
