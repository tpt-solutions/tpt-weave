# Changelog

All notable changes to `tpt-weave-core` are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html). While the crate is pre-1.0, minor versions may
contain breaking changes.

## [Unreleased]

## [0.1.0] - Unreleased

### Added

- Repository model: typed identifiers, files, context levels 0-5, requests and responses.
- `.tpt-weave/manifest.toml` configuration with context, feature, privacy and JEv/provider sections.
- Token accounting with baseline versus delivered tracking.
- Stable FNV-1a 64 hashing.
- Secret redaction engine and path-privacy primitives.
- Standard `.tpt-weave` integration layout, repository registry and agent/MCP configuration types.
- `unstable` feature gate for experimental APIs.
