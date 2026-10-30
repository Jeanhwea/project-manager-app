//! `pma config`：查看与初始化 `~/.pma` 下的配置。init 写入默认的主配置与 GitLab 配置，
//! show 打印当前生效的配置值（文件缺失时展示的是默认值），
//! path 只输出配置目录本身，方便被脚本取值。写文件动作都在 execute 阶段落地。

use crate::commands::Command;
use crate::domain::config::ConfigManager;
use crate::domain::config::schema;
use crate::engine::plan;
use crate::error::{AppError, Result};
use crate::model::operation::EditOperation;
use crate::model::plan::{DisplayMessage, ExecutionPlan, ExecutionResult, Phase};
use std::path::PathBuf;

/// `pma config` 的子命令：初始化（init）、查看配置（show）、查看路径（path）
#[derive(Debug, clap::Subcommand)]
pub enum ConfigArgs {
    /// Initialize a default configuration file
    Init,
    /// Show current configuration
    Show,
    /// Show configuration file path
    Path,
}

/// init 的采集结果：待创建的配置目录与两个配置文件路径
#[derive(Debug)]
pub(crate) struct ConfigInitContext {
    base_dir: PathBuf,
    config_path: PathBuf,
    gitlab_path: PathBuf,
}

/// show 的采集结果：配置目录是否已存在，以及两份配置的当前内容
#[derive(Debug)]
pub(crate) struct ConfigShowContext {
    base_dir: PathBuf,
    dir_exists: bool,
    config: schema::AppConfig,
    gitlab_config: schema::GitlabConfig,
}

/// path 的采集结果：配置根目录
#[derive(Debug)]
pub(crate) struct ConfigPathContext {
    base_dir: PathBuf,
}

// 三个子命令都不带参数，用零大小类型承接各自的 Command 实现
struct InitArgs;
struct ShowArgs;
struct PathArgs;

impl Command for InitArgs {
    type Context = ConfigInitContext;
    type Plan = ExecutionPlan;

    fn collect(&self) -> Result<ConfigInitContext> {
        let base_dir = ConfigManager::base_dir();
        // 目录已存在就直接放弃，避免覆盖用户已有的 config.toml / gitlab.toml（含 token）
        if base_dir.exists() {
            return Err(AppError::AlreadyExists {
                resource: "配置目录".into(),
                name: base_dir.display().to_string(),
            });
        }

        Ok(ConfigInitContext {
            base_dir: base_dir.clone(),
            config_path: ConfigManager::config_path(),
            gitlab_path: ConfigManager::gitlab_path(),
        })
    }

    fn plan(&self, ctx: &ConfigInitContext) -> Result<ExecutionPlan> {
        let mut plan = ExecutionPlan::new();

        let mut init_phase = Phase::new("创建配置");
        // 主配置与 GitLab 配置分开两个文件写：后者含凭据，方便单独收紧权限
        init_phase.add(EditOperation::WriteFile {
            path: ctx.config_path.to_string_lossy().to_string(),
            content: schema::default_config_content().to_string(),
            description: "create main config".to_string(),
        });

        init_phase.add(EditOperation::WriteFile {
            path: ctx.gitlab_path.to_string_lossy().to_string(),
            content: schema::default_gitlab_config_content().to_string(),
            description: "create gitlab config".to_string(),
        });
        plan.add_phase(init_phase);

        plan.add_message(DisplayMessage::Item {
            label: "已创建配置目录".to_string(),
            value: ctx.base_dir.display().to_string(),
        });
        plan.add_message(DisplayMessage::Detail {
            label: "主配置".to_string(),
            value: ctx.config_path.display().to_string(),
        });
        plan.add_message(DisplayMessage::Detail {
            label: "GitLab".to_string(),
            value: ctx.gitlab_path.display().to_string(),
        });

        Ok(plan)
    }

    fn execute(&self, plan: &ExecutionPlan) -> Result<ExecutionResult> {
        plan::run_plan(plan)
    }
}

impl Command for ShowArgs {
    type Context = ConfigShowContext;
    type Plan = ExecutionPlan;

    fn collect(&self) -> Result<ConfigShowContext> {
        let base_dir = ConfigManager::base_dir();
        let dir_exists = base_dir.exists();
        // load_* 在缺失或解析失败时返回默认值，所以 show 永远能打印出一份配置
        let config = ConfigManager::load_config();
        let gitlab_config = ConfigManager::load_gitlab();

        Ok(ConfigShowContext {
            base_dir,
            dir_exists,
            config,
            gitlab_config,
        })
    }

    fn plan(&self, ctx: &ConfigShowContext) -> Result<ExecutionPlan> {
        let mut plan = ExecutionPlan::new();

        // 目录不存在意味着上面拿到的是默认值而非用户配置，必须标注出来以免误导
        let dir_status = if ctx.dir_exists {
            String::new()
        } else {
            " (未创建, 使用默认值)".to_string()
        };
        plan.add_message(DisplayMessage::Item {
            label: "配置目录".to_string(),
            value: format!("{}{}", ctx.base_dir.display(), dir_status),
        });

        plan.add_message(DisplayMessage::Section {
            title: "[repository]".to_string(),
        });
        plan.add_message(DisplayMessage::Skip {
            msg: format!("max_depth  = {}", ctx.config.repository.max_depth),
        });
        plan.add_message(DisplayMessage::Skip {
            msg: format!("skip_dirs  = {:?}", ctx.config.repository.skip_dirs),
        });

        plan.add_message(DisplayMessage::Section {
            title: "[remote]".to_string(),
        });
        for rule in &ctx.config.remote.rules {
            plan.add_message(DisplayMessage::Item {
                label: rule.name.clone(),
                value: format!("{:?}", rule.hosts),
            });
            if let Some(ref url_prefix) = rule.url_prefix {
                plan.add_message(DisplayMessage::Detail {
                    label: "url_prefix".to_string(),
                    value: url_prefix.clone(),
                });
            }
            if !rule.path_prefixes.is_empty()
                && let Some(ref prefix_name) = rule.path_prefix_name
            {
                plan.add_message(DisplayMessage::Detail {
                    label: prefix_name.clone(),
                    value: format!("{:?}", rule.path_prefixes),
                });
            }
        }

        plan.add_message(DisplayMessage::Section {
            title: "[sync]".to_string(),
        });
        plan.add_message(DisplayMessage::Skip {
            msg: format!(
                "skip_push_remotes = {:?}",
                ctx.config.sync.skip_push_remotes
            ),
        });

        plan.add_message(DisplayMessage::Section {
            title: "[gitlab]".to_string(),
        });
        if ctx.gitlab_config.servers.is_empty() {
            plan.add_message(DisplayMessage::Skip {
                msg: "未配置 GitLab 服务器 (使用 pma gitlab login 添加)".to_string(),
            });
        } else {
            // 只显示 URL 与协议，绝不把 token 打进屏幕/日志
            for srv in &ctx.gitlab_config.servers {
                plan.add_message(DisplayMessage::Detail {
                    label: srv.url.clone(),
                    value: srv.protocol.clone(),
                });
            }
        }

        Ok(plan)
    }

    fn execute(&self, plan: &ExecutionPlan) -> Result<ExecutionResult> {
        plan::run_plan(plan)
    }
}

impl Command for PathArgs {
    type Context = ConfigPathContext;
    type Plan = ExecutionPlan;

    fn collect(&self) -> Result<ConfigPathContext> {
        Ok(ConfigPathContext {
            base_dir: ConfigManager::base_dir(),
        })
    }

    fn plan(&self, ctx: &ConfigPathContext) -> Result<ExecutionPlan> {
        let mut plan = ExecutionPlan::new();
        // 只输出裸路径，不带标签或装饰，便于 `$(pma config path)` 之类的脚本消费
        plan.add_message(DisplayMessage::Skip {
            msg: ctx.base_dir.display().to_string(),
        });
        Ok(plan)
    }

    fn execute(&self, plan: &ExecutionPlan) -> Result<ExecutionResult> {
        plan::run_plan(plan)
    }
}

/// 分发 `pma config` 子命令
pub fn run(args: ConfigArgs) -> Result<()> {
    match args {
        ConfigArgs::Init => Command::run(&InitArgs),
        ConfigArgs::Show => Command::run(&ShowArgs),
        ConfigArgs::Path => Command::run(&PathArgs),
    }
}
