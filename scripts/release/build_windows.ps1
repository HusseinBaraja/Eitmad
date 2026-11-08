#Requires -Version 7.4
[CmdletBinding()]
param([string]$Version, [switch]$Check)
$ErrorActionPreference = 'Stop'
if (-not $Version) { $Version = Read-Host 'New release version (for example 0.1.19)' }
$arguments = @((Join-Path $PSScriptRoot 'build_windows.py'), '--version', $Version)
if ($Check) { $arguments += '--check' }
& python @arguments
exit $LASTEXITCODE
