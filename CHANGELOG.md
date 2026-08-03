# Changelog

## [0.3.0] - 2026-08-03

### 🚀 Features
- (**files**) Expand doc scanning targets to include AUTHORS, CONTRIBUTORS, and TODO
- (**manifest**) Resolve external `Module::Path` and `Source::Path` files dynamically

### 🐛 Bug Fixes
- (**manifest**) Sanitize app_id and resolve external path references
- (**manifest**) Strip `.devel` and `.Devel` suffixes during App ID sanitization
- (**files**) Perform exact stem and extension matching for docs and licenses

### 🎨 Styling & Formatting
- (**manifest**) Collapse nested if blocks to satisfy Clippy lints cleanly

### 📚 Documentation
- (**readme**) Add signature verification instructions
- (**readme**) Fix formatting

### ⚙️ Continuous Integration
- (**ci**) Use git-cliff for automated changelog generation
- (**ci**) Add SSH asset signing for release packages
- (**deps**) Bump the github-actions-dependencies group with 2 updates

### 🧹 Chores
- Update spec file formatting and commit step
- Update workflow for better version handling

## [0.2.0] - 2026-08-01

### 🚀 Features
- Support Git submodule resolution and tracking

### 💡 Other Changes
- Improve forge integration via `forge-srpm-macros`
- Enhance Python environment detection

## [0.1.0] - 2026-07-31

### 🚀 Features
- Support remote and local repository parsing
- Parse Flatpak manifests, AppStream metadata, and Meson configurations
- Extract project changelogs to generate RPM `%changelog` entries
- Detect release versions and format forge download URLs dynamically
- Detect asset-only projects to assign `BuildArch: noarch`
- Generate idiomatic Fedora RPM spec files with standard macros
- Optimize output for Copr, GitHub Actions, and `mock` chroots
- Initial release