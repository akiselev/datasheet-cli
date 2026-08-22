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
mod extract_pages;
mod fdx;
mod file_cache;
mod footprint_image;
mod jlcpcb;
mod llm;
mod mouser;
mod page_render;
mod pdf_split;
mod prompts;
mod snapeda;
mod svd;

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
    /// JLCPCB/LCSC component search (no API key required)
    #[command(subcommand)]
    Jlcpcb(jlcpcb::JlcpcbSubcommand),
    /// SnapEDA/SnapMagic CAD library — symbols, footprints, pin mappings (no API key required)
    #[command(subcommand)]
    Snapeda(snapeda::SnapedaSubcommand),
    /// Download SVD (System View Description) register map files for microcontrollers
    #[command(subcommand)]
    Svd(svd::SvdSubcommand),
    /// Extract footprint drawings from a PDF datasheet as cropped images
    FootprintImage(footprint_image::FootprintImageArgs),
    /// Extract specific pages/regions from a PDF using LLM-guided detection
    ExtractPages(extract_pages::ExtractPagesArgs),
    /// Build a deterministic FDX1 sketch from a minimized DocumentIR fixture
    Sketch(fdx::SketchArgs),
    /// Compile the frozen FDX2 deterministic analog/power fixture baseline
    Baseline(fdx::BaselineArgs),
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Extract(args) => extract::run_extract(&args),
        Command::Mouser(subcommand) => mouser::execute(subcommand).map_err(|e| anyhow!(e)),
        Command::Digikey(subcommand) => digikey::execute(subcommand).map_err(|e| anyhow!(e)),
        Command::Jlcpcb(subcommand) => jlcpcb::execute(subcommand).map_err(|e| anyhow!(e)),
        Command::Snapeda(subcommand) => snapeda::execute(subcommand).map_err(|e| anyhow!(e)),
        Command::Svd(subcommand) => svd::execute(subcommand).map_err(|e| anyhow!(e)),
        Command::FootprintImage(args) => footprint_image::run(&args),
        Command::ExtractPages(args) => extract_pages::run(&args),
        Command::Sketch(args) => fdx::run_sketch(&args),
        Command::Baseline(args) => fdx::run_baseline(&args),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn existing_remote_extract_command_still_parses() {
        let cli = Cli::try_parse_from([
            "datasheet",
            "extract",
            "pinout",
            "synthetic.pdf",
            "--formatted",
        ])
        .unwrap();
        assert!(matches!(cli.command, Command::Extract(_)));
    }

    #[test]
    fn local_fixture_sketch_command_is_explicit() {
        let digest = "0".repeat(64);
        let cli = Cli::try_parse_from([
            "datasheet",
            "sketch",
            "fixture.json",
            "--code-revision",
            "revision",
            "--configuration-digest",
            &digest,
            "--ontology-snapshot-digest",
            &digest,
        ])
        .unwrap();
        assert!(matches!(cli.command, Command::Sketch(_)));
    }

    #[test]
    fn deterministic_baseline_command_is_explicit() {
        let digest = "0".repeat(64);
        let cli = Cli::try_parse_from([
            "datasheet",
            "baseline",
            "document.json",
            "vocabulary.json",
            "--code-revision",
            "revision",
            "--configuration-digest",
            &digest,
        ])
        .unwrap();
        assert!(matches!(cli.command, Command::Baseline(_)));
    }
}
