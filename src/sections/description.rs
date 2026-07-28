// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Infiniti151

use regex::Regex;
use std::fs;
use std::path::Path;

pub struct DescriptionSection;

impl DescriptionSection {
    pub fn generate(workspace_path: &Path, header: &str) -> String {
        let desc_body =
            Self::detect_description(workspace_path).unwrap_or_else(|| format!("{}.", header));

        format!("%description\n{}\n", desc_body)
    }

    fn detect_description(workspace_path: &Path) -> Option<String> {
        // 1. AppStream MetaInfo XML primary <description>
        if let Some(desc) = Self::extract_from_metainfo(workspace_path) {
            return Some(desc);
        }

        // 2. .desktop Comment= key
        if let Some(comment) = Self::extract_from_desktop(workspace_path) {
            return Some(comment);
        }

        // 3. First paragraph of README.md
        if let Some(readme) = Self::extract_from_readme(workspace_path) {
            return Some(readme);
        }

        None
    }

    fn extract_from_metainfo(dir: &Path) -> Option<String> {
        let xml_re = Regex::new(r"(?s)<component[^>]*>.*?<description>(.*?)</description>")
            .ok()
            .or_else(|| Regex::new(r"(?s)<description>(.*?)</description>").ok())?;

        let entries = fs::read_dir(dir).ok()?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(desc) = Self::extract_from_metainfo(&path) {
                    return Some(desc);
                }
            } else if let Some(file_name) = path.file_name().and_then(|f| f.to_str()) {
                let lower = file_name.to_lowercase();
                if lower.contains("metainfo.xml") || lower.contains("appdata.xml") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Some(caps) = xml_re.captures(&content) {
                            let raw_desc = &caps[1];
                            let formatted = Self::format_html_description(raw_desc);
                            if !formatted.is_empty() {
                                return Some(formatted);
                            }
                        }
                    }
                }
            }
        }
        None
    }

    fn format_html_description(raw: &str) -> String {
        let tag_ws_re = Regex::new(r"\s+").unwrap();

        let p_re = Regex::new(r"(?is)<p\b[^>]*>(.*?)</p>").unwrap();
        let ul_re = Regex::new(r"(?is)<ul\b[^>]*>(.*?)</ul>").unwrap();
        let ol_re = Regex::new(r"(?is)<ol\b[^>]*>(.*?)</ol>").unwrap();

        struct ParsedElement {
            index: usize,
            is_list: bool,
            content: String,
        }

        let mut elements = Vec::new();

        for cap in p_re.captures_iter(raw) {
            if let Some(m) = cap.get(0) {
                let inner = &cap[1];
                let cleaned_p = tag_ws_re
                    .replace_all(&Regex::new(r"<[^>]*>").unwrap().replace_all(inner, ""), " ")
                    .trim()
                    .to_string();
                if !cleaned_p.is_empty() {
                    elements.push(ParsedElement {
                        index: m.start(),
                        is_list: false,
                        content: cleaned_p,
                    });
                }
            }
        }

        let parse_list = |list_str: &str, elements: &mut Vec<ParsedElement>, start_idx: usize| {
            let mut list_items = Vec::new();
            let item_re = Regex::new(r"(?is)<li\b[^>]*>((.*?)?)</li>").unwrap();
            for item_cap in item_re.captures_iter(list_str) {
                let item_clean = tag_ws_re
                    .replace_all(
                        &Regex::new(r"<[^>]*>")
                            .unwrap()
                            .replace_all(&item_cap[1], ""),
                        " ",
                    )
                    .trim()
                    .to_string();
                if !item_clean.is_empty() {
                    list_items.push(format!("- {}", item_clean));
                }
            }
            if !list_items.is_empty() {
                elements.push(ParsedElement {
                    index: start_idx,
                    is_list: true,
                    content: list_items.join("\n"),
                });
            }
        };

        for cap in ul_re.captures_iter(raw) {
            if let Some(m) = cap.get(0) {
                parse_list(&cap[1], &mut elements, m.start());
            }
        }

        for cap in ol_re.captures_iter(raw) {
            if let Some(m) = cap.get(0) {
                parse_list(&cap[1], &mut elements, m.start());
            }
        }

        // Sort elements by their appearance order in the source XML
        elements.sort_by_key(|e| e.index);

        let mut result_parts = Vec::new();
        for (idx, elem) in elements.iter().enumerate() {
            result_parts.push(elem.content.clone());

            if idx + 1 < elements.len() {
                let next = &elements[idx + 1];
                if !elem.is_list && !next.is_list {
                    // P followed by P -> empty line
                    result_parts.push("\n\n".to_string());
                } else if !elem.is_list && next.is_list {
                    // P followed by List -> NO empty line
                    result_parts.push("\n".to_string());
                } else if elem.is_list && !next.is_list {
                    // List followed by P -> empty line
                    result_parts.push("\n\n".to_string());
                } else {
                    // List followed by List -> empty line
                    result_parts.push("\n\n".to_string());
                }
            }
        }

        result_parts.concat()
    }

    fn extract_from_desktop(dir: &Path) -> Option<String> {
        let comment_re = Regex::new(r"(?m)^_?Comment=(.+)$").ok()?;
        let entries = fs::read_dir(dir).ok()?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(comment) = Self::extract_from_desktop(&path) {
                    return Some(comment);
                }
            } else if let Some(file_name) = path.file_name().and_then(|f| f.to_str()) {
                if file_name.to_lowercase().contains(".desktop") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Some(caps) = comment_re.captures(&content) {
                            let comment = caps[1].trim();
                            if !comment.is_empty() {
                                return Some(comment.to_string());
                            }
                        }
                    }
                }
            }
        }
        None
    }

    fn extract_from_readme(workspace_path: &Path) -> Option<String> {
        let readme_path = workspace_path.join("README.md");
        if !readme_path.exists() {
            return None;
        }

        let content = fs::read_to_string(readme_path).ok()?;
        let mut paragraph = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("#") || trimmed.starts_with("[!") || trimmed.starts_with("<img")
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
