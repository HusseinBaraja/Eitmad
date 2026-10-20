$ErrorActionPreference = 'Stop'
$profiles = @(Get-NetConnectionProfile)
$rows = @(foreach ($ip in @(Get-NetIPAddress -AddressFamily IPv4 -AddressState Preferred)) {
    $adapter = Get-NetAdapter -InterfaceIndex $ip.InterfaceIndex -ErrorAction SilentlyContinue
    if (-not $adapter -or $adapter.Status -ne 'Up') { continue }
    $matches = @($profiles | Where-Object { $_.InterfaceIndex -eq $ip.InterfaceIndex })
    $private = $matches.Count -gt 0 -and @($matches | Where-Object { $_.NetworkCategory -ne 'Private' }).Count -eq 0
    @{index=[uint32]$ip.InterfaceIndex; address=[string]$ip.IPAddress; prefix=[byte]$ip.PrefixLength; private=[bool]$private}
})
ConvertTo-Json -InputObject $rows -Compress
