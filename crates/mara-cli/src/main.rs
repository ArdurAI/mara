//! `mara` — AI-native telemetry shipper command-line interface.
//!
//! Subcommands documented in the v1 functional requirements
//! (see plans/04-implementation/01-functional-requirements.md, FR-9).
//! M2 wires `run` and `validate` to live config + pipeline; the
//! remaining subcommands stay as stubs through M2/M3.

mod run;
mod setup;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "mara",
    version,
    author,
    about = "AI-native telemetry shipper for AI agents and LLM workloads",
    long_about = None
)]
struct Cli {
    /// Path to the configuration file.
    #[arg(long, short = 'c', env = "MARA_CONFIG", global = true)]
    config: Option<PathBuf>,

    /// Log level: trace, debug, info, warn, error.
    #[arg(long, env = "MARA_LOG_LEVEL", default_value = "info", global = true)]
    log_level: String,

    /// Log format: text or json.
    #[arg(long, env = "MARA_LOG_FORMAT", default_value = "text", global = true)]
    log_format: String,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Start the agent and run configured pipelines.
    Run,
    /// Validate the configuration file and exit.
    Validate,
    /// Run a single pipeline against a fixture input.
    Test {
        #[command(subcommand)]
        what: TestWhat,
    },
    /// Print diagnostics for adapters, sinks, policy stages, and WAL.
    Diag {
        /// Watch and re-print on change.
        #[arg(long)]
        watch: bool,
        /// JSON output instead of human-readable.
        #[arg(long)]
        json: bool,
    },
    /// Apply a runtime preset (`ollama`, …).
    Setup {
        /// Preset name.
        preset: String,
        /// Overwrite an existing configuration file.
        #[arg(long)]
        force: bool,
    },
    /// Inspect or manage the per-sink dead-letter queue.
    Dlq {
        #[command(subcommand)]
        op: DlqOp,
    },
    /// Print the running version, build commit, and pinned semconv version.
    Version,
    /// Print a shell completion script.
    Completions {
        /// Target shell: bash, zsh, fish, powershell, elvish.
        shell: String,
    },
}

#[derive(Subcommand, Debug)]
enum TestWhat {
    /// Feed a fixture into a configured pipeline and print resulting events.
    Pipeline {
        /// Pipeline name (defaults to the first configured pipeline).
        #[arg(long)]
        name: Option<String>,
        /// Path to the fixture file.
        #[arg(long)]
        input: PathBuf,
        /// Pretty-print the resulting canonical events.
        #[arg(long)]
        pretty: bool,
    },
}

#[derive(Subcommand, Debug)]
enum DlqOp {
    /// List dead-letter entries.
    List,
    /// Show a single dead-letter entry by id.
    Show { id: String },
    /// Replay a dead-letter entry through its original sink.
    Replay { id: String },
    /// Drop a dead-letter entry without replay.
    Drop { id: String },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    init_tracing(&cli.log_level, &cli.log_format);

    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
    rt.block_on(async {
        match cli.command {
            Command::Run => run::run(cli.config.as_deref()).await,
            Command::Validate => run::validate(cli.config.as_deref()),
            Command::Test { what } => match what {
                TestWhat::Pipeline { name, input, pretty } => {
                    tracing::info!(
                        ?name,
                        ?input,
                        pretty,
                        "mara test pipeline — not yet implemented (M2 follow-up)"
                    );
                    Ok(())
                }
            },
            Command::Diag { watch, json } => {
                tracing::info!(watch, json, "mara diag — not yet implemented (M2 follow-up)");
                Ok(())
            }
            Command::Setup { preset, force } => setup::setup(&preset, force),
            Command::Dlq { op } => {
                tracing::info!(?op, "mara dlq — not yet implemented (M2 follow-up)");
                Ok(())
            }
            Command::Version => {
                println!(
                    "mara {} (core {}, schema {})",
                    env!("CARGO_PKG_VERSION"),
                    mara_core::version(),
                    mara_schema::SCHEMA_VERSION
                );
                Ok(())
            }
            Command::Completions { shell } => {
                tracing::info!(shell, "mara completions — not yet implemented (M5)");
                Ok(())
            }
        }
    })
}

fn init_tracing(level: &str, format: &str) {
    use tracing_subscriber::EnvFilter;
    use tracing_subscriber::fmt;

    let filter = EnvFilter::try_new(level).unwrap_or_else(|_| EnvFilter::new("info"));

    let builder = fmt::Subscriber::builder().with_env_filter(filter);
    match format {
        "json" => builder.json().init(),
        _ => builder.init(),
    }
}
