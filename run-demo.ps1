#Requires -Version 7.4
[CmdletBinding()]
param(
    [ValidateSet('Prepare', 'Start', 'Stop', 'Status')][string]$Action = 'Start',
    [string]$PostgresBin,
    [switch]$NoDesktop,
    [switch]$PrepareCustomization
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# One fixed, isolated connected setup. Never use the developer account seeder.
$options = @{
    Action = $Action
    StateDirectory = Join-Path $PSScriptRoot 'tests/engine-integration/artifacts/demo-family-purchase'
    DatabasePort = 55459
    ServerPort = 58463
}
if ($PostgresBin) { $options.PostgresBin = $PostgresBin }
if ($NoDesktop -and $Action -ne 'Start') { throw 'NoDesktop is only valid for Start.' }
if ($PrepareCustomization -and $Action -ne 'Start') { throw 'PrepareCustomization is only valid for Start.' }
if ($Action -eq 'Start') {
    $options.SkipBuild = $true
    $options.VerifyDemo = $true
    $options.NoDesktop = $NoDesktop
    $options.PrepareCustomization = $PrepareCustomization
}
& (Join-Path $PSScriptRoot 'tools/workflow-environment.ps1') @options
