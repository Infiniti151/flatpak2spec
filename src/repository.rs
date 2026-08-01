// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Infiniti151

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
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
            // Re-use our extract_repo_name helper
            let repo_name = Self::extract_repo_name(Path::new(input), input);

            let workspace_dir = std::env::temp_dir()
                .join("flatpak2spec_workspace")
                .join(&repo_name);

            // Check if workspace is already fully populated and is a valid git repository
            if workspace_dir.exists() && workspace_dir.join(".git").exists() {
                println!(
                    "Found existing workspace at {}, updating...",
                    workspace_dir.display()
                );

                let _ = Command::new("git")
                    .args(["pull", "--ff-only", "--recurse-submodules"])
                    .current_dir(&workspace_dir)
                    .status();

                let _ = Command::new("git")
                    .args(["submodule", "update", "--init", "--recursive", "--depth=1"])
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

    /// Extracts the repository name from a Git URL or local workspace directory path.
    /// Preserves exact upstream casing and formatting (e.g., "NetPeek", "mission-center").
    pub fn extract_repo_name(workspace_path: &Path, repo_url: &str) -> String {
        let clean_url = repo_url.trim_end_matches('/').trim_end_matches(".git");

        if let Some(repo_name) = clean_url.split('/').last() {
            if !repo_name.is_empty() && !repo_name.starts_with('.') {
                return repo_name.to_string();
            }
        }

        // Fallback to workspace directory name
        workspace_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("app")
            .to_string()
    }

    /// Resolves the canonical HTTPS web URL from a workspace path (checking git remotes if local).
    pub fn resolve_web_url(workspace_path: &Path) -> String {
        let mut url = String::new();

        // 1. Try reading git remote origin inside workspace_path
        if workspace_path.join(".git").exists() {
            if let Ok(output) = Command::new("git")
                .args(["remote", "get-url", "origin"])
                .current_dir(workspace_path)
                .output()
            {
                if output.status.success() {
                    let remote = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !remote.is_empty() {
                        url = remote;
                    }
                }
            }
        }

        // 2. Convert SSH remotes (git@github.com:user/repo.git) -> (https://github.com/user/repo)
        if url.starts_with("git@") {
            url = url.replace(':', "/").replace("git@", "https://");
        }

        url.trim_end_matches(".git")
            .trim_end_matches('/')
            .to_string()
    }
}
