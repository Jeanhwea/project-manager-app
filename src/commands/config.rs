use crate::control::command::Command;
use crate::control::plan;
use crate::domain::config::ConfigManager;
use crate::domain::config::schema;
use crate::error::{AppError, Result};
use crate::model::plan::{DisplayMessage, EditOperation, ExecutionPlan, ExecutionResult, Phase};
use std::path::PathBuf;

#[derive(Debug, clap::Subcommand)]
pub enum ConfigArgs {
    /// Initialize a default configuration file
    Init,
    /// Show current configuration
    Show,
    /// Show configuration file path
    Path,
}

#[derive(Debug)]
pub(crate) struct ConfigInitContext {
    base_dir: PathBuf,
    config_path: PathBuf,
    gitlab_path: PathBuf,
}

#[derive(Debug)]
pub(crate) struct ConfigShowContext {
    base_dir: PathBuf,
    dir_exists: bool,
    config: schema::AppConfig,
    gitlab_config: schema::GitLabConfig,
}

#[derive(Debug)]
pub(crate) struct ConfigPathContext {
    base_dir: PathBuf,
}

struct InitArgs;
struct ShowArgs;
struct PathArgs;

impl Command for InitArgs {
    type Context = ConfigInitContext;
    type Plan = ExecutionPlan;

    fn collect(&self) -> Result<ConfigInitContext> {
        let base_dir = ConfigManager::base_dir();
        if base_dir.exists() {
            return Err(AppError::already_exists(format!(
                "配置目录已存在: {}",
                base_dir.display()
            )));
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
        plan.add_message(DisplayMessage::Skip {
            msg: ctx.base_dir.display().to_string(),
        });
        Ok(plan)
    }

    fn execute(&self, plan: &ExecutionPlan) -> Result<ExecutionResult> {
        plan::run_plan(plan)
    }
}

pub fn run(args: ConfigArgs) -> Result<()> {
    match args {
        ConfigArgs::Init => Command::run(&InitArgs),
        ConfigArgs::Show => Command::run(&ShowArgs),
        ConfigArgs::Path => Command::run(&PathArgs),
    }
}
