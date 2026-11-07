$ErrorActionPreference = 'Stop'
# Query the providers directly: loading the three CDXML wrappers can exceed the
# bounded profile read under the installed Windows service account.
$namespace = 'root/StandardCimv2'
$profiles = @(Get-CimInstance -Namespace $namespace -ClassName MSFT_NetConnectionProfile)
$addresses = @(Get-CimInstance -Namespace $namespace -ClassName MSFT_NetIPAddress -Filter 'AddressFamily=2 AND AddressState=4')
$adapters = @(Get-CimInstance -Namespace $namespace -ClassName MSFT_NetAdapter)
$rows = @(foreach ($ip in $addresses) {
    $adapter = @($adapters | Where-Object { $_.InterfaceIndex -eq $ip.InterfaceIndex })
    if (-not $adapter.Count -or @($adapter | Where-Object { $_.InterfaceOperationalStatus -ne 1 }).Count) { continue }
    $matches = @($profiles | Where-Object { $_.InterfaceIndex -eq $ip.InterfaceIndex })
    $private = $matches.Count -gt 0 -and @($matches | Where-Object { $_.NetworkCategory -ne 1 }).Count -eq 0
    @{index=[uint32]$ip.InterfaceIndex; address=[string]$ip.IPAddress; prefix=[byte]$ip.PrefixLength; private=[bool]$private}
})
ConvertTo-Json -InputObject $rows -Compress
