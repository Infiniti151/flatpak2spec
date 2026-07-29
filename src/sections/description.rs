// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Infiniti151

use crate::utils;
use regex::Regex;
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
        if utils::has_metainfo_file(workspace_path) {
            if let Some(desc) = Self::extract_from_metainfo(workspace_path) {
                return Some(desc);
            }
        }

        // 2. .desktop Comment= key
        if utils::has_desktop_file(workspace_path) {
            if let Some(comment) = Self::extract_from_desktop(workspace_path) {
                return Some(comment);
            }
        }

        // 3. First paragraph of README.md
        if let Some(readme) = Self::extract_from_readme(workspace_path) {
            return Some(readme);
        }

        None
    }

    fn extract_from_metainfo(workspace_path: &Path) -> Option<String> {
        let xml_re = Regex::new(r"(?s)<description>(.*?)</description>").ok()?;
        let files = utils::find_matching_files(workspace_path, &["metainfo.xml", "appdata.xml"]);

        for path in files {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Some(caps) = xml_re.captures(&content) {
                    let formatted = Self::format_html_description(&caps[1]);
                    if !formatted.is_empty() {
                        return Some(formatted);
                    }
                }
            }
        }
        None
    }

    fn extract_from_desktop(workspace_path: &Path) -> Option<String> {
        let comment_re = Regex::new(r"(?m)^_?Comment=(.+)$").ok()?;
        let files = utils::find_matching_files(workspace_path, &[".desktop"]);

        for path in files {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Some(caps) = comment_re.captures(&content) {
                    let comment = caps[1].trim();
                    if !comment.is_empty() {
                        return Some(comment.to_string());
                    }
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

    fn format_html_description(raw: &str) -> String {
        let tag_strip_re = Regex::new(r"<[^>]*>").unwrap();
        let ws_re = Regex::new(r"\s+").unwrap();

        let p_re = Regex::new(r"(?is)<p\b[^>]*>(.*?)</p>").unwrap();
        let ul_re = Regex::new(r"(?is)<ul\b[^>]*>(.*?)</ul>").unwrap();
        let ol_re = Regex::new(r"(?is)<ol\b[^>]*>(.*?)</ol>").unwrap();
        let li_re = Regex::new(r"(?is)<li\b[^>]*>(.*?)</li>").unwrap();

        let clean_text = |text: &str| -> String {
            let stripped = tag_strip_re.replace_all(text, "");
            ws_re.replace_all(&stripped, " ").trim().to_string()
        };

        struct ParsedElement {
            index: usize,
            is_list: bool,
            content: String,
        }

        let mut elements = Vec::new();

        // 1. Extract <p> tags
        for cap in p_re.captures_iter(raw) {
            if let Some(m) = cap.get(0) {
                let cleaned = clean_text(&cap[1]);
                if !cleaned.is_empty() {
                    elements.push(ParsedElement {
                        index: m.start(),
                        is_list: false,
                        content: cleaned,
                    });
                }
            }
        }

        // Helper for <ul> and <ol>
        let mut parse_list = |list_re: &Regex| {
            for cap in list_re.captures_iter(raw) {
                if let Some(m) = cap.get(0) {
                    let items: Vec<String> = li_re
                        .captures_iter(&cap[1])
                        .map(|ic| clean_text(&ic[1]))
                        .filter(|item| !item.is_empty())
                        .map(|item| format!("- {}", item))
                        .collect();

                    if !items.is_empty() {
                        elements.push(ParsedElement {
                            index: m.start(),
                            is_list: true,
                            content: items.join("\n"),
                        });
                    }
                }
            }
        };

        // 2. Extract <ul> and <ol> tags
        parse_list(&ul_re);
        parse_list(&ol_re);

        if elements.is_empty() {
            return String::new();
        }

        // Sort by position in original HTML string
        elements.sort_by_key(|e| e.index);

        // 3. Assemble with context-aware line spacing
        let mut result_parts = Vec::new();
        for (idx, elem) in elements.iter().enumerate() {
            result_parts.push(elem.content.as_str());

            if idx + 1 < elements.len() {
                let next = &elements[idx + 1];
                if !elem.is_list && next.is_list {
                    // <p> immediately followed by list -> single newline
                    result_parts.push("\n");
                } else {
                    // <p> to <p>, list to <p>, or list to list -> double newline
                    result_parts.push("\n\n");
                }
            }
        }

        result_parts.concat()
    }
}
