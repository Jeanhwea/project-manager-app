use crate::control::pipeline::Pipeline;
use crate::domain::config::ConfigManager;
use crate::domain::config::schema;
use crate::error::{AppError, Result};
use crate::model::plan::{EditOperation, ExecutionPlan, MessageOperation};
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

struct ConfigInitContext {
    base_dir: PathBuf,
    config_path: PathBuf,
    gitlab_path: PathBuf,
}

struct ConfigShowContext {
    base_dir: PathBuf,
    dir_exists: bool,
    config: schema::AppConfig,
    gitlab_config: schema::GitLabConfig,
}

struct ConfigPathContext {
    base_dir: PathBuf,
}

pub fn run(args: ConfigArgs) -> Result<()> {
    match args {
        ConfigArgs::Init => Pipeline::run(InitArgs, get_init_context, make_init_plan),
        ConfigArgs::Show => Pipeline::run(ShowArgs, get_show_context, make_show_plan),
        ConfigArgs::Path => Pipeline::run(PathArgs, get_path_context, make_path_plan),
    }
}

struct InitArgs;
struct ShowArgs;
struct PathArgs;

fn get_init_context(_args: &InitArgs) -> Result<ConfigInitContext> {
    let base_dir = ConfigManager::base_dir();
    if base_dir.exists() {
        return Err(
            AppError::already_exists(format!("配置目录已存在: {}", base_dir.display())).into(),
        );
    }

    Ok(ConfigInitContext {
        base_dir: base_dir.clone(),
        config_path: ConfigManager::config_path(),
        gitlab_path: ConfigManager::gitlab_path(),
    })
}

fn make_init_plan(_args: &InitArgs, ctx: &ConfigInitContext) -> Result<ExecutionPlan> {
    let mut plan = ExecutionPlan::new();

    plan.add(EditOperation::WriteFile {
        path: ctx.config_path.to_string_lossy().to_string(),
        content: schema::default_config_content().to_string(),
        description: "create main config".to_string(),
    });

    plan.add(EditOperation::WriteFile {
        path: ctx.gitlab_path.to_string_lossy().to_string(),
        content: schema::default_gitlab_config_content().to_string(),
        description: "create gitlab config".to_string(),
    });

    plan.add(MessageOperation::Item {
        label: "已创建配置目录".to_string(),
        value: ctx.base_dir.display().to_string(),
    });
    plan.add(MessageOperation::Detail {
        label: "主配置".to_string(),
        value: ctx.config_path.display().to_string(),
    });
    plan.add(MessageOperation::Detail {
        label: "GitLab".to_string(),
        value: ctx.gitlab_path.display().to_string(),
    });

    Ok(plan)
}

fn get_show_context(_args: &ShowArgs) -> Result<ConfigShowContext> {
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

fn make_show_plan(_args: &ShowArgs, ctx: &ConfigShowContext) -> Result<ExecutionPlan> {
    let mut plan = ExecutionPlan::new();

    let dir_status = if ctx.dir_exists {
        String::new()
    } else {
        " (未创建, 使用默认值)".to_string()
    };
    plan.add(MessageOperation::Item {
        label: "配置目录".to_string(),
        value: format!("{}{}", ctx.base_dir.display(), dir_status),
    });

    plan.add(MessageOperation::Section {
        title: "[repository]".to_string(),
    });
    plan.add(MessageOperation::Skip {
        msg: format!("max_depth  = {}", ctx.config.repository.max_depth),
    });
    plan.add(MessageOperation::Skip {
        msg: format!("skip_dirs  = {:?}", ctx.config.repository.skip_dirs),
    });

    plan.add(MessageOperation::Section {
        title: "[remote]".to_string(),
    });
    for rule in &ctx.config.remote.rules {
        plan.add(MessageOperation::Item {
            label: rule.name.clone(),
            value: format!("{:?}", rule.hosts),
        });
        if let Some(ref url_prefix) = rule.url_prefix {
            plan.add(MessageOperation::Detail {
                label: "url_prefix".to_string(),
                value: url_prefix.clone(),
            });
        }
        if !rule.path_prefixes.is_empty()
            && let Some(ref prefix_name) = rule.path_prefix_name
        {
            plan.add(MessageOperation::Detail {
                label: prefix_name.clone(),
                value: format!("{:?}", rule.path_prefixes),
            });
        }
    }

    plan.add(MessageOperation::Section {
        title: "[sync]".to_string(),
    });
    plan.add(MessageOperation::Skip {
        msg: format!("skip_push_hosts = {:?}", ctx.config.sync.skip_push_hosts),
    });

    plan.add(MessageOperation::Section {
        title: "[gitlab]".to_string(),
    });
    if ctx.gitlab_config.servers.is_empty() {
        plan.add(MessageOperation::Skip {
            msg: "未配置 GitLab 服务器 (使用 pma gitlab login 添加)".to_string(),
        });
    } else {
        for srv in &ctx.gitlab_config.servers {
            plan.add(MessageOperation::Detail {
                label: srv.url.clone(),
                value: srv.protocol.clone(),
            });
        }
    }

    Ok(plan)
}

fn get_path_context(_args: &PathArgs) -> Result<ConfigPathContext> {
    Ok(ConfigPathContext {
        base_dir: ConfigManager::base_dir(),
    })
}

fn make_path_plan(_args: &PathArgs, ctx: &ConfigPathContext) -> Result<ExecutionPlan> {
    let mut plan = ExecutionPlan::new();
    plan.add(MessageOperation::Skip {
        msg: ctx.base_dir.display().to_string(),
    });
    Ok(plan)
}
