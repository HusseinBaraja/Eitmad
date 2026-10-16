#Requires -Version 7.4
[CmdletBinding()]
param(
    [ValidateSet('Prepare', 'Start', 'Stop', 'Status')][string]$Action = 'Start',
    [string]$PostgresBin,
    [switch]$NoDesktop
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# One fixed, isolated connected setup. Never use the developer account seeder.
$options = @{
    Action = $Action
    StateDirectory = Join-Path $PSScriptRoot 'tests/engine-integration/artifacts/demo-presentation'
    DatabasePort = 55449
    ServerPort = 58453
}
if ($PostgresBin) { $options.PostgresBin = $PostgresBin }
if ($NoDesktop -and $Action -ne 'Start') { throw 'NoDesktop is only valid for Start.' }
if ($Action -eq 'Start') {
    $options.SkipBuild = $true
    $options.VerifyDemo = $true
    $options.NoDesktop = $NoDesktop
}
& (Join-Path $PSScriptRoot 'tools/workflow-environment.ps1') @options
