<#
.SYNOPSIS
  Run the local tpt-weave release preflight gates.
.DESCRIPTION
  This is non-publishing and non-mutating with respect to source files. It runs
  formatting, diff, metadata, compilation, tests, and core-package validation.
  It intentionally does not publish crates, create a Git tag, or contact a
  decision provider.
#>
param(
  [switch]$SkipTests
)

$ErrorActionPreference = "Stop"

function Invoke-Gate {
  param([string]$Name, [scriptblock]$Command)
  Write-Host "==> $Name"
  & $Command
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

Invoke-Gate "format" { cargo fmt --all -- --check }
Invoke-Gate "diff" { git diff --check }
Invoke-Gate "metadata" { cargo metadata --locked --no-deps --format-version 1 | Out-Null }
Invoke-Gate "check" { cargo check --workspace --locked }
Invoke-Gate "clippy" { cargo clippy --workspace --all-targets --locked -- -D warnings }
if (-not $SkipTests) {
  Invoke-Gate "tests" { cargo test --workspace --locked }
}
Invoke-Gate "core-package" { cargo package -p tpt-weave-core --allow-dirty --locked }

Write-Host "Local release preflight passed. External evidence/publication gates remain open; see docs/release-readiness.md."
