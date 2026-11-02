#Requires -Version 5.1
[CmdletBinding()]
param(
    [ValidateSet('Verify', 'Install', 'Repair', 'Upgrade', 'Status', 'Diagnose', 'Uninstall')][string]$Action = 'Verify',
    [string]$ExpectedReleaseSha256, [string]$Publisher,
    [switch]$AllowUnsignedValidation
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
Import-Module (Join-Path $PSScriptRoot 'package.psm1') -Force
Assert-WindowsPrerequisites -PlatformOnly
$installed = @(Get-AppxPackage -Name Eitmad.Desktop)
if ($installed.Count -gt 1) { throw 'More than one desktop identity is installed. Inspect the publisher before proceeding.' }
if ($Action -eq 'Status') {
    $installed | Select-Object Name, Version, Publisher, Status
    Write-Host 'Persistent engine storage: %LOCALAPPDATA%\Eitmad\engine; do not copy it to another PC.'
    return
}
if (Get-Process -Name Eitmad.WindowsShell, eitmad-engine-cli -ErrorAction SilentlyContinue) {
    throw 'Close the desktop and wait for its engine to stop. No process was terminated.'
}
if ($Action -eq 'Uninstall') {
    if ($installed) { Remove-AppxPackage -Package $installed[0].PackageFullName -ErrorAction Stop }
    Write-Host 'Desktop removed for this user. Engine data and Windows credentials are retained. Do not use Windows Reset.'
    return
}
if ($Action -eq 'Diagnose') {
    if (-not $installed) { throw 'No desktop package is installed for this Windows user.' }
    & (Join-Path $installed[0].InstallLocation 'eitmad-engine-cli.exe') diagnose
    if ($LASTEXITCODE -ne 0) { throw 'Engine diagnostics found an unhealthy or locked installation. Preserve data and use the reported error identifiers.' }
    return
}
$release = Read-EitmadPackage -Directory $PSScriptRoot -Target windows-desktop `
    -ExpectedReleaseSha256 $ExpectedReleaseSha256 -Publisher $Publisher -AllowUnsignedValidation:$AllowUnsignedValidation
$package = @(Get-ChildItem -LiteralPath $PSScriptRoot -Filter '*.msix' -File)
if ($package.Count -ne 1) { throw 'Expected exactly one desktop MSIX.' }
Assert-DesktopRuntime $package[0].FullName $release.payloadInventorySha256
$selected = Get-DesktopInstallAction $release $installed
if ($Action -eq 'Verify') {
    Write-Host "Package inventory verified: desktop $($release.version). Unsigned MSIX cannot be installed by this tool."
    return
}
# There is deliberately no Add-AppxPackage -AllowUnsigned or certificate-import path.
Assert-PackageSignature $package[0].FullName $Publisher
if ($Action -eq 'Install') { $Action = $selected }
elseif ($Action -cne $selected) { throw "The package requires $selected, not $Action. No registration was changed." }
if ($Action -eq 'Repair') { Add-AppxPackage -Path $package[0].FullName -ForceUpdateFromAnyVersion -ErrorAction Stop }
else { Add-AppxPackage -Path $package[0].FullName -ErrorAction Stop }
$current = @(Get-AppxPackage -Name Eitmad.Desktop)
if ($current.Count -ne 1 -or $current[0].Version -ne [version]$release.windowsVersion -or
    $current[0].Publisher -cne $release.publisher -or $current[0].Status -ne 'Ok') {
    throw 'Windows package registration did not match the release. Preserve data and inspect AppXDeploymentServer events.'
}
Write-Host 'Desktop registered for this Windows user. Open the app from Start, enroll with your own account, and verify connected work.'
