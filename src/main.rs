// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>
//
// main.rs
//
// Datasheet CLI for extracting structured data from PDF datasheets using LLMs.

use anyhow::{Result, anyhow};
use clap::{Parser, Subcommand};

mod digikey;
mod extract;
mod file_cache;
mod llm;
mod mouser;
mod prompts;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Extract structured JSON data from datasheets using LLMs
    Extract(extract::ExtractArgs),
    /// Mouser Electronics API for searching parts and downloading datasheets
    #[command(subcommand)]
    Mouser(mouser::MouserSubcommand),
    /// DigiKey Electronics API for searching parts and downloading datasheets
    #[command(subcommand)]
    Digikey(digikey::DigikeySubcommand),
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Extract(args) => extract::run_extract(&args),
        Command::Mouser(subcommand) => {
            mouser::execute(subcommand).map_err(|e| anyhow!(e))
        }
        Command::Digikey(subcommand) => {
            digikey::execute(subcommand).map_err(|e| anyhow!(e))
        }
    }
}
