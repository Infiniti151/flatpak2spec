// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Infiniti151

use crate::manifest::FlatpakManifest;
use crate::utils;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct RepoResolver;

impl RepoResolver {
    /// Resolves input into a local directory containing the project source.
    /// If input is a URL, it checks if a dedicated workspace for the app already exists.
    /// If it exists and is a valid git repository, it pre-calculates the target release tag:
    /// if the workspace is already checked out to that exact tag, it skips network fetches
    /// and checkout operations entirely; otherwise, it fetches updates and checks out the target tag.
    /// If no workspace exists, it performs a shallow clone into `flatpak2spec_workspace/<app-name>`.
    pub fn prepare_workspace(input: &str) -> Result<PathBuf> {
        let workspace_dir = if input.starts_with("http://")
            || input.starts_with("https://")
            || input.starts_with("git@")
        {
            let repo_name = Self::extract_repo_name(Path::new(input), input);

            let dir = std::env::temp_dir()
                .join("flatpak2spec_workspace")
                .join(&repo_name);

            if dir.exists() && dir.join(".git").exists() {
                utils::print_info(&format!("Found existing workspace at {}", dir.display()));

                let mut needs_fetch = true;

                // 1. Try to pre-calculate the target tag from the existing (cached) manifest
                if let Ok(manifest) = FlatpakManifest::load_from_workspace(&dir) {
                    let app_id = manifest.resolve_app_id(&dir).unwrap_or_default();

                    // Query Flathub / Local tags for the target version
                    if let Some((version, prefix)) = utils::detect_version_and_prefix(&dir, &app_id)
                    {
                        let tag_name = format!("{}{}", prefix, version);

                        // 2. Check if the target tag is already present in the local repository
                        let tag_exists = Command::new("git")
                            .args(["rev-parse", "--verify", "--quiet", &tag_name])
                            .current_dir(&dir)
                            .output()
                            .map(|out| out.status.success())
                            .unwrap_or(false);

                        if tag_exists {
                            needs_fetch = false;

                            // 3. Check if this exact tag is currently checked out
                            let current_tag = Command::new("git")
                                .args(["describe", "--tags", "--exact-match"])
                                .current_dir(&dir)
                                .output()
                                .ok()
                                .and_then(|out| {
                                    if out.status.success() {
                                        Some(
                                            String::from_utf8_lossy(&out.stdout).trim().to_string(),
                                        )
                                    } else {
                                        None
                                    }
                                });

                            if current_tag.as_deref() == Some(&tag_name) {
                                utils::print_info(&format!(
                                    "Workspace is already up to date at tag '{}'. Skipping fetch.",
                                    tag_name
                                ));
                            } else {
                                utils::print_info(&format!(
                                    "Target tag '{}' exists locally. Skipping fetch.",
                                    tag_name
                                ));
                            }
                        }
                    }
                }

                // 4. Fetch only if we don't have the tag we need
                if needs_fetch {
                    utils::print_info("Fetching remote updates...");
                    let _ = Command::new("git")
                        .args(["fetch", "--all", "--tags", "--prune"])
                        .current_dir(&dir)
                        .output();
                }
            } else {
                if let Some(parent) = dir.parent() {
                    std::fs::create_dir_all(parent)?;
                }

                utils::print_info(&format!(
                    "Cloning workspace from {} to {}...",
                    input,
                    dir.display()
                ));

                // Silence git clone terminal progress output with .output()
                let output = Command::new("git")
                    .args([
                        "clone",
                        "--depth=1",
                        "--recurse-submodules",
                        "--shallow-submodules",
                        input,
                        dir.to_str().unwrap(),
                    ])
                    .output()
                    .context("Failed to run git clone. Is git installed?")?;

                if !output.status.success() {
                    anyhow::bail!("Failed to clone Git repository: {}", input);
                }
            }

            dir
        } else {
            let path = PathBuf::from(input);
            if !path.exists() {
                anyhow::bail!("Local path does not exist: {}", input);
            }
            path
        };

        // 5. Checkout the target release tag if we aren't already on it
        if workspace_dir.join(".git").exists()
            && let Ok(manifest) = FlatpakManifest::load_from_workspace(&workspace_dir)
        {
            let app_id = manifest.resolve_app_id(&workspace_dir).unwrap_or_default();

            if let Some((version, prefix)) =
                utils::detect_version_and_prefix(&workspace_dir, &app_id)
            {
                let tag_name = format!("{}{}", prefix, version);

                let current_tag = Command::new("git")
                    .args(["describe", "--tags", "--exact-match"])
                    .current_dir(&workspace_dir)
                    .output()
                    .ok()
                    .and_then(|o| {
                        if o.status.success() {
                            Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
                        } else {
                            None
                        }
                    });

                if current_tag.as_deref() != Some(&tag_name) {
                    utils::print_info(&format!("Checking out tag {}...", tag_name));

                    // Explicitly fetch tag refspec into local tags if shallow clone missed it
                    let tag_refspec = format!("refs/tags/{}:refs/tags/{}", tag_name, tag_name);
                    let _ = Command::new("git")
                        .args(["fetch", "origin", &tag_refspec, "--depth=1"])
                        .current_dir(&workspace_dir)
                        .output();

                    let _ = Command::new("git")
                        .args(["checkout", &tag_name])
                        .current_dir(&workspace_dir)
                        .output();

                    // Always update submodules after a checkout changes the working tree
                    let _ = Command::new("git")
                        .args(["submodule", "update", "--init", "--recursive"])
                        .current_dir(&workspace_dir)
                        .output();
                }
            }
        }

        Ok(workspace_dir)
    }

    /// Extracts the repository name from a Git URL or local workspace directory path.
    /// Preserves exact upstream casing and formatting (e.g., "NetPeek", "mission-center").
    pub fn extract_repo_name(workspace_path: &Path, repo_url: &str) -> String {
        let clean_url = repo_url.trim_end_matches('/').trim_end_matches(".git");

        if let Some(repo_name) = clean_url.split('/').next_back()
            && !repo_name.is_empty()
            && !repo_name.starts_with('.')
        {
            return repo_name.to_string();
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
        if workspace_path.join(".git").exists()
            && let Ok(output) = Command::new("git")
                .args(["remote", "get-url", "origin"])
                .current_dir(workspace_path)
                .output()
            && output.status.success()
        {
            let remote = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !remote.is_empty() {
                url = remote;
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
