#Requires -Version 5.1
[CmdletBinding()]
param([ValidateSet('Run', 'Status', 'Install')][string]$Action = 'Status', [Parameter(Mandatory)][string]$PlanPath)
$ErrorActionPreference = 'Stop'
Import-Module (Join-Path $PSScriptRoot 'backup.psm1') -Force
try { Invoke-EitmadBackup @PSBoundParameters }
catch { Write-Error 'eitmad.backup.failed.v1'; exit 1 }
