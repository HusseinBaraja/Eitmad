#Requires -Version 5.1
[CmdletBinding()]
param(
    [Parameter(Mandatory)][ValidateSet('ClientPreflight', 'ServerPreflight', 'Elevate', 'Worker', 'Desktop', 'Install', 'ResumeInstall', 'Firewall', 'Start')][string]$Action,
    [Parameter(Mandatory)][string]$Backend,
    [string]$PipeName,
    [int]$BrokerId
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$env:PSModulePath = Join-Path $PSHOME 'Modules'
[Console]::InputEncoding = New-Object Text.UTF8Encoding($false, $true)
[Console]::OutputEncoding = New-Object Text.UTF8Encoding($false)
Import-Module (Join-Path $PSScriptRoot 'installation.psm1') -Force
try {
    if ($Action -eq 'Worker') {
        $code = Invoke-EitmadInstallationWorker -Backend $Backend -PipeName $PipeName -BrokerId $BrokerId
        exit $code
    }
    if ($Action -eq 'Elevate') {
        Add-Type -Path (Join-Path $PSScriptRoot 'installation-channel.cs')
    }
    $privateInput = if ($Action -eq 'Elevate') { [EitmadInstallationChannel]::ReadPrivateInput([Console]::In) }
        else { [Console]::In.ReadToEnd() }
    if ([Text.Encoding]::UTF8.GetByteCount($privateInput) -gt 65536) { throw 'Invalid private input.' }
    if ($Action -eq 'Elevate') {
        $code = Invoke-EitmadInstallationElevation -Backend $Backend -PrivateInput $privateInput
        exit $code
    }
    $request = $privateInput | ConvertFrom-Json
    $privateInput = $null
    Invoke-EitmadInstallationAction -Action $Action -Backend $Backend -Request $request
} catch {
    # OS exceptions can contain private paths or child text. Rust reports the typed failed stage.
    exit 1
} finally { $privateInput = $null; $request = $null }
