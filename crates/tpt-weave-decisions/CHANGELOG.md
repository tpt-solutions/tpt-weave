# Changelog

All notable changes to `tpt-weave-decisions` are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html). While the crate is pre-1.0, minor versions may
contain breaking changes.

## [Unreleased]

## [0.1.0] - Unreleased

### Added

- `DecisionProvider` trait with mock and deterministic fallback providers.
- Eight JEv decision categories.
- Policy engine with confidence threshold, safety overrides and low-confidence fallback.
- Never-failing decision engine with evaluation log.
