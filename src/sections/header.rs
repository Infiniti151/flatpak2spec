// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Infiniti151

use crate::manifest::FlatpakManifest;
use crate::meson::MesonProject;
use regex::Regex;
use std::fs;
use std::path::Path;
use std::process::Command;

pub struct HeaderSection;

impl HeaderSection {
    pub fn generate(
        manifest: &FlatpakManifest,
        meson: &MesonProject,
        workspace_path: &Path,
        repo_url: &str,
        bug_url: Option<&str>,
    ) -> String {
        let fallback_name = manifest
            .get_app_id()
            .as_deref()
            .and_then(|id| id.split('.').last())
            .unwrap_or("app")
            .to_string();

        let name = meson.name.clone().unwrap_or(fallback_name);

        let (version, tag_prefix) = Self::detect_version_and_prefix(workspace_path)
            .unwrap_or_else(|| ("1.0.0".to_string(), "".to_string()));

        let license = meson
            .license
            .clone()
            .unwrap_or_else(|| "GPL-3.0-or-later".to_string());

        let summary = Self::detect_summary(workspace_path, &name);

        // Resolve local path to actual git remote URL if available
        let resolved_url = Self::resolve_repo_url(workspace_path, repo_url);
        let clean_url = resolved_url.trim_end_matches(".git");
        let source0 = Self::format_source_url(clean_url, &tag_prefix);

        let mut header = format!(
            "Name:           {}\n\
             Version:        {}\n\
             Release:        1%{{?dist}}\n\
             Summary:        {}\n\
             License:        {}\n\
             URL:            {}",
            name, version, summary, license, clean_url
        );

        if let Some(url) = bug_url {
            header.push_str(&format!("\nBugURL:         {}\n\n", url));
        } else {
            header.push_str("\n\n");
        }

        header.push_str(&format!("Source0:        {}", source0));

        if meson.is_noarch(workspace_path) {
            header.push_str("\n\nBuildArch:      noarch\n\n");
        } else {
            // A single blank line between Source0 and the next section (BuildRequires)
            header.push_str("\n\n");
        }

        header
    }

    /// Resolves local directory paths to their git origin remote URL if available.
    fn resolve_repo_url(workspace_path: &Path, repo_url: &str) -> String {
        let is_local = !repo_url.starts_with("http://")
            && !repo_url.starts_with("https://")
            && !repo_url.starts_with("git@");

        if is_local && workspace_path.join(".git").exists() {
            if let Ok(output) = Command::new("git")
                .args(["remote", "get-url", "origin"])
                .current_dir(workspace_path)
                .output()
            {
                if output.status.success() {
                    let remote = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !remote.is_empty() {
                        // Normalize SSH/git URLs (git@github.com:user/repo.git -> https://github.com/user/repo)
                        if remote.starts_with("git@") {
                            let normalized = remote.replace(':', "/").replace("git@", "https://");
                            return normalized;
                        }
                        return remote;
                    }
                }
            }
        }

        repo_url.to_string()
    }

    fn detect_version_and_prefix(workspace_path: &Path) -> Option<(String, String)> {
        if workspace_path.join(".git").exists() {
            if let Ok(output) = Command::new("git")
                .args(["tag", "-l", "--sort=-v:refname"])
                .current_dir(workspace_path)
                .output()
            {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    if let Some(latest_tag) = stdout.lines().next().map(|s| s.trim()) {
                        if !latest_tag.is_empty() {
                            if let Some(idx) = latest_tag.find(|c: char| c.is_ascii_digit()) {
                                let prefix = latest_tag[..idx].to_string();
                                let version = latest_tag[idx..].to_string();
                                if !version.is_empty() {
                                    return Some((version, prefix));
                                }
                            }
                        }
                    }
                }
            }
        }

        let rel_re = Regex::new(r#"<release\s+version\s*=\s*['"]([^'"]+)['"]"#).ok()?;
        if let Some(version) = Self::search_metainfo(workspace_path, &rel_re) {
            return Some((version, "".to_string()));
        }

        None
    }

    fn detect_summary(workspace_path: &Path, app_name: &str) -> String {
        let xml_re = Regex::new(r"(?s)<summary>(.*?)</summary>").unwrap();
        if let Some(summary) = Self::search_metainfo(workspace_path, &xml_re) {
            let clean = Regex::new(r"<[^>]*>")
                .unwrap()
                .replace_all(&summary, "")
                .trim()
                .to_string();
            if !clean.is_empty() {
                return clean;
            }
        }

        if let Some(comment) = Self::extract_from_desktop_file(workspace_path) {
            return comment;
        }

        format!("{} application", app_name)
    }

    fn search_metainfo(dir: &Path, re: &Regex) -> Option<String> {
        let entries = fs::read_dir(dir).ok()?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(val) = Self::search_metainfo(&path, re) {
                    return Some(val);
                }
            } else if let Some(file_name) = path.file_name().and_then(|f| f.to_str()) {
                let lower = file_name.to_lowercase();
                if lower.contains("metainfo.xml") || lower.contains("appdata.xml") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Some(caps) = re.captures(&content) {
                            let val = caps[1].trim().to_string();
                            if !val.is_empty() {
                                return Some(val);
                            }
                        }
                    }
                }
            }
        }

        None
    }

    fn extract_from_desktop_file(dir: &Path) -> Option<String> {
        let comment_re = Regex::new(r"(?m)^_?Comment=(.+)$").ok()?;
        let entries = fs::read_dir(dir).ok()?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(comment) = Self::extract_from_desktop_file(&path) {
                    return Some(comment);
                }
            } else if let Some(file_name) = path.file_name().and_then(|f| f.to_str()) {
                if file_name.to_lowercase().contains(".desktop") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Some(caps) = comment_re.captures(&content) {
                            let comment = caps[1].trim().to_string();
                            if !comment.is_empty() {
                                return Some(comment);
                            }
                        }
                    }
                }
            }
        }

        None
    }

    fn format_source_url(clean_url: &str, tag_prefix: &str) -> String {
        if !clean_url.starts_with("http://") && !clean_url.starts_with("https://") {
            return "%{name}-%{version}.tar.gz".to_string();
        }

        let tag_var = if tag_prefix.is_empty() {
            "%{version}".to_string()
        } else {
            format!("{}%{{version}}", tag_prefix)
        };

        if clean_url.contains("gitlab") {
            format!("%{{url}}/-/archive/{}.tar.gz", tag_var)
        } else {
            format!("%{{url}}/archive/{}.tar.gz", tag_var)
        }
    }
}
