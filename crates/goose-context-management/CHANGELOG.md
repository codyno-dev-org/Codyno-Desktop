# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0-alpha.9](https://github.com/codyno-dev-org/Codyno-Desktop/compare/gdk-v0.1.0-alpha.8...gdk-v0.1.0-alpha.9) - 2026-09-19

### Added

- *(gdk)* add first-class document content support ([#11629](https://github.com/codyno-dev-org/Codyno-Desktop/pull/11629))
- compaction in the GDK ([#11042](https://github.com/codyno-dev-org/Codyno-Desktop/pull/11042))

### Fixed

- unify context limit resolution behind provider API ([#11213](https://github.com/codyno-dev-org/Codyno-Desktop/pull/11213))
- honest compaction failure message and fast-fail when no tool responses exist ([#10500](https://github.com/codyno-dev-org/Codyno-Desktop/pull/10500))
- *(agents)* fail fast when a recipe's structured response can't reach an ACP-bridged provider ([#11307](https://github.com/codyno-dev-org/Codyno-Desktop/pull/11307))

### Other

- goose-sdk version bump alpha 8 ([#11815](https://github.com/codyno-dev-org/Codyno-Desktop/pull/11815))
- publish GDK packages from version tags ([#11595](https://github.com/codyno-dev-org/Codyno-Desktop/pull/11595))
- Clean up obsolete extension paths before redesigning the extension manager ([#11645](https://github.com/codyno-dev-org/Codyno-Desktop/pull/11645))
- add SDK API reference for Rust, Python, and Kotlin ([#11251](https://github.com/codyno-dev-org/Codyno-Desktop/pull/11251))
- create the goose-agent crate with the unrolled agent loop state machine ([#11216](https://github.com/codyno-dev-org/Codyno-Desktop/pull/11216))
