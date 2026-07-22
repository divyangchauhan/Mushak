<#
.SYNOPSIS
  Build an MSIX package for the Microsoft Store build of Mushak.

.DESCRIPTION
  Stages the release exe, the logo assets, and AppxManifest.xml (with the
  Version substituted), then runs makeappx to produce a .msix.

  Before this works end-to-end you must replace the REPLACE_ME_* identity
  placeholders in AppxManifest.xml with your Partner Center values (see
  docs/ms-store-submission.md). The script warns if they are still present.

  The Store re-signs the package on submission, so signing here is only for
  installing/testing on your own machine. Use -Sign for that (needs a
  self-signed cert whose subject matches the manifest Publisher).

.EXAMPLE
  # Structural pack (version taken from Cargo.toml):
  pwsh packaging/msix/build-msix.ps1

.EXAMPLE
  # Pack and sign for a local test install:
  pwsh packaging/msix/build-msix.ps1 -Sign -CertSubject "CN=<your-publisher-id>"
#>
[CmdletBinding()]
param(
    [string]$Version,
    [switch]$Sign,
    [string]$CertSubject
)

$ErrorActionPreference = 'Stop'
$msixDir = $PSScriptRoot
$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path

# --- version: default from Cargo.toml, coerce to the Store's x.x.x.0 shape ----
if (-not $Version) {
    $line = Select-String -Path (Join-Path $root 'Cargo.toml') -Pattern '^\s*version\s*=\s*"([^"]+)"' | Select-Object -First 1
    if (-not $line) { throw "Could not read version from Cargo.toml" }
    $Version = $line.Matches[0].Groups[1].Value
}
$parts = $Version.Split('.')
while ($parts.Count -lt 3) { $parts += '0' }
$msixVersion = "{0}.{1}.{2}.0" -f $parts[0], $parts[1], $parts[2]
Write-Host "MSIX version: $msixVersion"

# --- ensure the release exe exists --------------------------------------------
$exe = Join-Path $root 'target\release\mushak.exe'
if (-not (Test-Path $exe)) {
    Write-Host "Building release binary..."
    Push-Location $root
    try { cargo build --release; if ($LASTEXITCODE -ne 0) { throw "cargo build failed" } }
    finally { Pop-Location }
}

# --- find makeappx (latest Windows SDK) ---------------------------------------
$makeappx = Get-ChildItem "C:\Program Files (x86)\Windows Kits\10\bin\*\x64\makeappx.exe" -ErrorAction SilentlyContinue |
    Sort-Object FullName -Descending | Select-Object -First 1
if (-not $makeappx) { throw "makeappx.exe not found. Install the Windows 10/11 SDK." }

# --- stage --------------------------------------------------------------------
$stage = Join-Path $root 'target\msix\stage'
if (Test-Path $stage) { Remove-Item -Recurse -Force $stage }
New-Item -ItemType Directory -Force $stage | Out-Null
New-Item -ItemType Directory -Force (Join-Path $stage 'Assets') | Out-Null

$manifest = Get-Content (Join-Path $msixDir 'AppxManifest.xml') -Raw
if ($manifest -match 'REPLACEME') {
    Write-Warning "AppxManifest.xml still has REPLACEME identity placeholders. The package will pack but the Store will reject it, and signing/installing locally will not match. Fill them in from Partner Center first."
}
$manifest = $manifest -replace 'Version="0\.0\.0\.0"', ("Version=""{0}""" -f $msixVersion)
Set-Content -Path (Join-Path $stage 'AppxManifest.xml') -Value $manifest -Encoding UTF8

Copy-Item $exe (Join-Path $stage 'mushak.exe')
Copy-Item (Join-Path $msixDir 'Assets\*') (Join-Path $stage 'Assets') -Recurse

# --- pack ---------------------------------------------------------------------
$outDir = Join-Path $root 'target\msix'
$out = Join-Path $outDir ("mushak-{0}-x64.msix" -f $Version)
& $makeappx.FullName pack /d $stage /p $out /o
if ($LASTEXITCODE -ne 0) { throw "makeappx failed" }
Write-Host "Packed: $out"

# --- optional local-test signing ----------------------------------------------
if ($Sign) {
    if (-not $CertSubject) { throw "-Sign requires -CertSubject 'CN=<publisher-id>' matching the manifest Publisher." }
    $signtool = Get-ChildItem "C:\Program Files (x86)\Windows Kits\10\bin\*\x64\signtool.exe" -ErrorAction SilentlyContinue |
        Sort-Object FullName -Descending | Select-Object -First 1
    if (-not $signtool) { throw "signtool.exe not found. Install the Windows 10/11 SDK." }
    & $signtool.FullName sign /fd SHA256 /a /n ($CertSubject -replace '^CN=','') $out
    if ($LASTEXITCODE -ne 0) { throw "signtool failed" }
    Write-Host "Signed (local test): $out"
}
