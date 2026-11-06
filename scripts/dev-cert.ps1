<#
.SYNOPSIS
  Makes a self-signed certificate for trying out the Windows 11 right-click menu on your own PC.

.DESCRIPTION
  Creates "CN=NOMNOM Dev" in your certificate store, saves it to a .pfx file, and (with -Trust)
  tells Windows to trust it for installing packages.

  Trusting needs administrator rights, so this script does NOT do it unless you pass -Trust from
  an elevated PowerShell. It is only for development. A released NOMNOM is signed with a real
  certificate instead.

  To undo: delete the certificate in certmgr.msc (Personal and Trusted People), or run with -Remove.

.EXAMPLE
  scripts\dev-cert.ps1                 # make the certificate and .pfx (no admin needed)
  scripts\dev-cert.ps1 -Trust          # also trust it (run PowerShell as administrator)
  scripts\dev-cert.ps1 -Remove         # delete it again
#>
param(
  [string]$Out = "dev",
  [string]$Password = "nomnom-dev",
  [switch]$Trust,
  [switch]$Remove
)
$ErrorActionPreference = "Stop"
$subject = "CN=NOMNOM Dev"

if ($Remove) {
  Get-ChildItem Cert:\CurrentUser\My | Where-Object Subject -eq $subject | Remove-Item
  try {
    Get-ChildItem Cert:\LocalMachine\TrustedPeople | Where-Object Subject -eq $subject | Remove-Item
  } catch { Write-Host "Couldn't remove the trusted copy (run as administrator)." }
  if (Test-Path $Out) { Remove-Item -Recurse -Force $Out }
  Write-Host "Removed."
  return
}

New-Item -ItemType Directory -Force -Path $Out | Out-Null
$cert = Get-ChildItem Cert:\CurrentUser\My | Where-Object Subject -eq $subject | Select-Object -First 1
if (-not $cert) {
  $cert = New-SelfSignedCertificate -Type Custom -Subject $subject `
    -KeyUsage DigitalSignature -FriendlyName "NOMNOM development" `
    -CertStoreLocation "Cert:\CurrentUser\My" `
    -TextExtension @("2.5.29.37={text}1.3.6.1.5.5.7.3.3", "2.5.29.19={text}")
}
$secure = ConvertTo-SecureString -String $Password -Force -AsPlainText
Export-PfxCertificate -Cert $cert -FilePath "$Out\nomnom-dev.pfx" -Password $secure | Out-Null
Export-Certificate -Cert $cert -FilePath "$Out\nomnom-dev.cer" | Out-Null
Write-Host "Certificate saved to $Out\nomnom-dev.pfx (password: $Password)"

if ($Trust) {
  $admin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
  if (-not $admin) { throw "Trusting the certificate needs an administrator PowerShell." }
  Import-Certificate -FilePath "$Out\nomnom-dev.cer" -CertStoreLocation Cert:\LocalMachine\TrustedPeople | Out-Null
  Write-Host "Trusted. NOMNOM can now add its right-click menu."
} else {
  Write-Host "Not trusted yet. For the Windows 11 menu, run again with -Trust from an administrator PowerShell."
}
