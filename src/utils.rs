// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Infiniti151

use colored::Colorize;
use regex::Regex;
use roxmltree::Node;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::manifest::FlatpakManifest;

/// Recursively searches a directory for files matching a specific extension or name snippet,
/// ignoring subprojects, build directories, and hidden files.
pub fn check_file_extension(dir: &Path, ext: &str) -> bool {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return false,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        // Skip hidden dotfiles/directories (.git, .build, etc.), subprojects, and vendor dirs
        if name.starts_with('.')
            || name == "subprojects"
            || name == "build"
            || name == "_build"
            || name == "vendor"
        {
            continue;
        }

        if path.is_dir() {
            if check_file_extension(&path, ext) {
                return true;
            }
        } else if name.ends_with(ext)
            || name.ends_with(&format!("{}.in", ext))
            || name.contains(ext)
        {
            return true;
        }
    }
    false
}

/// Checks if the workspace contains desktop files or templates (.desktop / .desktop.in).
pub fn has_desktop_file(dir: &Path) -> bool {
    check_file_extension(dir, ".desktop")
}

/// Finds and returns the primary AppStream/MetaInfo XML file path within the workspace.
///
/// Automatically attempts to resolve `manifest.id` from the workspace to match
/// exact filenames like `{id}.metainfo.xml` or `{id}.appdata.xml`.
pub fn find_metainfo_file(dir: &Path) -> Option<PathBuf> {
    // Attempt to parse manifest and extract ID internally
    let app_id = FlatpakManifest::load_from_workspace(dir)
        .ok()
        .and_then(|m| m.id);

    if let Some(id) = app_id {
        let metainfo_pattern = format!("{}.metainfo.xml", id);
        let appdata_pattern = format!("{}.appdata.xml", id);

        let matches = find_matching_files(dir, &[&metainfo_pattern, &appdata_pattern]);
        if let Some(path) = matches.into_iter().next() {
            return Some(path);
        }
    }

    // Generic fallback if no ID was found or no exact match was matched
    find_matching_files(dir, &["metainfo.xml", "appdata.xml"])
        .into_iter()
        .next()
}

/// Convenience check returning bool.
pub fn has_metainfo_file(dir: &Path) -> bool {
    find_metainfo_file(dir).is_some()
}

/// Recursively finds all file paths matching a given set of pattern strings,
/// ignoring hidden folders, Meson subprojects, build outputs, and vendor trees.
pub fn find_matching_files(dir: &Path, patterns: &[&str]) -> Vec<PathBuf> {
    let mut matches = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return matches,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        // Skip hidden folders, Meson subprojects, build outputs, and vendor trees
        if name.starts_with('.')
            || name == "subprojects"
            || name == "build"
            || name == "_build"
            || name == "vendor"
        {
            continue;
        }

        if path.is_dir() {
            matches.extend(find_matching_files(&path, patterns));
        } else {
            let lower = name.to_lowercase();
            if patterns
                .iter()
                .any(|pat| lower.contains(&pat.to_lowercase()))
            {
                matches.push(path);
            }
        }
    }
    matches
}

#[derive(Debug, Clone, Default)]
pub struct SystemdUnits {
    pub has_user: bool,
    pub has_system: bool,
    pub unit_names: Vec<String>,
}

/// Detects real systemd unit files in the workspace (ignoring D-Bus service files),
/// and extracts both their scope (User/System) and their clean filenames.
pub fn detect_systemd_units(workspace: &Path) -> SystemdUnits {
    let mut units = SystemdUnits::default();

    // Base patterns (find_matching_files uses `.contains()`, so this covers `.in` variants too)
    let unit_patterns = [".service", ".socket", ".timer", ".path", ".target"];
    let matching_paths = find_matching_files(workspace, &unit_patterns);

    for path in matching_paths {
        let path_str = path.to_string_lossy().to_lowercase();

        // 1. Ignore D-Bus service activation directories
        if path_str.contains("dbus-1") || path_str.contains("dbus-services") {
            continue;
        }

        if let Ok(content) = std::fs::read_to_string(&path) {
            // 2. Ignore D-Bus service activation files by content
            if content.contains("[D-BUS Service]") {
                continue;
            }

            // 3. Determine scope
            if path_str.contains("/user/") || path_str.contains("user-") {
                units.has_user = true;
            } else if path_str.contains("/system/")
                || content.contains("WantedBy=multi-user.target")
                || content.contains("User=root")
            {
                units.has_system = true;
            } else {
                units.has_user = true; // Default fallback
            }
        } else {
            // Fallback if file read fails but it matched the extension
            units.has_user = true;
        }

        // 4. Extract clean unit file name (e.g., "my-daemon.service")
        if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
            let clean_name = file_name.strip_suffix(".in").unwrap_or(file_name);
            if !units.unit_names.contains(&clean_name.to_string()) {
                units.unit_names.push(clean_name.to_string());
            }
        }
    }

    units.unit_names.sort();
    units
}

/// Formats an AppStream XML node containing `<p>`, `<ul>`, `<ol>`, and `<li>` elements
/// into clean, plain text suitable for RPM %description or release notes.
pub fn format_appstream_node(node: Node) -> String {
    let mut elements = Vec::new();

    for child in node
        .descendants()
        .filter(|n| n.is_element() && n.id() != node.id())
    {
        // Skip <li> elements that are inside a list (since ul/ol handles them)
        if child.tag_name().name() == "li"
            && child
                .ancestors()
                .any(|a| a.tag_name().name() == "ul" || a.tag_name().name() == "ol")
        {
            continue;
        }

        match child.tag_name().name() {
            "p" => {
                let text = collect_node_text(child);
                if !text.is_empty() {
                    elements.push((false, text));
                }
            }
            "ul" | "ol" => {
                let mut items = Vec::new();
                for li in child
                    .children()
                    .filter(|n| n.is_element() && n.tag_name().name() == "li")
                {
                    let text = collect_node_text(li);
                    if !text.is_empty() {
                        items.push(format!("- {}", text)); // Re-add bullet prefix for lists
                    }
                }
                if !items.is_empty() {
                    elements.push((true, items.join("\n")));
                }
            }
            _ => {}
        }
    }

    // Assemble with context-aware line spacing
    let mut result_parts = Vec::new();
    for (idx, (is_list, content)) in elements.iter().enumerate() {
        result_parts.push(content.as_str());

        if idx + 1 < elements.len() {
            let (next_is_list, _) = &elements[idx + 1];
            if !is_list && *next_is_list {
                result_parts.push("\n");
            } else {
                result_parts.push("\n\n");
            }
        }
    }

    result_parts.concat()
}

/// Recursively collects and normalizes all inner text within an XML node,
/// stripping inner XML/HTML tags (e.g. `<em>`, `<code>`, `<a>`).
pub fn collect_node_text(node: Node) -> String {
    let mut text_parts = Vec::new();
    for desc in node.descendants() {
        if desc.is_text()
            && let Some(txt) = desc.text()
        {
            text_parts.push(txt);
        }
    }
    let combined = text_parts.join("");
    // Replace contiguous whitespace/newlines with a single space
    combined.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Scans matching files in a directory for a regex pattern and returns the first capture group.
pub fn find_and_extract_regex(dir: &Path, patterns: &[&str], re: &Regex) -> Option<String> {
    let matching_files = find_matching_files(dir, patterns);

    for path in matching_files {
        if let Ok(content) = fs::read_to_string(&path)
            && let Some(caps) = re.captures(&content)
        {
            let val = caps[1].trim().to_string();
            if !val.is_empty() {
                return Some(val);
            }
        }
    }

    None
}

/// Queries Flathub AppStream API to detect the latest released version,
/// cross-references with local or remote Git tags to find the tag prefix (e.g. "v"),
/// and falls back to Git tags if Flathub info is unavailable.
pub fn detect_version_and_prefix(workspace_path: &Path, app_id: &str) -> Option<(String, String)> {
    // Collect git tags if repository exists
    let git_tags: Vec<String> = if workspace_path.join(".git").exists() {
        // 1. Try to fetch tags locally first
        let mut tags = Command::new("git")
            .args(["tag", "-l", "--sort=-v:refname"])
            .current_dir(workspace_path)
            .output()
            .ok()
            .and_then(|output| {
                if output.status.success() {
                    let lines: Vec<String> = String::from_utf8_lossy(&output.stdout)
                        .lines()
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    if lines.is_empty() { None } else { Some(lines) }
                } else {
                    None
                }
            })
            .unwrap_or_default();

        // 2. If local tags are empty (e.g., due to a shallow clone), query remote tags natively
        if tags.is_empty() {
            tags = Command::new("git")
                .args([
                    "ls-remote",
                    "--tags",
                    "--refs",
                    "--sort=-v:refname",
                    "origin",
                ])
                .current_dir(workspace_path)
                .output()
                .ok()
                .and_then(|output| {
                    if output.status.success() {
                        Some(
                            String::from_utf8_lossy(&output.stdout)
                                .lines()
                                .filter_map(|line| {
                                    line.split_whitespace()
                                        .last()?
                                        .strip_prefix("refs/tags/")
                                        .map(|s| s.to_string())
                                })
                                .collect(),
                        )
                    } else {
                        None
                    }
                })
                .unwrap_or_default();
        }
        tags
    } else {
        Vec::new()
    };

    // 1. Query Flathub AppStream API via curl
    let flathub_version = Command::new("curl")
        .args([
            "-sSfL", // Silent, fail fast on HTTP errors, follow redirects
            &format!("https://flathub.org/api/v2/appstream/{}", app_id),
        ])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);

                // Parse JSON and extract releases[0].version
                serde_json::from_str::<serde_json::Value>(&stdout)
                    .ok()
                    .and_then(|data| {
                        data.get("releases")
                            .and_then(|r| r.as_array())
                            .and_then(|arr| arr.first())
                            .and_then(|first_release| first_release.get("version"))
                            .and_then(|v| v.as_str())
                            .map(|v| v.trim().to_string())
                            .filter(|v| !v.is_empty())
                    })
            } else {
                None
            }
        });

    // 2. If Flathub provided a version, cross-reference with git tags to find prefix
    if let Some(version) = flathub_version {
        let prefix = git_tags
            .iter()
            .find_map(|tag| {
                if tag.ends_with(&version) {
                    Some(tag[..tag.len() - version.len()].to_string())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| {
                git_tags
                    .first()
                    .and_then(|latest_tag| {
                        let idx = latest_tag.find(|c: char| c.is_ascii_digit())?;
                        Some(latest_tag[..idx].to_string())
                    })
                    .unwrap_or_default()
            });

        return Some((version, prefix));
    }

    // 3. Fallback: Parse version and prefix strictly from latest git tag
    if let Some(latest_tag) = git_tags.first()
        && let Some(idx) = latest_tag.find(|c: char| c.is_ascii_digit())
    {
        let prefix = latest_tag[..idx].to_string();
        let version = latest_tag[idx..].to_string();
        if !version.is_empty() {
            return Some((version, prefix));
        }
    }

    None
}

pub fn print_info(message: &str) {
    println!("{}", message.yellow());
}

pub fn print_success(message: &str) {
    println!("{}", message.green());
}

pub fn print_error(message: &str) {
    eprintln!("{}", message.red());
}
