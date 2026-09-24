<#
.SYNOPSIS
  Capture a deterministic tpt-weave workflow as Phase 18 metadata.
.DESCRIPTION
  Runs overview, symbol, context, skeleton, and repeated diff commands. Only
  token counts, latencies, and stable event IDs are written; source text,
  prompts, credentials, and model output are never retained.
#>
param(
  [Parameter(Mandatory = $true)]
  [string]$Path,
  [string]$Executable = ".\target\release\tpt-weave.exe",
  [string]$OutputDirectory = ".tpt-weave\workload",
  [string]$SessionId = "local-workload-session",
  [double]$DurationSeconds = 0
)

$ErrorActionPreference = "Stop"
$root = (Resolve-Path $Path).Path
$exe = (Resolve-Path $Executable).Path
$outputDir = if ([IO.Path]::IsPathRooted($OutputDirectory)) {
  $OutputDirectory
} else {
  Join-Path $root $OutputDirectory
}
New-Item -ItemType Directory -Force -Path $outputDir | Out-Null
$events = [Collections.Generic.List[object]]::new()
$totalWatch = [Diagnostics.Stopwatch]::StartNew()

function Estimate-Tokens([string]$Text) {
  if ([string]::IsNullOrEmpty($Text)) { return [uint64]0 }
  return [uint64][math]::Ceiling($Text.Length / 4.0)
}

function Invoke-MetadataCommand([string[]]$CommandArgs) {
  $watch = [Diagnostics.Stopwatch]::StartNew()
  $allArgs = @("--path", $root, "--json") + $CommandArgs
  $output = & $exe @allArgs 2>&1
  $exitCode = $LASTEXITCODE
  $watch.Stop()
  if ($exitCode -ne 0) {
    throw "tpt-weave $($CommandArgs -join ' ') failed with exit ${exitCode}: $($output -join [Environment]::NewLine)"
  }
  $json = ($output -join [Environment]::NewLine) | ConvertFrom-Json
  [pscustomobject]@{ Data = $json; ElapsedMs = [uint64][math]::Round($watch.Elapsed.TotalMilliseconds) }
}

function Add-Event(
  [string]$Kind,
  [string]$Id,
  [uint64]$RawTokens,
  [uint64]$DeliveredTokens,
  [uint64]$LatencyMs,
  [uint64]$JevTokens = 0,
  [uint64]$OutputTokens = 0,
  [bool]$CacheHit = $false
) {
  $events.Add([ordered]@{
    kind = $Kind
    id = $Id
    raw_tokens = $RawTokens
    delivered_tokens = $DeliveredTokens
    jev_tokens = $JevTokens
    output_tokens = $OutputTokens
    latency_ms = $LatencyMs
    cache_hit = $CacheHit
  }) | Out-Null
}

$overview = Invoke-MetadataCommand @("overview")
$overviewTokens = [uint64]$overview.Data.token_estimate
Add-Event "repository_exploration" "overview" $overviewTokens $overviewTokens $overview.ElapsedMs

$symbol = Invoke-MetadataCommand @("symbol", "WorkloadCapture")
$symbolText = $symbol.Data | ConvertTo-Json -Depth 20 -Compress
$symbolTokens = Estimate-Tokens $symbolText
Add-Event "context" "symbol:WorkloadCapture" $symbolTokens $symbolTokens $symbol.ElapsedMs

$context = Invoke-MetadataCommand @("context", "review workload capture and phase 18 reporting")
$contextTokens = $context.Data.tokens
Add-Event "context" "context:workload-review" ([uint64]$contextTokens.raw_tokens) ([uint64]$contextTokens.selected_tokens) $context.ElapsedMs ([uint64]$contextTokens.jev_tokens) 0 ([bool]$contextTokens.cache_hit)
$contextRepeat = Invoke-MetadataCommand @("context", "review workload capture and phase 18 reporting")
$contextRepeatTokens = $contextRepeat.Data.tokens
Add-Event "context" "context:workload-review" ([uint64]$contextRepeatTokens.raw_tokens) ([uint64]$contextRepeatTokens.selected_tokens) $contextRepeat.ElapsedMs ([uint64]$contextRepeatTokens.jev_tokens) 0 ([bool]$contextRepeatTokens.cache_hit)

$skeletonPath = "crates/tpt-weave-eval/src/workload.rs"
$skeleton = Invoke-MetadataCommand @("skeleton", $skeletonPath, "--level", "skeleton")
$sourceText = Get-Content (Join-Path $root $skeletonPath) -Raw
$sourceTokens = Estimate-Tokens $sourceText
Add-Event "context" "skeleton:workload.rs" $sourceTokens ([uint64]$skeleton.Data.token_estimate) $skeleton.ElapsedMs

$diff = Invoke-MetadataCommand @("diff")
Add-Event "tool_output" "git-diff" ([uint64]$diff.Data.raw_tokens) ([uint64]$diff.Data.reduced_tokens) $diff.ElapsedMs
$diffRepeat = Invoke-MetadataCommand @("diff")
Add-Event "tool_output" "git-diff" ([uint64]$diffRepeat.Data.raw_tokens) ([uint64]$diffRepeat.Data.reduced_tokens) $diffRepeat.ElapsedMs

$totalWatch.Stop()
$duration = if ($DurationSeconds -gt 0) { $DurationSeconds } else { $totalWatch.Elapsed.TotalSeconds }
$capturePath = Join-Path $outputDir "$SessionId.jsonl"
$jsonLines = @($events | ForEach-Object { $_ | ConvertTo-Json -Compress })
$utf8 = New-Object System.Text.UTF8Encoding($false)
[IO.File]::WriteAllLines($capturePath, [string[]]$jsonLines, $utf8)

$reportPath = Join-Path $outputDir "$SessionId-report.json"
$reportArgs = @("--path", $root, "--json", "workload", $capturePath, "--duration", $duration.ToString([Globalization.CultureInfo]::InvariantCulture))
$report = & $exe @reportArgs 2>&1
if ($LASTEXITCODE -ne 0) { throw "workload report failed: $($report -join [Environment]::NewLine)" }
[IO.File]::WriteAllText($reportPath, ($report -join [Environment]::NewLine), $utf8)

[pscustomobject]@{
  capture = $capturePath
  report = $reportPath
  events = $events.Count
  duration_seconds = [math]::Round($duration, 3)
} | ConvertTo-Json -Compress
