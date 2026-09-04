# Changelog

## [0.4.0] - 2026-09-04

### ⚙️ Continuous Integration
- (**changelog**) Update git-cliff configuration for scope grouping
- (**dependabot**) Add dependabot for cargo
- (**deps**) Change dependabot commit prefix
- (**deps**) Bump dtolnay/rust-toolchain
- (**release**) Fix spec file changelog formatting
- (**release**) Use ssh-asset-signer action to sign tarballs instead of expect implementation
- (**release**) Remove orphan step

### 🐛 Bug Fixes
- (**files**) Make datadir dependency check more robust
- (**manifest**) Correctly resolve app_id to avoid double-suffixed metainfo names
- (**release**) Use the correct git-cliff action
- (**spec**) Use correct detected tag prefix in %global tag
- (**spec**) Use glob patterns in %files section for artifacts
- (**version**) Fetch latest released version from Flathub AppStream API
- (**workspace**) Checkout repository to the latest fetched release tag

### 💡 Other Changes
- (**deps**) Bump clap from 4.6.4 to 4.6.5 in the app-dependencies group
- (**deps**) Bump dtolnay/rust-toolchain
- (**deps**) Bump clap from 4.6.5 to 4.6.6 in the app-dependencies group

### 📚 Documentation
- (**changelog**) Update the full changelog in git-cliff format
- (**readme**) Add instructions to download tarball and sig
- (**readme**) Add build attestation verification section
- (**repository**) Update doc comments
- (**spec**) Fix changelog entries
- (**spec**) Format %description section correctly

### 🚀 Features
- (**cli**) Automatically create missing target directories for -o flag
- (**deps**) Parse Cargo.toml to detect GTK4, Libadwaita, and GStreamer dependencies
- (**release**) Add artifact caching to optimize build process

### 🛠️ Dependencies
- (**deps**) Bump clap from 4.6.4 to 4.6.5 in the app-dependencies group
- (**deps**) Bump clap from 4.6.5 to 4.6.6 in the app-dependencies group
- (**deps**) Update cargo dependencies
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