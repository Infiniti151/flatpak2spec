// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Infiniti151

use anyhow::Result;
use clap::Parser;
use std::fs;
use std::path::PathBuf;

mod forge;
mod manifest;
mod meson;
mod repository;
mod sections;
mod spec;
mod utils;

use manifest::FlatpakManifest;
use meson::MesonProject;
use repository::RepoResolver;
use spec::SpecGenerator;
use utils::{print_error, print_info, print_success};

#[derive(Parser, Debug)]
#[command(
    name = "flatpak2spec",
    author = "Infiniti151",
    version,
    about = "Generates an RPM spec file from a Flatpak application repository"
)]
pub struct Cli {
    /// Path or URL to the Flatpak application repository
    #[arg(short, long, value_name = "PATH_OR_URL")]
    pub repo: String,

    /// Bug tracker URL for the BugURL spec field (optional)
    #[arg(short = 'b', long, value_name = "URL")]
    pub bug_url: Option<String>,

    /// Packager name for the %changelog section (optional)
    #[arg(short, long, value_name = "PACKAGER")]
    pub packager: Option<String>,

    /// Packager email for the %changelog section (optional)
    #[arg(short, long, value_name = "EMAIL")]
    pub email: Option<String>,

    /// Optional output file path (defaults to stdout if omitted)
    #[arg(short, long, value_name = "FILE")]
    pub output: Option<PathBuf>,

    /// Enable verbose logging output
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,
}

impl Cli {
    /// Resolves packager name with fallback
    pub fn packager_name(&self) -> &str {
        self.packager.as_deref().unwrap_or("Packager")
    }

    /// Resolves packager email with fallback
    pub fn packager_email(&self) -> &str {
        self.email.as_deref().unwrap_or("packager@localhost")
    }
}

/// Resolves the output path:
/// - If None: returns None (stdout).
/// - If Directory: appends `<default_name>.spec`.
/// - If File Path: ensures extension is `.spec`.
pub fn resolve_output_path(output_arg: Option<PathBuf>, default_name: &str) -> Option<PathBuf> {
    output_arg.map(|path| {
        if path.is_dir() {
            path.join(format!("{default_name}.spec"))
        } else {
            let mut p = path;
            p.set_extension("spec");
            p
        }
    })
}

fn main() {
    if let Err(err) = run() {
        print_error(&format!("\nError: {:#}", err));
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    if cli.verbose > 0 {
        print_info(&format!("Verbosity level: {}", cli.verbose));
        if let Some(ref url) = cli.bug_url {
            println!("Bug URL:     {url}");
        }
    }

    // 1. Prepare Workspace
    print_info(&format!("Preparing workspace for: {}", cli.repo));
    let workspace = RepoResolver::prepare_workspace(&cli.repo)?;
    print_info(&format!("Workspace ready at: {}", workspace.display()));

    // 2. Load Manifest & Meson Metadata
    print_info("Loading and validating Flatpak manifest...");
    let manifest = FlatpakManifest::load_from_workspace(&workspace)?;

    print_info("Parsing Meson build configuration...");
    let meson_proj = MesonProject::parse_from_workspace(&workspace)?;

    if cli.verbose > 0 {
        print_info("\n--- Extracted Metadata ---");
        print_info(&format!("  App ID:          {:?}", manifest.id));
        print_info(&format!("  Command:         {:?}", manifest.command));
        print_info(&format!("  Runtime:         {:?}", manifest.runtime));
        print_info(&format!(
            "  Runtime Version: {:?}",
            manifest.runtime_version
        ));
        print_info(&format!("  SDK:             {:?}", manifest.sdk));
        print_info(&format!("  Meson Name:      {:?}", meson_proj.name));
        print_info(&format!("  License:         {:?}", meson_proj.license));
        print_info(&format!("  Is Noarch:       {}", meson_proj.is_noarch()));
    }

    // 3. Resolve Upstream App / Package Name
    let app_name = meson_proj.name.clone().unwrap_or_else(|| {
        manifest
            .get_app_id()
            .as_deref()
            .and_then(|id| id.rsplit('.').next())
            .unwrap_or("app")
            .to_string()
    });

    // 4. Generate Complete Spec Content
    print_info("\nGenerating RPM spec file...");
    let spec_content = SpecGenerator::generate(&manifest, &meson_proj, &workspace, &cli);

    // 5. Output Handling (Write to file or stdout)
    if let Some(out_path) = resolve_output_path(cli.output, &app_name) {
        fs::write(&out_path, &spec_content)?;
        print_success(&format!(
            "Successfully wrote spec file to: {}",
            out_path.display()
        ));
    } else {
        print_success("\n================= GENERATED SPEC =================");
        print_success(&spec_content);
        print_success("=======================================================");
    }

    Ok(())
}
