<#
.SYNOPSIS
  Run the Windows App Certification Kit against a Mushak MSIX package.

.DESCRIPTION
  Finds appcert.exe in the installed Windows SDK, resets its test state, runs
  the package tests, and fails unless the report's overall result is PASS.

  Quit Mushak before running this script. The certification kit may need an
  elevated PowerShell session to deploy and inspect the package.

.EXAMPLE
  pwsh packaging/msix/test-msix.ps1

.EXAMPLE
  pwsh packaging/msix/test-msix.ps1 -PackagePath target/msix/mushak-0.0.4-x64.msix
#>
[CmdletBinding()]
param(
    [string]$PackagePath,
    [string]$ReportPath
)

$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path

$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = [Security.Principal.WindowsPrincipal]::new($identity)
$isAdministrator = $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdministrator) {
    throw 'WACK requires an elevated PowerShell session. Run this script as administrator.'
}

if (-not $PackagePath) {
    $package = Get-ChildItem (Join-Path $root 'target\msix\mushak-*-x64.msix') -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1
    if (-not $package) {
        throw 'No MSIX package found. Run packaging/msix/build-msix.ps1 first.'
    }
    $PackagePath = $package.FullName
} else {
    $PackagePath = (Resolve-Path $PackagePath).Path
}

if (-not $ReportPath) {
    $ReportPath = Join-Path $root 'target\msix\wack-report.xml'
} else {
    $ReportPath = [System.IO.Path]::GetFullPath($ReportPath)
}

$appcert = Get-Item 'C:\Program Files (x86)\Windows Kits\10\App Certification Kit\appcert.exe' -ErrorAction SilentlyContinue
if (-not $appcert) {
    throw 'appcert.exe not found. Install the Windows App Certification Kit.'
}

& $appcert.FullName reset
if ($LASTEXITCODE -ne 0) { throw 'Windows App Certification Kit reset failed' }

& $appcert.FullName test -appxpackagepath $PackagePath -reportoutputpath $ReportPath
if ($LASTEXITCODE -ne 0) { throw "Windows App Certification Kit exited with code $LASTEXITCODE" }
if (-not (Test-Path $ReportPath)) { throw "Certification report was not created: $ReportPath" }

[xml]$report = Get-Content $ReportPath -Raw
$overall = $report.REPORT.OVERALL_RESULT
Write-Host "WACK result: $overall"
Write-Host "Report: $ReportPath"
if ($overall -ne 'PASS') {
    throw "Windows App Certification Kit result was $overall"
}
