<#
.SYNOPSIS
  Measures NOMNOM's memory, CPU and start-up time on a release build.

.DESCRIPTION
  Starts the app in the background (tray only), waits, measures it, then opens the window and
  measures again, including the WebView2 processes the window starts. Stops only the processes it
  started. It refuses to run if NOMNOM is already running, because a second launch would just wake
  that copy and the numbers would be someone else's.

  It uses your real NOMNOM data folder (settings and history are read, not changed; a log line and
  integration.json are written, as on any start).

  Numbers are "private working set" in MB, the part of memory only that process uses.

.EXAMPLE
  scripts\bench.ps1
  scripts\bench.ps1 -Exe C:\Program Files\NomNom\nomnom.exe -IdleSeconds 30
#>
param(
  [string]$Exe = (Join-Path (Split-Path -Parent $PSScriptRoot) "target\release\nomnom.exe"),
  [int]$IdleSeconds = 60,
  [int]$CpuSeconds = 30
)
$ErrorActionPreference = "Stop"
if (-not (Test-Path $Exe)) { throw "Build first: scripts\build-release.ps1 -SkipInstaller. Not found: $Exe" }
if (Get-Process nomnom -ErrorAction SilentlyContinue) { throw "NOMNOM is already running. Quit it first (tray icon, Quit)." }

function PrivateMB($ids) {
  $total = 0
  foreach ($id in $ids) {
    $p = Get-CimInstance Win32_PerfFormattedData_PerfProc_Process -Filter "IDProcess = $id" -ErrorAction SilentlyContinue
    if ($p) { $total += $p.WorkingSetPrivate }
  }
  [math]::Round($total / 1MB, 1)
}
function Descendants($rootId) {
  $all = Get-CimInstance Win32_Process
  $found = New-Object System.Collections.Generic.List[int]
  $queue = New-Object System.Collections.Generic.Queue[int]
  $queue.Enqueue($rootId)
  while ($queue.Count) {
    $id = $queue.Dequeue()
    foreach ($c in ($all | Where-Object ParentProcessId -eq $id)) { $found.Add([int]$c.ProcessId); $queue.Enqueue([int]$c.ProcessId) }
  }
  $found
}

$logDir = Join-Path $env:LOCALAPPDATA "NOMNOM\logs"
$result = [ordered]@{}
$started = Get-Date
$app = Start-Process -FilePath $Exe -ArgumentList "--background" -PassThru
try {
  Write-Host "Background mode: waiting $IdleSeconds s..."
  Start-Sleep -Seconds $IdleSeconds
  $result["Background: nomnom.exe private MB"] = PrivateMB @($app.Id)
  $result["Background: WebView2 processes"] = @(Descendants $app.Id).Count

  $cpu0 = (Get-Process -Id $app.Id).TotalProcessorTime.TotalSeconds
  Start-Sleep -Seconds $CpuSeconds
  $cpu1 = (Get-Process -Id $app.Id).TotalProcessorTime.TotalSeconds
  $result["Idle CPU over $CpuSeconds s (% of one core)"] = [math]::Round(($cpu1 - $cpu0) / $CpuSeconds * 100, 3)

  Write-Host "Opening the window..."
  $opened = Get-Date
  Start-Process -FilePath $Exe | Out-Null          # second launch: wakes the running copy and opens the window
  Start-Sleep -Seconds 12
  $kids = @(Descendants $app.Id)
  $result["Window open: nomnom.exe private MB"] = PrivateMB @($app.Id)
  $result["Window open: WebView2 private MB (all its processes)"] = PrivateMB $kids
  $result["Window open: WebView2 processes"] = $kids.Count

  $log = Get-ChildItem $logDir -Filter "nomnom.*.log" -ErrorAction SilentlyContinue | Sort-Object LastWriteTime | Select-Object -Last 1
  if ($log) {
    $line = Get-Content $log.FullName | Select-String "main window ready" | Select-Object -Last 1
    if ($line) { $result["Window ready (ms since the process started)"] = ($line.Line -replace '.*since_process_start_ms=', '') }
  }
} finally {
  # Only our own process and what it started.
  $mine = @(Descendants $app.Id) + $app.Id
  foreach ($id in $mine) { Stop-Process -Id $id -Force -ErrorAction SilentlyContinue }
}
$result.GetEnumerator() | ForEach-Object { "{0,-62} {1}" -f $_.Key, $_.Value }
Write-Host "`nTargets: background <= 25 MB and no WebView2, window open nomnom.exe <= 40 MB and WebView2 <= 120 MB, idle CPU < 0.1 %, window ready <= 1500 ms."
