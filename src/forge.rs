// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Infiniti151

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Forge {
    GitHub,
    Codeberg,
    GitLab,
    Generic,
}

impl Forge {
    /// Detect forge provider from repository URL
    pub fn detect(repo_url: &str) -> Self {
        if repo_url.contains("github.com") {
            Forge::GitHub
        } else if repo_url.contains("codeberg.org") {
            Forge::Codeberg
        } else if repo_url.contains("gitlab.") {
            Forge::GitLab
        } else {
            Forge::Generic
        }
    }

    /// Formats the Source0 URL pattern according to the forge provider conventions
    pub fn format_source_url(&self, clean_url: &str, tag_prefix: &str) -> String {
        if !clean_url.starts_with("http://") && !clean_url.starts_with("https://") {
            return "%{name}-%{version}.tar.gz".to_string();
        }

        let tag_var = if tag_prefix.is_empty() {
            "%{version}".to_string()
        } else {
            format!("{}%{{version}}", tag_prefix)
        };

        match self {
            Forge::GitLab => format!("%{{url}}/-/archive/{}.tar.gz", tag_var),
            Forge::GitHub | Forge::Codeberg | Forge::Generic => {
                format!("%{{url}}/archive/{}.tar.gz", tag_var)
            }
        }
    }
}
