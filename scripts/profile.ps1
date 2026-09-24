<#
.SYNOPSIS
  Measure tpt-weave CPU time and peak working set on Windows.
.DESCRIPTION
  Runs the CLI in a child process and samples its process counters. This
  measures the CLI process only; it does not claim to include compiler or
  operating-system cache overhead. Use a release build for stable results.
#>
param(
  [Parameter(Mandatory = $true)]
  [string]$Path,
  [string[]]$Arguments = @("index", "--full"),
  [string]$Executable = ".\target\release\tpt-weave.exe"
)

$ErrorActionPreference = "Stop"
$exe = (Resolve-Path $Executable).Path
$root = (Resolve-Path $Path).Path
$processArgs = @("--path", $root) + $Arguments
$quotedArgs = $processArgs | ForEach-Object {
  $escaped = [string]$_ -replace '"', '\"'
  '"' + $escaped + '"'
}
$process = Start-Process -FilePath $exe -ArgumentList $quotedArgs -WorkingDirectory $root -PassThru
$peak = 0
$sw = [Diagnostics.Stopwatch]::StartNew()
while (-not $process.HasExited) {
  try {
    $process.Refresh()
    if ($process.PeakWorkingSet64 -gt $peak) { $peak = $process.PeakWorkingSet64 }
  } catch {
    break
  }
  Start-Sleep -Milliseconds 25
}
$process.Refresh()
$cpu = $process.TotalProcessorTime.TotalMilliseconds
$exitCode = $process.ExitCode
$sw.Stop()
[pscustomobject]@{
  executable = $exe
  path = $root
  arguments = $Arguments -join " "
  exit_code = $exitCode
  elapsed_ms = [math]::Round($sw.Elapsed.TotalMilliseconds, 1)
  cpu_ms = [math]::Round($cpu, 1)
  peak_working_set_bytes = $peak
  peak_working_set_mib = [math]::Round($peak / 1MB, 2)
} | ConvertTo-Json -Compress
exit $exitCode
