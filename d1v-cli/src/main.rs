use std::io::{IsTerminal, stdin};
use std::process::ExitCode;

use clap::{ArgMatches, CommandFactory, FromArgMatches, Parser, Subcommand, parser::ValueSource};
use colorchoice_clap::Color;
use futures_util::FutureExt as _;
use jiff::SignedDuration;
use tracing::info;

use d1v_cli::banner::Banner;
use d1v_cli::config::Config;
use d1v_cli::config::cmd::ConfigKey;
use d1v_cli::error::{Error, Result};
use d1v_cli::output::{Format, Output, format_duration};
use d1v_cli::token::TokenSource;
use d1v_cli::{
    BaseUrlCandidate, Context, agent, api_key, auth, base_url, config, db, debug, deploy, env,
    expose, github, i18n, logging, node, project, runtime_install, session, shell, skill, t,
    upgrade, user, workspace,
};

#[derive(Parser)]
#[command(name = "d1v", version, before_help = banner())]
struct Cli {
    /// Output format
    #[arg(short, long, global = true, default_value_t, env = "D1V_FORMAT")]
    format: Format,

    /// Color output
    #[command(flatten)]
    color: Color,

    /// Language override
    #[arg(long, global = true)]
    lang: Option<String>,

    /// Log file path [default: ~/.d1v/d1v.YYYY-MM-DD.log]
    #[arg(long, env = "D1V_LOG_FILE")]
    log_file: Option<std::path::PathBuf>,

    /// Override API base URL
    #[arg(long, global = true, env = "D1V_BASE_URL")]
    base_url: Option<String>,

    /// Increase log verbosity (-v info, -vv debug, -vvv trace)
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Save HTTP exchanges to a JSON file [default: ~/.d1v/recordings/{date}.json]
    #[cfg(feature = "record")]
    #[arg(long, value_name = "FILE", num_args = 0..=1, env = "D1V_RECORD_FILE")]
    record: Option<Option<std::path::PathBuf>>,

    /// Deploy the current directory to Preview without a subcommand
    #[arg(long, alias = "prev", conflicts_with = "prod")]
    preview: bool,

    /// Deploy the current directory to production without a subcommand
    #[arg(long, conflicts_with = "preview")]
    prod: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

fn banner() -> String {
    Banner::new()
        .padding_top("\n\n\n")
        .padding_bottom("")
        .render()
}

#[derive(Subcommand)]
enum Command {
    /// Manage authentication
    Auth {
        #[command(subcommand)]
        command: AuthCommand,
    },
    /// Manage user account
    User {
        #[command(subcommand)]
        command: user::UserCommand,
    },
    /// Manage CLI configuration
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    /// Manage projects
    Project {
        #[command(subcommand)]
        command: project::ProjectCommand,
    },
    /// Manage GitHub App connection
    Github {
        #[command(subcommand)]
        command: github::GitHubCommand,
    },
    /// Manage database workflows
    Db {
        #[command(subcommand)]
        command: db::DbCommand,
    },
    /// Manage deployments
    Deploy {
        #[command(subcommand)]
        command: deploy::DeployCommand,
    },
    /// Manage public ingress bindings
    Expose(expose::ExposeArgs),
    /// Manage node-backed ingress bindings
    Node {
        #[command(subcommand)]
        command: node::NodeCommand,
    },
    /// Manage project environment variables
    Env {
        #[command(subcommand)]
        command: env::EnvCommand,
    },
    /// Manage API keys
    ApiKey {
        #[command(subcommand)]
        command: api_key::ApiKeyCommand,
    },
    /// Manage AI runtime sessions
    Session {
        #[command(subcommand)]
        command: session::SessionCommand,
    },
    /// Open an interactive shell in a workspace container
    Shell(shell::ShellArgs),
    /// Execute a command in a workspace container
    Exec(shell::ExecArgs),
    /// Manage local device agent runtime
    Agent {
        #[command(subcommand)]
        command: agent::AgentCommand,
    },
    /// Manage local opcode-api runtime installation
    Runtime {
        #[command(subcommand)]
        command: RuntimeCommand,
    },
    /// Install or update d1v instructions for coding agents
    Skill {
        #[command(subcommand)]
        command: skill::SkillCommand,
    },
    /// Initialize a local directory as a d1v workspace
    Init(workspace::InitArgs),
    /// Inspect local workspace pull readiness
    Pull(workspace::PullArgs),
    /// Push local git commits to the bound project repository branch
    Push(workspace::PushArgs),
    /// Show debug information
    Debug,
    /// Check for a newer release and upgrade this CLI
    Upgrade(upgrade::UpgradeArgs),
    /// Uninstall this CLI from the current executable path
    Uninstall(upgrade::UninstallArgs),
    /// Print the ASCII art banner
    Banner,
}

impl Command {
    fn requires_auth(&self) -> bool {
        match self {
            Command::Auth { .. }
            | Command::Config { .. }
            | Command::Debug
            | Command::Runtime { .. }
            | Command::Skill { .. }
            | Command::Upgrade(..)
            | Command::Uninstall(..)
            | Command::Banner => false,
            Command::Node { .. } => false,
            Command::Agent { command } => !matches!(
                command,
                agent::AgentCommand::InitHome(_) | agent::AgentCommand::Status
            ),
            Command::User { command } => command.requires_auth(),
            Command::Project { .. }
            | Command::Github { .. }
            | Command::Db { .. }
            | Command::Deploy { .. }
            | Command::Session { .. }
            | Command::Shell(..)
            | Command::Exec(..)
            | Command::Env { .. }
            | Command::ApiKey { .. } => true,
            Command::Expose(..) => false,
            Command::Init(..) => false,
            Command::Pull(..) | Command::Push(..) => true,
        }
    }
}

#[derive(Subcommand)]
enum RuntimeCommand {
    /// Install opcode-api runtime binary
    Install(runtime_install::InstallRuntimeArgs),
    /// Upgrade opcode-api runtime binary
    Upgrade(runtime_install::UpgradeRuntimeArgs),
    /// Inspect runtime installation and health
    Doctor(runtime_install::DoctorArgs),
}

#[derive(Subcommand)]
enum ConfigCommand {
    /// Show current configuration
    Show,
    /// Get a config value
    Get {
        /// Config key
        key: ConfigKey,
    },
    /// Set a config value
    Set {
        /// Config key
        key: ConfigKey,
        /// New value (empty string clears optional fields)
        value: String,
    },
    /// List available config keys
    List,
    /// Print config file path
    Path,
    /// Reset configuration to defaults
    Reset,
    /// Open config file in editor
    Edit,
}

#[derive(Subcommand)]
enum AuthCommand {
    /// Log in with email and verification code
    Login {
        /// Use password instead of verification code
        #[arg(short, long, conflicts_with = "with_token", conflicts_with = "api_key")]
        password: bool,
        /// Log in with an authentication token
        #[arg(long, conflicts_with = "api_key")]
        with_token: bool,
        /// Log in with an API key
        #[arg(long)]
        api_key: bool,
    },
    /// Log out and clear stored credentials
    Logout,
    /// Show authentication status
    Status,
}

async fn run(cli: Cli, base_url_override: BaseUrlCandidate) -> Result<()> {
    #[cfg(feature = "record")]
    let _recorder = {
        use d1v_cli::config::record_path;
        use d1v_cli::recorder::FileRecorder;

        let path = match cli.record {
            Some(Some(path)) => Some(path),
            Some(None) => {
                let dir = Config::load().ok().and_then(|c| c.record.dir);
                Some(record_path(dir.as_deref())?)
            }
            None => Config::load().ok().and_then(|c| c.record.resolve_path()),
        };

        path.map(|path| d1v_api::set_recorder(FileRecorder::new(path)))
            .transpose()
            .map_err(anyhow::Error::from)?
    };

    let ctx = if matches!(&cli.command, Some(Command::Skill { .. })) {
        Context::new_without_token_lookup(cli.format, cli.color.as_choice(), base_url_override)?
    } else {
        Context::new(cli.format, cli.color.as_choice(), base_url_override)?
    };

    if cli.preview || cli.prod || cli.command.as_ref().is_some_and(Command::requires_auth) {
        if ctx.tokens.lookup()?.is_none() {
            return Err(Error::NotLoggedIn);
        }

        if ctx.client.is_token_expired() {
            if stdin().is_terminal() {
                match auth::prompt_relogin(&ctx).await {
                    Ok(true) => ctx.success(t!("auth-relogin-success")),
                    Ok(false) => return Ok(()),
                    Err(err) => return Err(err),
                }
            } else {
                return Err(Error::TokenExpired);
            }
        }

        if let Some(claims) = ctx.client.claims()
            && let Some(remaining) = claims.expires_in()
            && remaining < SignedDuration::from_hours(24)
        {
            let duration = format_duration(remaining.as_secs());
            ctx.output
                .hint(&t!("warn-token-expiring", duration = duration));
        }
    }

    if cli.preview || cli.prod {
        return d1v_cli::quick_deploy::run(&ctx, cli.preview).await;
    }

    let command = cli
        .command
        .ok_or_else(|| anyhow::anyhow!("a command or --preview/--prod is required"))?;
    match command {
        Command::Auth { command } => match command {
            AuthCommand::Login {
                password,
                with_token,
                api_key,
            } => {
                if api_key {
                    auth::login_with_api_key(&ctx)
                } else if with_token {
                    auth::login_with_token(&ctx)
                } else if password {
                    auth::login(&ctx, true).await
                } else if stdin().is_terminal() {
                    auth::login_interactive(&ctx).await
                } else {
                    auth::login(&ctx, false).await
                }
            }
            AuthCommand::Logout => auth::logout(&ctx),
            AuthCommand::Status => auth::status(&ctx),
        },
        Command::User { command } => user::run(&ctx, command).await,
        Command::Project { command } => project::run(&ctx, command).await,
        Command::Github { command } => github::run(&ctx, command).await,
        Command::Db { command } => db::run(&ctx, command).await,
        Command::Deploy { command } => deploy::run(&ctx, command).await,
        Command::Expose(args) => expose::run(&ctx, args).await,
        Command::Node { command } => node::run(&ctx, command).await,
        Command::Session { command } => session::run(&ctx, command).await,
        Command::Shell(args) => shell::run(&ctx, args).await,
        Command::Exec(args) => shell::run_exec(&ctx, args).await,
        Command::Env { command } => env::run(&ctx, command).await,
        Command::ApiKey { command } => api_key::run(&ctx, command).await,
        Command::Agent { command } => agent::run(&ctx, command).await,
        Command::Runtime { command } => match command {
            RuntimeCommand::Install(args) => runtime_install::run_install(&ctx, args).await,
            RuntimeCommand::Upgrade(args) => runtime_install::run_upgrade(&ctx, args).await,
            RuntimeCommand::Doctor(args) => runtime_install::run_doctor(&ctx, args).await,
        },
        Command::Skill { command } => skill::run(&ctx, command).await,
        Command::Init(args) => workspace::init(&ctx, args).await,
        Command::Pull(args) => workspace::pull(&ctx, args).await,
        Command::Push(args) => workspace::push(&ctx, args).await,
        Command::Config { command } => match command {
            ConfigCommand::Show => config::cmd::show(&ctx),
            ConfigCommand::Get { key } => config::cmd::get(&ctx, key),
            ConfigCommand::Set { key, value } => config::cmd::set(&ctx, key, &value),
            ConfigCommand::List => config::cmd::list(&ctx),
            ConfigCommand::Path => config::cmd::path(&ctx),
            ConfigCommand::Reset => config::cmd::reset(&ctx),
            ConfigCommand::Edit => config::cmd::edit(),
        },
        Command::Debug => debug::run(&ctx),
        Command::Upgrade(args) => upgrade::run(&ctx, args).await,
        Command::Uninstall(args) => upgrade::run_uninstall(&ctx, args),
        Command::Banner => {
            print!("{}", Banner::new().render());
            Ok(())
        }
    }
}

fn locale_sources(cli_lang: Option<&str>) -> impl Iterator<Item = String> {
    [
        cli_lang.map(ToOwned::to_owned),
        std::env::var("D1V_LANG").ok().filter(|s| !s.is_empty()),
        Config::load().ok().and_then(|c| c.language),
        sys_locale::get_locale(),
    ]
    .into_iter()
    .flatten()
}

fn base_url_candidate(matches: &ArgMatches) -> BaseUrlCandidate {
    let value = matches.get_one::<String>("base_url").cloned();
    match matches.value_source("base_url") {
        Some(ValueSource::CommandLine) => base_url::from_cli(value),
        Some(ValueSource::EnvVariable) => base_url::from_env(value),
        _ => base_url::default(),
    }
}

fn parse_cli() -> (Cli, BaseUrlCandidate) {
    let matches = Cli::command().get_matches();
    let base_url = base_url_candidate(&matches);
    let cli = match Cli::from_arg_matches(&matches) {
        Ok(cli) => cli,
        Err(err) => err.exit(),
    };
    (cli, base_url)
}

#[tokio::main]
async fn main() -> ExitCode {
    let (mut cli, base_url_override) = parse_cli();
    let _log = logging::init(cli.log_file.take(), cli.verbose).ok();
    i18n::init(locale_sources(cli.lang.as_deref()));

    cli.color.write_global();
    let output = Output::new(cli.format, cli.color.as_choice());

    info!(version = env!("CARGO_PKG_VERSION"), "D1V CLI");

    // 后台检查新版本（不阻塞主流程）
    // 仅在 stderr 是终端 + 不是 upgrade/uninstall 命令时执行
    let check_update_task = if std::io::stderr().is_terminal()
        && !matches!(
            cli.command,
            Some(Command::Upgrade(..)) | Some(Command::Uninstall(..))
        ) {
        Some(tokio::spawn(upgrade::check_update_hint()))
    } else {
        None
    };

    let result = match run(cli, base_url_override).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => err.handle(&output),
    };

    // 如果后台任务已完成则呈现提示，未完成则忽略（不等待）
    if let Some(task) = check_update_task {
        if let Some(Ok(Ok(()))) = task.now_or_never() {
            // hint was already printed to stderr
        }
    }

    result
}
