<#
.SYNOPSIS
  Builds everything a NOMNOM release contains: the app, the helper programs, the Windows 11 menu
  package, the Firefox extension and the installer.

.DESCRIPTION
  Output:
    target\release\bundle\nsis\*-setup.exe   the installer (per user, no administrator needed)
    target\release\nomnom.exe                the app with its window embedded
    target\sparse\nomnom-shell.msix          the Windows 11 menu package (signed only with -PfxPath)
    dist\extension\*.zip                     the Firefox extension, unsigned

  The installer is not code-signed here. Windows SmartScreen warns about unsigned installers.

.EXAMPLE
  scripts\build-release.ps1
  scripts\build-release.ps1 -PfxPath C:\keys\nomnom.pfx -PfxPassword (Read-Host)
  scripts\build-release.ps1 -SkipInstaller      # just the exe, much faster
#>
param(
  [string]$PfxPath,
  [string]$PfxPassword,
  [string]$Publisher = "CN=NOMNOM Dev",
  [switch]$SkipInstaller
)
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root
$env:Path += ";$env:USERPROFILE\.cargo\bin"
$version = (Get-Content package.json -Raw | ConvertFrom-Json).version

function Step($text) { Write-Host "`n== $text" -ForegroundColor Cyan }
function Run($what, [scriptblock]$block) {
  & $block
  if ($LASTEXITCODE -ne 0) { throw "$what failed (exit code $LASTEXITCODE)." }
}

Step "Firefox extension"
Run "extension build" { npm run package -w "@nomnom/firefox-extension" }

Step "Helper programs"
Run "cargo build" { cargo build --release -p nomnom-native-host -p nomnom-shell }

Step "Windows 11 menu package"
$sparseArgs = @{ Out = "target\sparse"; Publisher = $Publisher; Version = $version }
if ($PfxPath) { $sparseArgs.PfxPath = $PfxPath; $sparseArgs.PfxPassword = $PfxPassword }
& "$PSScriptRoot\build-sparse.ps1" @sparseArgs

Step "App"
Push-Location apps\desktop
try {
  if ($SkipInstaller) {
    Run "app build" { npx tauri build --no-bundle }
  } else {
    Run "installer build" { npx tauri build --config src-tauri/tauri.release.conf.json }
  }
} finally { Pop-Location }

Step "Done"
Get-ChildItem target\release\nomnom.exe, target\release\nomnom-native-host.exe, target\release\nomnom_shell.dll,
  target\sparse\nomnom-shell.msix -ErrorAction SilentlyContinue |
  ForEach-Object { "{0,-34} {1,8:N1} MB" -f $_.Name, ($_.Length / 1MB) }
Get-ChildItem target\release\bundle\nsis\*.exe -ErrorAction SilentlyContinue |
  ForEach-Object { "{0,-34} {1,8:N1} MB  (installer)" -f $_.Name, ($_.Length / 1MB) }
Get-ChildItem dist\extension\*.zip -ErrorAction SilentlyContinue |
  ForEach-Object { "{0,-34} {1,8:N1} MB  (Firefox extension)" -f $_.Name, ($_.Length / 1MB) }
