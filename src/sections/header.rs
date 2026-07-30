// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Infiniti151

use crate::forge::Forge;
use crate::manifest::FlatpakManifest;
use crate::meson::MesonProject;
use crate::utils;
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

        let resolved_url = Self::resolve_repo_url(workspace_path, repo_url);
        let clean_url = resolved_url.trim_end_matches(".git");
        let forge = Forge::detect(clean_url);
        let source0 = forge.format_source_url(clean_url, &tag_prefix);

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

        if meson.is_noarch() {
            header.push_str("\n\nBuildArch:      noarch\n\n");
        } else {
            header.push_str("\n\n");
        }

        header
    }

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
                        if remote.starts_with("git@") {
                            return remote.replace(':', "/").replace("git@", "https://");
                        }
                        return remote;
                    }
                }
            }
        }

        repo_url.to_string()
    }

    pub fn detect_version_and_prefix(workspace_path: &Path) -> Option<(String, String)> {
        // 1. Try Git tags first
        if workspace_path.join(".git").exists() {
            if let Ok(output) = Command::new("git")
                .args(["tag", "-l", "--sort=-v:refname"])
                .current_dir(workspace_path)
                .output()
            {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    if let Some(latest_tag) = stdout.lines().next().map(|s| s.trim()) {
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

        // 2. Fallback to AppStream MetaInfo release tag
        if utils::has_metainfo_file(workspace_path) {
            let rel_re = Regex::new(r#"<release\s+version\s*=\s*['"]([^'"]+)['"]"#).ok()?;
            if let Some(version) = Self::extract_from_workspace(
                workspace_path,
                &rel_re,
                &["metainfo.xml", "appdata.xml"],
            ) {
                return Some((version, "".to_string()));
            }
        }

        None
    }

    fn detect_summary(workspace_path: &Path, app_name: &str) -> String {
        // 1. Try MetaInfo / AppData XML <summary>
        if utils::has_metainfo_file(workspace_path) {
            let xml_re = Regex::new(r"(?s)<summary>(.*?)</summary>").unwrap();
            if let Some(summary) = Self::extract_from_workspace(
                workspace_path,
                &xml_re,
                &["metainfo.xml", "appdata.xml"],
            ) {
                let clean = Regex::new(r"<[^>]*>")
                    .unwrap()
                    .replace_all(&summary, "")
                    .trim()
                    .to_string();
                if !clean.is_empty() {
                    return clean;
                }
            }
        }

        // 2. Try Desktop file Comment=
        if utils::has_desktop_file(workspace_path) {
            let comment_re = Regex::new(r"(?m)^_?Comment=(.+)$").unwrap();
            if let Some(comment) =
                Self::extract_from_workspace(workspace_path, &comment_re, &[".desktop"])
            {
                return comment;
            }
        }

        format!("{} application", app_name)
    }

    /// Uses `utils::find_matching_files` to retrieve file paths, then parses regex patterns.
    fn extract_from_workspace(dir: &Path, re: &Regex, patterns: &[&str]) -> Option<String> {
        let matching_files = utils::find_matching_files(dir, patterns);

        for path in matching_files {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Some(caps) = re.captures(&content) {
                    let val = caps[1].trim().to_string();
                    if !val.is_empty() {
                        return Some(val);
                    }
                }
            }
        }

        None
    }
}
