<# Verify startup registration in the built installer, without installing it. #>
[CmdletBinding()]
param([Parameter(Mandatory)][string]$Path)
$ErrorActionPreference = 'Stop'
$installer = New-Object -ComObject WindowsInstaller.Installer
$database = $installer.OpenDatabase((Resolve-Path -LiteralPath $Path).Path, 0)
$view = $database.OpenView('SELECT `Root`, `Key`, `Name`, `Value`, `Component_` FROM `Registry`')
$view.Execute()
$startupComponent = $null
while ($record = $view.Fetch()) {
    if ($record.StringData(2) -eq 'Software\Microsoft\Windows\CurrentVersion\Run' -and
        $record.StringData(3) -eq 'Mushak') {
        if ($record.IntegerData(1) -ne 1 -or $record.StringData(4) -cne '"[INSTALLFOLDER]mushak.exe"') {
            throw 'Startup must use HKCU and the quoted installed executable path.'
        }
        $startupComponent = $record.StringData(5)
    }
}
$view.Close()
if (-not $startupComponent) { throw 'MSI has no Mushak startup registration.' }
$view = $database.OpenView('SELECT `Feature_`, `Component_` FROM `FeatureComponents`')
$view.Execute()
$included = $false
while ($record = $view.Fetch()) {
    if ($record.StringData(1) -eq 'Main' -and $record.StringData(2) -eq $startupComponent) {
        $included = $true
    }
}
$view.Close()
if (-not $included) { throw 'Startup component is not included in the installed feature.' }
Write-Host 'PASS: MSI installs the per-user Mushak startup entry with a quoted executable path.'
