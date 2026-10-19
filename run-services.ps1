#Requires -Version 5.1
[CmdletBinding()]
param(
    [ValidateSet('Install', 'Start', 'Stop', 'Status', 'Uninstall', 'Firewall')][string]$Action = 'Status',
    [string]$StateDirectory = "$env:ProgramData\EitmadHost",
    [ValidatePattern('^[A-Za-z][A-Za-z0-9]{0,31}$')][string]$Name = 'Eitmad',
    [string]$ClientAddress,
    [switch]$WhatIf,
    [string]$LanDirectory,
    [string]$ClusterDirectory,
    [string]$PostgresBin,
    [string]$WinSwExecutable,
    [ValidatePattern('^[A-Fa-f0-9]{64}$')][string]$WinSwSha256
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
Import-Module (Join-Path $PSScriptRoot 'deploy/windows/host.psm1') -Force
Invoke-EitmadHost @PSBoundParameters
