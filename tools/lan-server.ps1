#Requires -Version 7.4
[CmdletBinding(SupportsShouldProcess)]
param(
    [Parameter(Mandatory)][ValidateSet('Save', 'Check', 'Start', 'Firewall', 'EnableRenewal')][string]$Action,
    [Parameter(Mandatory)][string]$StateDirectory,
    [string]$ServerExecutable,
    [string]$ListenAddress,
    [ValidateRange(1024, 65535)][int]$Port = 8443,
    [string[]]$ClientAddress,
    [string]$HostDirectory = "$env:ProgramData\EitmadHost"
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$root = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($StateDirectory)
$certificateAssembly = Join-Path $PSScriptRoot 'lan-certificate/bin/Release/net8.0-windows/Eitmad.LanCertificate.dll'
if (-not (Test-Path -LiteralPath $certificateAssembly)) { throw 'Build Eitmad.LanCertificate in Release first.' }
Add-Type -Path $certificateAssembly
[Eitmad.LanCertificate.Certificates]::RequirePrivateDirectory($root)
if ($Action -eq 'EnableRenewal') {
    if (-not $PSCmdlet.ShouldProcess($HostDirectory, 'Retain protected CA material and schedule Rust TLS renewal')) { return }
    $backend = (Get-Content -LiteralPath (Join-Path $root 'server.json') -Raw | ConvertFrom-Json).executable
    # Private stdin only. Neither the scheduler nor process arguments receive the unlock credential.
    $secure = Read-Host 'Retained CA password' -AsSecureString
    $pointer = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($secure)
    $password = $null
    $previousEncoding = $OutputEncoding
    $OutputEncoding = [Text.UTF8Encoding]::new($false, $true)
    try {
        $password = [Runtime.InteropServices.Marshal]::PtrToStringBSTR($pointer)
        @{lanDirectory=$root; certificateExecutable=(Join-Path (Split-Path $certificateAssembly -Parent) 'Eitmad.LanCertificate.exe')
            password=$password} | ConvertTo-Json -Compress | & $backend windows-tls configure $HostDirectory
        if ($LASTEXITCODE -ne 0) { throw 'Renewal configuration failed. Check elevation, retained CA, tool deployment, and protected host state.' }
    } finally {
        [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($pointer)
        $secure.Dispose(); $password = $null
        $OutputEncoding = $previousEncoding
    }
    return
}
$settingsFile = Join-Path $root 'server.json'
$identity = Get-Content -LiteralPath (Join-Path $root 'identity.json') -Raw | ConvertFrom-Json
if ($identity.Version -ne 1) { throw 'Unsupported LAN identity version.' }
$names = @('EITMAD_SERVER_DATABASE_URL', 'EITMAD_SERVER_TOKEN_KEY', 'EITMAD_SERVER_MAX_CONNECTIONS',
    'EITMAD_SERVER_UPDATE_MANIFEST_DIRECTORY', 'EITMAD_SERVER_UPDATE_OPERATOR_TENANT_ID',
    'EITMAD_SERVER_UPDATE_KEY_ID', 'EITMAD_SERVER_UPDATE_PUBLIC_KEY')

function Assert-PrivateAddress([string]$Value) {
    $address = [Net.IPAddress]::Parse($Value)
    $bytes = $address.GetAddressBytes()
    if ($bytes.Length -ne 4 -or -not ($bytes[0] -eq 10 -or
        ($bytes[0] -eq 172 -and $bytes[1] -ge 16 -and $bytes[1] -le 31) -or
        ($bytes[0] -eq 192 -and $bytes[1] -eq 168))) {
        throw 'Use an explicit RFC1918 IPv4 LAN address.'
    }
}

function Assert-PrivateDatabase($Environment) {
    $database = [Uri]$Environment.EITMAD_SERVER_DATABASE_URL
    if ($database.Scheme -notin @('postgres', 'postgresql') -or $database.Host -ne '127.0.0.1') {
        throw 'This LAN setup requires PostgreSQL on 127.0.0.1. Do not expose PostgreSQL to clients.'
    }
    return $(if ($database.Port -gt 0) { $database.Port } else { 5432 })
}

if ($Action -eq 'Save') {
    if (Test-Path -LiteralPath $settingsFile) { throw 'Server settings already exist. Review and edit the protected server.json for an intentional configuration change.' }
    Assert-PrivateAddress $ListenAddress
    $certificateAddress = $null
    if ([Net.IPAddress]::TryParse($identity.ServerName, [ref]$certificateAddress) -and
        -not $certificateAddress.Equals([Net.IPAddress]::Parse($ListenAddress))) {
        throw 'An IP certificate must match the saved listener address.'
    }
    $executable = (Resolve-Path -LiteralPath $ServerExecutable).Path
    $environment = [ordered]@{}
    foreach ($name in $names) {
        $value = [Environment]::GetEnvironmentVariable($name, 'Process')
        if ($value) { $environment[$name] = $value }
        elseif ($name -ne 'EITMAD_SERVER_MAX_CONNECTIONS') { throw "Required deployment setting is missing: $name" }
    }
    $databasePort = Assert-PrivateDatabase $environment
    if ($databasePort -eq $Port) { throw 'Database and HTTPS ports must differ.' }
    $settings = [ordered]@{ version = 1; executable = $executable; address = $ListenAddress; port = $Port
        endpoint = "https://$($identity.ServerName):$Port/"; environment = $environment }
} else {
    $settings = Get-Content -LiteralPath $settingsFile -Raw | ConvertFrom-Json -AsHashtable
    if ($settings.version -ne 1 -or $settings.endpoint -cne "https://$($identity.ServerName):$($settings.port)/") {
        throw 'Invalid retained LAN server settings.'
    }
    Assert-PrivateAddress $settings.address
    $databasePort = Assert-PrivateDatabase $settings.environment
    if ([int]$settings.port -notin 1024..65535 -or $databasePort -eq [int]$settings.port) {
        throw 'Invalid HTTPS or database port.'
    }
}

if ($Action -eq 'Firewall') {
    if (-not $ClientAddress) { throw 'Supply the exact client IPv4 addresses or private subnet CIDRs with -ClientAddress.' }
    foreach ($client in $ClientAddress) {
        $parts = $client.Split('/')
        Assert-PrivateAddress $parts[0]
        if ($parts.Count -gt 2 -or ($parts.Count -eq 2 -and [int]$parts[1] -notin 24..32)) {
            throw 'Use individual private clients or a narrow /24 through /32 subnet.'
        }
    }
    $ruleName = 'Eitmad-LAN-' + [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($root))).Substring(0, 16)
    if (Get-NetFirewallRule -Name $ruleName -ErrorAction SilentlyContinue) {
        throw "Rule $ruleName already exists. Inspect it before an explicit replacement."
    }
    New-NetFirewallRule -Name $ruleName -DisplayName 'Eitmad LAN HTTPS' -Direction Inbound -Action Allow `
        -Profile Private -Protocol TCP -LocalAddress $settings.address -LocalPort $settings.port `
        -RemoteAddress $ClientAddress -Program $settings.executable -EdgeTraversalPolicy Block -WhatIf:$WhatIfPreference | Out-Null
    if ($WhatIfPreference) { Write-Host 'Firewall preview complete; no policy changed.' }
    else { Write-Host "Created private-network HTTPS rule: $ruleName" }
    return
}

$active = Get-Content -LiteralPath (Join-Path $root 'active-tls.json') -Raw | ConvertFrom-Json
if ($active.Version -ne 1) { throw 'Unsupported TLS bundle version.' }
$certificate = Join-Path $root $active.Certificate
$privateKey = Join-Path $root $active.PrivateKey
foreach ($file in @($certificate, $privateKey)) {
    $resolved = (Resolve-Path -LiteralPath $file).Path
    if (-not $resolved.StartsWith($root + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
        throw 'TLS bundle must remain inside the private state directory.'
    }
}
# Loading the pair catches incomplete or mismatched material before starting Rust.
$pair = [Security.Cryptography.X509Certificates.X509Certificate2]::CreateFromPemFile($certificate, $privateKey)
try {
    if ($pair.NotBefore.ToUniversalTime() -gt [DateTime]::UtcNow -or $pair.NotAfter.ToUniversalTime() -le [DateTime]::UtcNow) {
        throw 'Server certificate is not currently valid. Renew it without changing client trust.'
    }
} finally { $pair.Dispose() }

$previous = @{}
$allNames = @($names) + @('EITMAD_SERVER_LISTEN', 'EITMAD_SERVER_TLS_CERTIFICATE', 'EITMAD_SERVER_TLS_PRIVATE_KEY',
    'EITMAD_SERVER_ALLOW_INSECURE_LOOPBACK') + @(Get-ChildItem Env:EITMAD_SERVER_* | ForEach-Object Name)
foreach ($name in ($allNames | Select-Object -Unique)) {
    $previous[$name] = [Environment]::GetEnvironmentVariable($name, 'Process')
    Remove-Item -LiteralPath "Env:$name" -ErrorAction SilentlyContinue
}
try {
    foreach ($name in $settings.environment.Keys) {
        if ($name -notin $names) { throw 'Unknown persisted server setting.' }
        [Environment]::SetEnvironmentVariable($name, $settings.environment[$name], 'Process')
    }
    $env:EITMAD_SERVER_LISTEN = "$($settings.address):$($settings.port)"
    $env:EITMAD_SERVER_TLS_CERTIFICATE = $certificate
    $env:EITMAD_SERVER_TLS_PRIVATE_KEY = $privateKey
    & $settings.executable check-config
    if ($LASTEXITCODE -ne 0) { throw 'Rust rejected the server configuration.' }
    if ($Action -eq 'Save') {
        if (-not $PSCmdlet.ShouldProcess($settingsFile, 'Save private LAN server settings')) { return }
        $pending = Join-Path $root 'server.pending.json'
        $settings | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $pending
        [IO.File]::Move($pending, $settingsFile, $false)
        Write-Host "Saved LAN endpoint: $($settings.endpoint)"
    } elseif ($Action -eq 'Start') {
        if (-not $PSCmdlet.ShouldProcess($settings.executable, 'Start LAN server')) { return }
        $listeners = @(Get-NetTCPConnection -State Listen -LocalPort $databasePort -ErrorAction Stop)
        if (-not $listeners.Count -or @($listeners | Where-Object LocalAddress -NotIn @('127.0.0.1', '::1')).Count) {
            throw 'PostgreSQL must be running and listen only on loopback before LAN startup.'
        }
        # Foreground diagnostics only. run-services.ps1 owns installed unattended hosts.
        & $settings.executable serve
        if ($LASTEXITCODE -ne 0) { throw 'The LAN server stopped with an error. Preserve its private state.' }
    }
} finally {
    foreach ($name in $previous.Keys) {
        if ($null -eq $previous[$name]) { Remove-Item -LiteralPath "Env:$name" -ErrorAction SilentlyContinue }
        else { [Environment]::SetEnvironmentVariable($name, $previous[$name], 'Process') }
    }
}
