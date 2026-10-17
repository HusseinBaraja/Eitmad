#Requires -Version 7.4
[CmdletBinding()]
param(
    [ValidateSet('Start', 'Stop', 'Status')][string]$Action = 'Start',
    [string]$StateDirectory,
    [switch]$SkipBuild
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
if ($SkipBuild -and $Action -ne 'Start') { throw 'SkipBuild is only valid for Start.' }

$options = @{
    Action = $Action
    ServicesOnly = $true
    SkipBuild = $SkipBuild
}
if ($StateDirectory) { $options.StateDirectory = $StateDirectory }
& (Join-Path $PSScriptRoot 'tools/workflow-environment.ps1') @options
