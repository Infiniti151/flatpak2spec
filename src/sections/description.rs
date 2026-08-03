// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Infiniti151

use crate::utils;
use regex::Regex;
use roxmltree::Document;
use std::fs;
use std::path::Path;

pub struct DescriptionSection;

impl DescriptionSection {
    pub fn generate(workspace_path: &Path, summary_or_name: &str) -> String {
        let desc_body = Self::detect_description(workspace_path)
            .unwrap_or_else(|| format!("{}.", summary_or_name));

        format!("%description\n{}\n", desc_body)
    }

    fn detect_description(workspace_path: &Path) -> Option<String> {
        // 1. AppStream MetaInfo XML primary <description>
        if utils::has_metainfo_file(workspace_path)
            && let Some(desc) = Self::extract_from_metainfo(workspace_path)
        {
            return Some(desc);
        }

        // 2. .desktop Comment= key
        if utils::has_desktop_file(workspace_path)
            && let Some(comment) = Self::extract_from_desktop(workspace_path)
        {
            return Some(comment);
        }

        // 3. First paragraph of README.md
        if let Some(readme) = Self::extract_from_readme(workspace_path) {
            return Some(readme);
        }

        None
    }

    fn extract_from_metainfo(workspace_path: &Path) -> Option<String> {
        let metainfo_path = utils::find_metainfo_file(workspace_path)?;
        let content = fs::read_to_string(metainfo_path).ok()?;
        let doc = Document::parse(&content).ok()?;

        // Locate <description>
        let desc_node = doc
            .descendants()
            .find(|n| n.is_element() && n.tag_name().name() == "description")?;

        let formatted = utils::format_appstream_node(desc_node);
        if formatted.is_empty() {
            None
        } else {
            Some(formatted)
        }
    }

    fn extract_from_desktop(workspace_path: &Path) -> Option<String> {
        let comment_re = Regex::new(r"(?m)^_?Comment=(.+)$").ok()?;
        let files = utils::find_matching_files(workspace_path, &[".desktop"]);

        for path in files {
            if let Ok(content) = fs::read_to_string(&path)
                && let Some(caps) = comment_re.captures(&content)
            {
                let comment = caps[1].trim();
                if !comment.is_empty() {
                    return Some(comment.to_string());
                }
            }
        }
        None
    }

    fn extract_from_readme(workspace_path: &Path) -> Option<String> {
        let readme_path = workspace_path.join("README.md");
        let content = fs::read_to_string(readme_path).ok()?;
        let mut paragraph = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('#') || trimmed.starts_with("[!") || trimmed.starts_with("<img")
            {
                if !paragraph.is_empty() {
                    break;
                }
                continue;
            }

            if trimmed.is_empty() {
                if !paragraph.is_empty() {
                    break;
                }
            } else {
                paragraph.push(trimmed);
            }
        }

        if paragraph.is_empty() {
            None
        } else {
            Some(paragraph.join(" "))
        }
    }
}
