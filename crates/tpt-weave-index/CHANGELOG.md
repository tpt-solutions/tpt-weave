# Changelog

All notable changes to `tpt-weave-index` are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html). While the crate is pre-1.0, minor versions may
contain breaking changes.

## [Unreleased]

## [0.1.0] - Unreleased

### Added

- Cargo workspace indexing from `cargo metadata --no-deps`: packages, targets, dependencies, features.
- Git integration: root discovery, revision, working-tree status, diff, changed files and history lookup.
- Cross-repository dependency discovery for sibling TPT repositories.
- Stable, sorted, serialisable models.
