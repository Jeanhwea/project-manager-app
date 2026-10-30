//! 语义化版本发布：`pma release patch|minor|major`，另外用 `--init` 生成/查看 `.pma.json`。
//! 主线流程是「定位仓库根 → 由 tag（或文件版本）确定基准版本 → 算出新版本号 → 改写各版本文件
//! → git add/commit → 打 tag → 推送到所有 remote」。
//! 改动全部先攒成 ExecutionPlan，所以 `--dry-run` 能预览而不落盘，`--skip-push` 则停在本地。

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

/// `pma release` 的参数集合
#[derive(Debug, clap::Args)]
pub struct ReleaseArgs {
    #[arg(
        value_enum,
        default_value = "patch",
        help = "Bump type: major, minor, patch"
    )]
    pub bump_type: BumpType,
    /// 优先级：命令行 > `.pma.json` 的 files 字段 > 按编辑器候选路径自动探测
    #[arg(help = "Files to update version (auto-detect if not specified)")]
    pub files: Vec<String>,
    /// true 表示留在当前目录；默认会切到 git 根目录再定位版本文件
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
    /// 基准版本、目标版本、提交信息等发布前置计算结果
    state: ReleaseGitState,
    /// 本次要改写的版本文件（绝对路径、正斜杠分隔）
    config_files: Vec<String>,
    /// 改文件时需要按路径查编辑器，故把注册表一并带上给 plan 阶段用
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
        let config_files = match resolve_config_files(&registry, &resolved_files) {
            Ok(files) => files,
            Err(AppError::Release(ReleaseError::NoConfigFiles)) if resolved_files.is_empty() => {
                vec![init_version_file(&work_dir)?]
            }
            Err(e) => return Err(e),
        };

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
    // --init 只负责生成配置，短路返回，不进入 release 的 collect/plan/execute
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
    // 已存在时不覆盖：里面可能是用户手工挑选/调整过的 files 列表
    if target.exists() {
        return Err(AppError::AlreadyExists {
            resource: "项目配置".into(),
            name: target.display().to_string(),
        });
    }

    let registry = EditorRegistry::default_with_editors();
    // 自动探测走的是相对候选路径，因此结果取决于当前目录（通常就是仓库根）
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

fn init_version_file(work_dir: &Path) -> Result<String> {
    // 项目里一个版本文件都没有时兜底建一个；返回路径后它会作为唯一的 config_file，
    // 其内容 0.0.0 同时充当本次发布的基准版本，于是首次发布得到 v0.0.1
    let target = work_dir.join("version.txt");
    std::fs::write(&target, "0.0.0\n")?;
    let path_str = canonicalize_path(&target)
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| target.to_string_lossy().replace('\\', "/"));
    output::warning("未检测到版本文件，已初始化 version.txt");
    output::item("已创建", &path_str);
    Ok(path_str)
}

fn resolve_file_paths(files: &[String], base_dir: &Path) -> Vec<String> {
    // 统一成正斜杠再往后传：下游按文件名后缀选编辑器，Windows 的反斜杠会让判断失效
    files
        .iter()
        .map(|f| {
            let path = Path::new(f);
            if path.is_absolute() {
                f.clone().replace('\\', "/")
            } else if path.starts_with(".") || f.contains('/') || f.contains('\\') {
                // 带路径的相对路径按 base_dir（仓库根）解析
                canonicalize_path(base_dir.join(f))
                    .map(|p| p.to_string_lossy().replace('\\', "/"))
                    .unwrap_or_else(|_| f.clone().replace('\\', "/"))
            } else {
                // 裸文件名不走 base_dir：自动探测给出的候选本来就是相对当前目录的
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
    // 有没有文件真的被改写决定要不要 commit：Phase 里只剩说明消息说明版本号已是目标值
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
        // 首次发布或 tag 被删净时的常见情形，提醒用户基准值是猜出来的
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

        // 文件与 tag 不一致时仍以 tag 为准，否则同一 tag 会被反复改写成不同内容
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

        // 版本号已经是目标值时没有可写的内容，跳过本文件（也就不产生 Op，见 has_changes）
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

        // lockfile 交给各自的包管理器重算，避免提交与新版本号不一致的锁文件
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

    // 按下标逐行比对（不是真正的 diff 算法）：版本号改一行时足够直观，
    // 一旦发生增删行，后面的行会整体错位、表现为变更行偏多；仅供预览，不参与改写
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

    // 这里的操作都以当前进程 cwd（"."）执行：文件参数是绝对路径，commit/tag 本来就作用于整个仓库，
    // 因此 --no-root 时在子目录里执行也不会漏掉仓库根的改动
    if has_changes {
        git_phase.add(GitOperation::Commit {
            message: state.commit_message.clone(),
            working_dir: PathBuf::from("."),
        });
    }

    // 没有文件改动就只补 tag（例如版本号已是目标值、或只想给当前 HEAD 打标）
    git_phase.add(GitOperation::CreateTag {
        tag: state.new_tag.clone(),
        working_dir: PathBuf::from("."),
    });

    // 推送不分主次：每个 remote 都要拿到同一个 tag 与分支
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
