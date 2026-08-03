// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Infiniti151

use crate::manifest::FlatpakManifest;
use crate::meson::MesonProject;
use crate::utils;
use regex::Regex;
use std::path::Path;
use std::process::Command;

pub struct HeaderSection;

impl HeaderSection {
    pub fn generate(
        manifest: &FlatpakManifest,
        meson: &MesonProject,
        workspace_path: &Path,
        canonical_url: &str,
        bug_url: Option<&str>,
    ) -> String {
        let app_id = manifest
            .resolve_app_id(workspace_path)
            .unwrap_or_else(|| "app".to_string());

        let name = meson
            .name
            .clone()
            .unwrap_or_else(|| app_id.split('.').next_back().unwrap_or("app").to_string());

        let (version, tag_prefix) = Self::detect_version_and_prefix(workspace_path)
            .unwrap_or_else(|| ("1.0.0".to_string(), "".to_string()));

        let tag_value = format!("{}%{{version}}", tag_prefix);

        let license = meson
            .license
            .clone()
            .unwrap_or_else(|| "GPL-3.0-or-later".to_string());

        let summary = Self::detect_summary(workspace_path, &name);

        let bug_url_line = match bug_url {
            Some(url) => format!("BugURL:         {}\n", url),
            None => String::new(),
        };

        let build_arch_line = if meson.is_noarch() {
            "BuildArch:      noarch\n"
        } else {
            ""
        };

        let mut header = format!(
            "%global         app_id        {}\n\
             %global         forgeurl      {}\n\
             %global         tag           {}\n\n\
             Name:           {}\n\
             Version:        {}\n\
             Release:        1%{{?dist}}\n\
             Summary:        {}\n\
             License:        {}\n\
             {}\
             {}\n\
             %forgemeta\n\n\
             URL:            %{{forgeurl}}\n\
             Source0:        %{{forgesource}}",
            app_id,
            canonical_url,
            tag_value,
            name,
            version,
            summary,
            license,
            bug_url_line,
            build_arch_line
        );

        header.push_str("\n\n");
        header
    }

    pub fn detect_version_and_prefix(workspace_path: &Path) -> Option<(String, String)> {
        // 1. Try Git tags first
        if workspace_path.join(".git").exists()
            && let Ok(output) = Command::new("git")
                .args(["tag", "-l", "--sort=-v:refname"])
                .current_dir(workspace_path)
                .output()
            && output.status.success()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(latest_tag) = stdout.lines().next().map(|s| s.trim())
                && let Some(idx) = latest_tag.find(|c: char| c.is_ascii_digit())
            {
                let prefix = latest_tag[..idx].to_string();
                let version = latest_tag[idx..].to_string();
                if !version.is_empty() {
                    return Some((version, prefix));
                }
            }
        }

        // 2. Fallback to AppStream MetaInfo release tag
        if utils::has_metainfo_file(workspace_path) {
            let rel_re = Regex::new(r#"<release\s+version\s*=\s*['"]([^'"]+)['"]"#).ok()?;
            if let Some(version) = utils::find_and_extract_regex(
                workspace_path,
                &["metainfo.xml", "appdata.xml"],
                &rel_re,
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
            if let Some(summary) = utils::find_and_extract_regex(
                workspace_path,
                &["metainfo.xml", "appdata.xml"],
                &xml_re,
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
                utils::find_and_extract_regex(workspace_path, &[".desktop"], &comment_re)
            {
                return comment;
            }
        }

        format!("{} application", app_name)
    }
}
