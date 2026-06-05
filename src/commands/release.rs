use crate::commands::Command;
use crate::domain::editor::{
    BumpType, EditorRegistry, add_lockfile_operations, compute_edited_content,
    detect_config_files, extract_fallback_version, read_file_version, resolve_config_files,
};
use crate::domain::git::GitOperation;
use crate::domain::git::{
    ReleaseError, ReleaseGitState, collect_context, resolve_git_root, validate_git_state,
};
use crate::domain::project_config;
use crate::engine::plan;
use crate::error::{AppError, Result};
use crate::model::git::GitContext;
use crate::model::operation::EditOperation;
use crate::model::plan::{DisplayMessage, ExecutionPlan, ExecutionResult, Phase, Step};
use crate::model::project_config::ProjectConfig;
use crate::utils::output;
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
    #[arg(
        long,
        default_value = "false",
        help = "Initialize .pma.json with auto-detected files and exit"
    )]
    pub init: bool,
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
    type Plan = ExecutionPlan;

    fn collect(&self) -> Result<ReleaseContext> {
        let work_dir = if self.no_root {
            std::env::current_dir()?
        } else {
            resolve_git_root()?
        };

        let cli_files = if self.files.is_empty() {
            project_config::load(&work_dir)
                .map(|c| c.files)
                .unwrap_or_default()
        } else {
            self.files.clone()
        };

        let resolved_files = resolve_file_paths(&cli_files, &work_dir);

        let git_ctx = collect_context(&work_dir)?;
        let registry = EditorRegistry::default_with_editors();
        let config_files = resolve_config_files(&registry, &resolved_files)?;

        let fallback_version = extract_fallback_version(&registry, &config_files);

        let state = validate_git_state(
            &work_dir,
            self.force,
            &self.bump_type,
            &self.pre_release,
            &self.message,
            &git_ctx,
            fallback_version.as_deref(),
        )?;

        Ok(ReleaseContext {
            git_ctx,
            state,
            config_files,
            registry,
        })
    }

    fn plan(&self, ctx: &ReleaseContext) -> Result<ExecutionPlan> {
        let plan = build_execution_plan(
            self,
            &ctx.config_files,
            &ctx.state,
            &ctx.git_ctx,
            &ctx.registry,
        )?;
        Ok(plan.with_dry_run(self.dry_run))
    }

    fn execute(&self, plan: &ExecutionPlan) -> Result<ExecutionResult> {
        plan::run_plan(plan)
    }
}

pub fn run(args: ReleaseArgs) -> Result<()> {
    if args.init {
        return init_project_config(&args);
    }
    Command::run(&args)
}

fn init_project_config(args: &ReleaseArgs) -> Result<()> {
    let work_dir = if args.no_root {
        std::env::current_dir()?
    } else {
        resolve_git_root()?
    };

    let target = project_config::config_path(&work_dir);
    if target.exists() {
        return Err(AppError::AlreadyExists {
            resource: "项目配置".into(),
            name: target.display().to_string(),
        });
    }

    let registry = EditorRegistry::default_with_editors();
    let detected = detect_config_files(&registry).unwrap_or_default();

    let content = ProjectConfig::render(&detected);
    std::fs::write(&target, content)?;

    output::item("已创建", &target.display().to_string());
    if detected.is_empty() {
        output::warning("未自动探测到任何版本文件，请手动编辑 files 字段");
    } else {
        for f in &detected {
            output::item("文件", f);
        }
    }
    Ok(())
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
) -> Result<ExecutionPlan> {
    let mut plan = ExecutionPlan::new();
    add_release_metadata(&mut plan, args, state);

    let edit_phase = build_edit_phase(config_files, state, registry)?;
    let has_changes = edit_phase
        .steps()
        .iter()
        .any(|step| matches!(step, Step::Op(_)));

    if !edit_phase.is_empty() {
        plan.add_phase(edit_phase);
    }

    let git_phase = build_git_phase(args, state, ctx, has_changes);
    if !git_phase.is_empty() {
        plan.add_phase(git_phase);
    }

    Ok(plan)
}

fn add_release_metadata(plan: &mut ExecutionPlan, args: &ReleaseArgs, state: &ReleaseGitState) {
    plan.add_message(DisplayMessage::Item {
        label: "当前版本".to_string(),
        value: state.current_tag.clone(),
    });
    plan.add_message(DisplayMessage::Item {
        label: "目标版本".to_string(),
        value: state.new_tag.clone(),
    });
    if state.used_fallback {
        plan.add_message(DisplayMessage::Warning {
            msg: format!("无 git tag，使用文件版本 {} 作为基准", state.current_tag),
        });
    }
    if args.message.is_some() {
        plan.add_message(DisplayMessage::Detail {
            label: "提交消息".to_string(),
            value: state.commit_message.clone(),
        });
    }
}

fn build_edit_phase(
    config_files: &[String],
    state: &ReleaseGitState,
    registry: &EditorRegistry,
) -> Result<Phase> {
    let mut edit_phase = Phase::new("版本修改");

    for file_path in config_files {
        let editor = registry
            .detect_editor(Path::new(file_path))
            .ok_or_else(|| ReleaseError::UnknownFileType {
                path: file_path.clone(),
            })?;

        let (original, edited) = compute_edited_content(editor, &state.new_tag, file_path)?;

        if let Ok(file_ver) = read_file_version(editor, file_path) {
            let git_ver = state.current_tag.trim_start_matches('v');
            if file_ver != git_ver {
                edit_phase.add_message(DisplayMessage::Warning {
                    msg: format!(
                        "文件版本 {} 与 git tag {} 不一致，以 git tag 为准",
                        file_ver, git_ver
                    ),
                });
            }
        }

        if original == edited {
            continue;
        }

        edit_phase.add(EditOperation::WriteFile {
            path: file_path.clone(),
            content: edited.clone(),
            description: format!("edit {}", file_path),
        });

        if let Some(diff) = compute_line_diff(file_path, &original, &edited) {
            edit_phase.add_message(diff);
        }

        add_lockfile_operations(&mut edit_phase, file_path);

        edit_phase.add(GitOperation::Add {
            path: file_path.clone(),
            working_dir: PathBuf::from("."),
        });
    }

    Ok(edit_phase)
}

fn compute_line_diff(file_path: &str, original: &str, edited: &str) -> Option<DisplayMessage> {
    let old_lines: Vec<&str> = original.lines().collect();
    let new_lines: Vec<&str> = edited.lines().collect();

    let changed_lines: Vec<(String, String)> = old_lines
        .iter()
        .zip(new_lines.iter())
        .filter(|(old, new)| old != new)
        .map(|(old, new)| (old.to_string(), new.to_string()))
        .collect();

    if changed_lines.is_empty() {
        return None;
    }

    let count = changed_lines.len();
    Some(DisplayMessage::Diff {
        file: file_path.to_string(),
        old_start: 1,
        new_start: 1,
        old_lines: changed_lines.iter().map(|(old, _)| old.clone()).collect(),
        new_lines: changed_lines.iter().map(|(_, new)| new.clone()).collect(),
        old_count: count,
        new_count: count,
    })
}

fn build_git_phase(
    args: &ReleaseArgs,
    state: &ReleaseGitState,
    ctx: &GitContext,
    has_changes: bool,
) -> Phase {
    let mut git_phase = Phase::new("Git 提交推送");

    if has_changes {
        git_phase.add(GitOperation::Commit {
            message: state.commit_message.clone(),
            working_dir: PathBuf::from("."),
        });
    }

    git_phase.add(GitOperation::CreateTag {
        tag: state.new_tag.clone(),
        working_dir: PathBuf::from("."),
    });

    if !args.skip_push {
        for remote in ctx.remote_names() {
            git_phase.add(GitOperation::PushTag {
                remote: remote.to_string(),
                tag: state.new_tag.clone(),
                working_dir: PathBuf::from("."),
            });
            git_phase.add(GitOperation::PushBranch {
                remote: remote.to_string(),
                branch: state.current_branch.clone(),
                working_dir: PathBuf::from("."),
            });
        }
    }

    git_phase
}
