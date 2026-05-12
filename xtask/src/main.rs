//! `xtask` — internal automation runner.
//!
//! Subcommands cover:
//!   - codegen-semconv: regenerate mara-schema from pinned OTel
//!     semconv commit (lands in M1).
//!   - release: tag, build, sign, publish (lands in M5).
//!   - license-check: enforce license headers (lands in M0).

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "xtask", about = "Mara internal automation runner")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Regenerate `mara-schema` types from the pinned OTel semconv commit.
    CodegenSemconv,
    /// Run release workflow steps locally.
    Release,
    /// Enforce license headers across the repo.
    LicenseCheck,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::CodegenSemconv => {
            println!("xtask codegen-semconv — not yet implemented (M1)");
        }
        Command::Release => {
            println!("xtask release — not yet implemented (M5)");
        }
        Command::LicenseCheck => {
            println!("xtask license-check — not yet implemented");
        }
    }
}
