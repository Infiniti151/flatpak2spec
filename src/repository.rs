// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Infiniti151

use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::Command;

pub struct RepoResolver;

impl RepoResolver {
    /// Resolves input into a local directory containing the project source.
    /// If input is a URL, it checks if a dedicated workspace for the app already exists.
    /// If it exists and is a valid git repository, it reuses it (with a pull/fetch);
    /// otherwise, it performs a shallow clone into `flatpak2spec_workspace/<app-name>`.
    pub fn prepare_workspace(input: &str) -> Result<PathBuf> {
        if input.starts_with("http://")
            || input.starts_with("https://")
            || input.starts_with("git@")
        {
            // Extract a clean app name from the URL (e.g., .../repo.git -> repo)
            let repo_name = input
                .trim_end_matches('/')
                .trim_end_matches(".git")
                .split('/')
                .last()
                .unwrap_or("app");

            let workspace_dir = std::env::temp_dir()
                .join("flatpak2spec_workspace")
                .join(repo_name);

            // Check if workspace is already fully populated and is a valid git repository
            if workspace_dir.exists() && workspace_dir.join(".git").exists() {
                println!(
                    "Found existing workspace at {}, updating...",
                    workspace_dir.display()
                );

                // Fast update instead of full re-clone
                let _ = Command::new("git")
                    .args(["pull", "--ff-only"])
                    .current_dir(&workspace_dir)
                    .status();

                let _ = Command::new("git")
                    .args(["fetch", "--tags", "--depth=1"])
                    .current_dir(&workspace_dir)
                    .output();

                return Ok(workspace_dir);
            }

            // Ensure parent directory exists
            if let Some(parent) = workspace_dir.parent() {
                std::fs::create_dir_all(parent)?;
            }

            println!(
                "Cloning workspace from {} to {}...",
                input,
                workspace_dir.display()
            );

            // Perform a clean shallow clone into the specific app subdirectory
            let status = Command::new("git")
                .args([
                    "clone",
                    "--depth=1",
                    "--recurse-submodules",
                    "--shallow-submodules",
                    input,
                    workspace_dir.to_str().unwrap(),
                ])
                .status()
                .context("Failed to run git clone. Is git installed?")?;

            if !status.success() {
                anyhow::bail!("Failed to clone Git repository: {}", input);
            }

            // Fetch tags shallowly so git tag -l can see them
            let _ = Command::new("git")
                .args(["fetch", "--tags", "--depth=1"])
                .current_dir(&workspace_dir)
                .output();

            Ok(workspace_dir)
        } else {
            let path = PathBuf::from(input);
            if !path.exists() {
                anyhow::bail!("Local path does not exist: {}", input);
            }
            Ok(path)
        }
    }
}
