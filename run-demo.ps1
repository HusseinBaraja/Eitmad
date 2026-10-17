#Requires -Version 7.4
[CmdletBinding()]
param(
    [ValidateSet('Prepare', 'Start', 'Stop', 'Status')][string]$Action = 'Start',
    [ValidateSet('Presentation', 'Rehearsal')][string]$Environment = 'Presentation',
    [string]$PostgresBin,
    [string]$DotNetOutputDirectory,
    [string]$EnginePath,
    [switch]$NoDesktop,
    [switch]$PrepareCustomization
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# Separate retained stories. Never use the developer account seeder or reset state.
$rehearsal = $Environment -eq 'Rehearsal'
$options = @{
    Action = $Action
    StateDirectory = Join-Path $PSScriptRoot $(if ($rehearsal) {
        'tests/engine-integration/artifacts/demo-family-rehearsal'
    } else { 'tests/engine-integration/artifacts/demo-family-purchase' })
    DatabasePort = $(if ($rehearsal) { 55475 } else { 55459 })
    ServerPort = $(if ($rehearsal) { 58479 } else { 58463 })
}
if ($PostgresBin) { $options.PostgresBin = $PostgresBin }
if ($DotNetOutputDirectory) { $options.DotNetOutputDirectory = $DotNetOutputDirectory }
if ($EnginePath) { $options.EnginePath = $EnginePath }
if ($NoDesktop -and $Action -ne 'Start') { throw 'NoDesktop is only valid for Start.' }
if ($PrepareCustomization -and $Action -ne 'Start') { throw 'PrepareCustomization is only valid for Start.' }
if ($Action -eq 'Start') {
    $options.SkipBuild = $true
    $options.VerifyDemo = $true
    $options.NoDesktop = $NoDesktop
    $options.PrepareCustomization = $PrepareCustomization
}
Write-Host "Demo environment: $Environment"
& (Join-Path $PSScriptRoot 'tools/workflow-environment.ps1') @options
