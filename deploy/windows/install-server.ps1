#Requires -Version 5.1
[CmdletBinding()]
param(
    [ValidateSet('Verify', 'Install', 'Repair', 'Upgrade', 'Start', 'Stop', 'Status', 'Uninstall', 'Firewall')][string]$Action = 'Verify',
    [string]$StateDirectory = "$env:ProgramData\EitmadHost",
    [ValidatePattern('^[A-Za-z][A-Za-z0-9]{0,31}$')][string]$Name = 'Eitmad',
    [string]$ExpectedReleaseSha256, [string]$Publisher, [switch]$AllowUnsignedValidation,
    [string]$LanDirectory, [string]$ClusterDirectory, [string]$PostgresBin,
    [string]$WinSwExecutable, [string]$WinSwSha256,
    [string]$BackupGeneration, [switch]$RestoreVerified,
    [string]$ClientAddress, [switch]$WhatIf
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
Import-Module (Join-Path $PSScriptRoot 'package.psm1') -Force
$arguments = @{Action = $Action; StateDirectory = $StateDirectory; Name = $Name}
if ($Action -in 'Verify', 'Install', 'Repair', 'Upgrade') {
    $release = Read-EitmadPackage -Directory $PSScriptRoot -Target windows-server `
        -ExpectedReleaseSha256 $ExpectedReleaseSha256 -Publisher $Publisher -AllowUnsignedValidation:$AllowUnsignedValidation
    Assert-WindowsPrerequisites
    if ($Action -eq 'Verify') {
        Write-Host "Package verified: server $($release.version); production acceptance is a separate gate."
        return
    }
    $arguments.ReleaseDirectory = $PSScriptRoot
    $arguments.ExpectedReleaseSha256 = $ExpectedReleaseSha256
    $arguments.Publisher = $Publisher
    $arguments.AllowUnsignedValidation = $AllowUnsignedValidation
}
foreach ($key in 'LanDirectory', 'ClusterDirectory', 'PostgresBin', 'WinSwExecutable', 'WinSwSha256',
    'BackupGeneration', 'RestoreVerified', 'ClientAddress', 'WhatIf') {
    if ($PSBoundParameters.ContainsKey($key)) { $arguments[$key] = $PSBoundParameters[$key] }
}
Import-Module (Join-Path $PSScriptRoot 'deploy/windows/host.psm1') -Force
Invoke-EitmadHost @arguments
