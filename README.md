# flatpak2spec

`flatpak2spec` is a fast, robust Rust-based CLI tool designed to inspect a Flatpak application repository (or remote URL) and automatically generate a complete, production-ready RPM `.spec` file.

> [!WARNING]
> **Prerelease Notice:** `flatpak2spec` is currently under development and is in a prerelease state. It **does not yet generate complete RPM spec files**—Files and changelog generation is coming soon! Supported only for `meson` buildsystem for now.

## Features

- 🚀 **Remote & Local Repositories:** Supports direct cloning and workspace parsing from GitHub, GitLab, Codeberg, or local directories.
- 📦 **Manifest & Build Parsing:** Inspects Flatpak JSON/YAML manifests and Meson project configurations (`meson.build`).
- 🏷️ **Smart Version & Prefix Detection:** Queries remote tags to determine the latest semantic release version and dynamically handles tag prefixes for correct `Source0` URL generation.
- 🏗️ **Platform-Specific Archives:** Automatically formats archive download links for GitHub, GitLab, and Codeberg/Gitea.
- 🎯 **Noarch Detection:** Automatically detects header parameters like `BuildArch: noarch` when applicable.

## Installation

Ensure you have Rust and Cargo installed, then build from source:

```bash
git clone https://github.com/Infiniti151/flatpak2spec.git
cd flatpak2spec
cargo build --release
```

The compiled binary will be available at `target/release/flatpak2spec`.

## Usage

```
flatpak2spec [OPTIONS] --repo <PATH_OR_URL>

Options:
  -r, --repo <PATH_OR_URL>   Path or URL to the Flatpak application repository
  -b, --bug-url <URL>        Bug tracker URL for the BugURL spec field (optional)
  -p, --packager <PACKAGER>  Packager name for the %changelog section (optional)
  -e, --email <EMAIL>        Packager email for the %changelog section (optional)
  -o, --output <FILE>        Optional output file path (defaults to stdout if omitted)
  -v, --verbose...           Enable verbose logging output
  -h, --help                 Print help
  -V, --version              Print version
  ```

## Generated Spec File

Command:

`flatpak2spec -r https://codeberg.org/ckruse/Gitte -b https://github.com/Infiniti151/flatpak-apps`

Output:
```
%global         app_id de.wwwtech.gitte

Name:           gitte
Version:        0.9.1
Release:        1%{?dist}
Summary:        Manage your code history
License:        AGPL-3.0-or-later
URL:            https://codeberg.org/ckruse/Gitte
BugURL:         https://github.com/Infiniti151/flatpak-apps

Source0:        %{url}/archive/%{version}.tar.gz

BuildRequires:  meson >= 1.1.0
BuildRequires:  ninja-build
BuildRequires:  cargo
BuildRequires:  cargo-rpm-macros
BuildRequires:  desktop-file-utils
BuildRequires:  gettext
BuildRequires:  glibc-langpack-en
BuildRequires:  gtk-update-icon-cache
BuildRequires:  libappstream-glib
BuildRequires:  rustc

%description
Gitte is built around putting commits together with care. Stage or discard your changes line by line, in larger blocks, or a whole file at a time, so every commit holds exactly the work you mean it to and nothing more. It opens straight into your current changes, ready to work.

Beyond that, follow how your project's history branches and merges in a visual graph, organize branches and tags, set work aside for later, and sync with the places your code is stored, all in a clean, native interface.

%prep
%autosetup -C -p1

%build
%meson
%meson_build

%install
%meson_install
%find_lang %{name}

%check
%meson_test
desktop-file-validate %{buildroot}%{_datadir}/applications/*.desktop
appstream-util validate-relax --nonet %{buildroot}%{_metainfodir}/*.metainfo.xml
glib-compile-schemas --dry-run --strict %{buildroot}%{_datadir}/glib-2.0/schemas/
```

## Project Structure

```
flatpak2spec/
├── Cargo.toml                      # Project metadata, binary definition, and crate dependencies
├── .github/
│   └── workflows/
│       └── release.yml             # CI/CD workflow for automated building, attestation, and releases
└── src/
    ├── main.rs                     # Entry point: orchestrates CLI args, workspace cloning, and pipeline execution
    ├── spec.rs                     # Assembles all generated section strings into a final RPM .spec file
    ├── repository.rs               # Workspace preparation and remote Git repository cloning logic
    ├── forge.rs                    # Detects Git hosts (GitHub, GitLab, Codeberg) to format Source0 URLs & %autosetup flags
    ├── manifest.rs                 # Flatpak manifest parser and validation engine
    ├── meson.rs                    # Meson build file inspection and metadata extraction
    ├── utils.rs                    # Common filesystem, regex search, and string helper utilities
    └── sections/                   # Spec file section generators
        ├── mod.rs                  # Module aggregator exporting all spec section submodules
        ├── header.rs               # Generates RPM spec preamble metadata (Name, Version, License, URL, Source0)
        ├── deps.rs                 # Translates Flatpak modules/dependencies into BuildRequires and Requires directives
        ├── description.rs          # Generates the %description section
        ├── build.rs                # Generates %prep, %build, %install, and %check sections
        ├── scriptlets.rs           # Generates post/un scriptlets (icon cache updates, glib schemas)
        ├── files.rs                # Scans project artifacts to generate %files listings and %find_lang macros
        └── changelog.rs            # Generates the %changelog section
```

## Contributing

Contributions, bug reports, and pull requests are welcome! Feel free to check out the issues page.

## License

This project is licensed under GPL-3.0-or-later.
