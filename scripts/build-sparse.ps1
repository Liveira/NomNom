<#
.SYNOPSIS
  Builds and signs the "sparse" package that puts NOMNOM in the Windows 11 right-click menu.

.DESCRIPTION
  1. Fills in packaging/sparse/AppxManifest.template.xml (the list of file types comes from the
     Rust menu table, so the package and the menu agree).
  2. Makes the three small logo files the manifest names.
  3. Packs everything with makeappx.
  4. Signs the package with the certificate in -PfxPath (see dev-cert.ps1 for a development one).

  The package holds no program. nomnom.exe and nomnom_shell.dll stay in NOMNOM's own folder.
  Windows only accepts the package if the signing certificate is trusted on the PC. For a
  development certificate that is a one-time step that needs administrator rights (dev-cert.ps1).

.EXAMPLE
  scripts\build-sparse.ps1 -PfxPath .\dev\nomnom-dev.pfx -PfxPassword 'dev'
#>
param(
  [string]$Out = "target\sparse",
  [string]$PfxPath,
  [string]$PfxPassword,
  [string]$Publisher = "CN=NOMNOM Dev",
  [string]$Version = "0.1.0"
)
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root
$env:Path += ";$env:USERPROFILE\.cargo\bin"

$sdk = Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\bin" -Directory |
  Where-Object { Test-Path "$($_.FullName)\x64\makeappx.exe" } |
  Sort-Object Name -Descending | Select-Object -First 1
if (-not $sdk) { throw "The Windows SDK (makeappx.exe) isn't installed." }
$makeappx = "$($sdk.FullName)\x64\makeappx.exe"
$signtool = "$($sdk.FullName)\x64\signtool.exe"

$stage = Join-Path $Out "stage"
if (Test-Path $stage) { Remove-Item -Recurse -Force $stage }
New-Item -ItemType Directory -Force -Path "$stage\Assets" | Out-Null

# 1. Manifest, generated from the same table the menu uses.
$manifest = cargo run --quiet -p nomnom-win --example sparse_manifest -- packaging\sparse\AppxManifest.template.xml $Publisher $Version
if ($LASTEXITCODE -ne 0) { throw "Couldn't generate the manifest." }
[IO.File]::WriteAllText("$stage\AppxManifest.xml", ($manifest -join "`n"), (New-Object Text.UTF8Encoding($false)))
[xml]$check = Get-Content "$stage\AppxManifest.xml" -Raw   # throws if the XML is malformed

# 2. Logos, scaled from the app icon.
Add-Type -AssemblyName System.Drawing
$icon = [Drawing.Image]::FromFile((Resolve-Path "apps\desktop\src-tauri\icons\128x128@2x.png"))
foreach ($logo in @(@("StoreLogo.png", 50), @("Square44x44Logo.png", 44), @("Square150x150Logo.png", 150))) {
  $size = $logo[1]
  $bmp = New-Object Drawing.Bitmap $size, $size
  $g = [Drawing.Graphics]::FromImage($bmp)
  $g.InterpolationMode = "HighQualityBicubic"
  $g.DrawImage($icon, 0, 0, $size, $size)
  $g.Dispose()
  $bmp.Save("$stage\Assets\$($logo[0])", [Drawing.Imaging.ImageFormat]::Png)
  $bmp.Dispose()
}
$icon.Dispose()

# 3. Pack. /nv skips the schema check that insists the program is inside the package.
$msix = Join-Path $Out "nomnom-shell.msix"
if (Test-Path $msix) { Remove-Item -Force $msix }
& $makeappx pack /d $stage /p $msix /nv | Out-Host
if ($LASTEXITCODE -ne 0) { throw "makeappx failed." }

# 4. Sign.
if ($PfxPath) {
  & $signtool sign /fd SHA256 /f $PfxPath /p $PfxPassword $msix | Out-Host
  if ($LASTEXITCODE -ne 0) { throw "signtool failed." }
  Write-Host "Signed $msix"
} else {
  Write-Host "Packed but NOT signed: Windows will refuse an unsigned package. Pass -PfxPath to sign."
}
Write-Host "Done: $msix"
