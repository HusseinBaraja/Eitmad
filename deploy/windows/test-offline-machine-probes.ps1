#Requires -Version 5.1
# Exercise acceptance verdicts with synthetic command boundaries. No network or installation calls.
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = Join-Path ([IO.Path]::GetTempPath()) ('eitmad-machine-probes-' + [Guid]::NewGuid().ToString('N'))
[void][IO.Directory]::CreateDirectory($root)
[IO.File]::WriteAllText((Join-Path $root 'baseline.json'), '{}')
$global:OfflineProbeTcp = $false
$global:OfflineProbeHttp = '204'
$global:OfflineProbeCurlExit = 0
function curl.exe { $global:LASTEXITCODE = $global:OfflineProbeCurlExit; return $global:OfflineProbeHttp }
function Get-NetConnectionProfile { [pscustomobject]@{NetworkCategory='Private'} }
function Test-NetConnection { param($ComputerName, $Port, $InformationLevel, $WarningAction) return $global:OfflineProbeTcp }
$probe = @{EvidenceDirectory=$root; Endpoint='https://synthetic.invalid:58443/'; TrustedCa='synthetic.pem'; ServerAddress='192.0.2.10'}
function Invoke-Probe([string]$Stage, [bool]$MustPass) {
    $failed = $false
    try { & (Join-Path $PSScriptRoot 'test-offline-machine.ps1') -Stage $Stage @probe } catch { $failed=$true; if ($MustPass) { throw } }
    if ($failed -eq $MustPass) { throw "Incorrect $Stage acceptance verdict." }
}
Invoke-Probe Private $true
$global:OfflineProbeHttp = '503'
Invoke-Probe Private $false
$global:OfflineProbeHttp = '000'; $global:OfflineProbeCurlExit = 60
Invoke-Probe Private $false
# A certificate failure with an open TCP port cannot be reported as Public firewall denial.
$global:OfflineProbeTcp = $true
Invoke-Probe Public $false
$global:OfflineProbeTcp = $false; $global:OfflineProbeCurlExit = 7
Invoke-Probe Public $true
$global:OfflineProbeHttp = '204'; $global:OfflineProbeCurlExit = 0
Invoke-Probe Public $false
$global:OfflineProbeTcp = $true
Invoke-Probe Private $false
Write-Host 'PASS: validated 204, readiness/TLS failures, reachable database, and Public TCP versus certificate failure verdicts. All commands were synthetic.'
