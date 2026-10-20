#Requires -Version 5.1
# Only synthetic cmdlet responses. No live network, profile, or firewall calls.
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
function Get-NetConnectionProfile {
    @(
        [pscustomobject]@{InterfaceIndex=1; NetworkCategory='Private'},
        [pscustomobject]@{InterfaceIndex=2; NetworkCategory='Public'},
        [pscustomobject]@{InterfaceIndex=3; NetworkCategory='DomainAuthenticated'},
        [pscustomobject]@{InterfaceIndex=5; NetworkCategory='Private'},
        [pscustomobject]@{InterfaceIndex=5; NetworkCategory='Public'}
    )
}
function Get-NetIPAddress {
    param($AddressFamily, $AddressState)
    foreach ($index in 1..6) {
        [pscustomobject]@{InterfaceIndex=$index; IPAddress="192.168.$index.10"; PrefixLength=24}
    }
}
function Get-NetAdapter {
    param($InterfaceIndex, $ErrorAction)
    [pscustomobject]@{Status=$(if ($InterfaceIndex -eq 6) { 'Down' } else { 'Up' })}
}
$adapter = Join-Path $PSScriptRoot '../../crates/server-discovery/src/interfaces.ps1'
$rows = (& $adapter) | ConvertFrom-Json
if ($rows.Count -ne 5 -or @($rows | Where-Object { $_.private }).Count -ne 1 -or
    -not $rows[0].private -or $rows[0].index -ne 1) {
    throw 'Only active interfaces with exclusively Private profiles may qualify.'
}
Write-Host 'PASS: synthetic Private, Public, Domain, unknown, mixed-profile, and disconnected interfaces.'
