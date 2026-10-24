# Changelog

All notable changes to `tpt-weave-cache` are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html). While the crate is pre-1.0, minor versions may
contain breaking changes.

## [Unreleased]

## [0.1.0] - Unreleased

### Added

- Filesystem cache keyed by revision, schema, query, policy and level.
- Invalidation sweep, clear and statistics.
- Revision fingerprint covering uncommitted edits and manifest changes.
