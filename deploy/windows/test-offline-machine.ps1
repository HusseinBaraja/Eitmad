#Requires -Version 5.1
# Read-only acceptance probes for disposable lab PCs. Setup is launched separately by the operator.
[CmdletBinding()]
param(
    [Parameter(Mandatory)][ValidateSet('Baseline','Client','Server','Private','Public')][string]$Stage,
    [Parameter(Mandatory)][string]$EvidenceDirectory,
    [string]$BundleDirectory,
    [ValidatePattern('^[a-fA-F0-9]{64}$')][string]$ExpectedCatalogSha256,
    [string]$Endpoint, [string]$TrustedCa,
    [string]$ServerAddress, [int]$DatabasePort = 55439
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
function Assert($Value, [string]$Message) { if (-not $Value) { throw $Message } }
function RuntimeState {
    $values = foreach ($path in 'HKLM:\SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\x64',
        'HKLM:\SOFTWARE\Microsoft\NET Framework Setup\NDP\v4\Full') {
        $value = Get-ItemProperty -LiteralPath $path -ErrorAction SilentlyContinue
        if ($value) { $value | Select-Object Version, Release -ErrorAction SilentlyContinue }
        else { $null }
    }
    return ConvertTo-Json -InputObject @($values) -Compress
}
function Snapshot {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($identity)
    return [pscustomobject]@{
        computer = $env:COMPUTERNAME; sid = $identity.User.Value
        elevated = $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
        administratorGroupPresent = @($identity.Groups | Where-Object { $_.Value -eq 'S-1-5-32-544' }).Count -gt 0
        build = [Environment]::OSVersion.Version.ToString(); runtimeState = RuntimeState
        services = @(Get-Service | Sort-Object Name | Select-Object -ExpandProperty Name)
        firewall = @(Get-NetFirewallRule -PolicyStore PersistentStore | Sort-Object Name | Select-Object -ExpandProperty Name)
    }
}
function NoDevelopmentTools {
    foreach ($name in 'cargo','rustc','cl','msbuild','csc','makeappx','signtool','dotnet') {
        Assert (-not (Get-Command $name -ErrorAction SilentlyContinue)) "Clean offline proof requires $name to be absent."
    }
    foreach ($path in (Join-Path $env:ProgramFiles 'dotnet'), (Join-Path $env:ProgramFiles 'Microsoft Visual Studio'),
        (Join-Path ${env:ProgramFiles(x86)} 'Windows Kits'), (Join-Path $env:USERPROFILE '.cargo/bin/rustc.exe')) {
        Assert (-not (Test-Path -LiteralPath $path)) 'A development toolchain or separate .NET runtime exists. Use a clean lab image.'
    }
}
$root = [IO.Path]::GetFullPath($EvidenceDirectory)
$baselineFile = Join-Path $root 'baseline.json'
if ($Stage -eq 'Baseline') {
    Assert (-not (Test-Path -LiteralPath $root)) 'Use a new evidence directory. Existing evidence is retained.'
    Assert ($BundleDirectory -and $ExpectedCatalogSha256) 'Supply the extracted signed bundle and independently authenticated catalog SHA-256.'
    $catalog = Join-Path $BundleDirectory 'offline.cat'
    Assert ((Get-FileHash -LiteralPath $catalog).Hash -ieq $ExpectedCatalogSha256) 'Catalog differs from the approved handover.'
    # Reuse the same fail-closed trust/catalog boundary as the launcher. No payload is executed.
    @{root=[IO.Path]::GetFullPath($BundleDirectory); anchor=(Join-Path $BundleDirectory 'setup/EitmadSetup.exe'); publisher=''} |
        ConvertTo-Json -Compress | & "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -NonInteractive -File (Join-Path $PSScriptRoot 'verify-offline.ps1')
    Assert ($LASTEXITCODE -eq 0) 'Bundle authentication failed.'
    NoDevelopmentTools
    $state = Snapshot
    Assert (-not $state.elevated -and -not $state.administratorGroupPresent) 'Capture Baseline as an ordinary user without Administrators membership.'
    Assert (-not @(Get-AppxPackage -Name Eitmad.Desktop).Count) 'Desktop is already installed.'
    Assert (-not @($state.services | Where-Object { $_ -match 'Eitmad|postgres' }).Count) 'A server component already exists.'
    foreach ($path in (Join-Path $env:LOCALAPPDATA 'Eitmad'), (Join-Path $env:LOCALAPPDATA 'EitmadSetupProgress'),
        (Join-Path $env:ProgramData 'EitmadHost'), (Join-Path $env:ProgramData 'EitmadPreparedWorkshop'),
        (Join-Path $env:ProgramData 'EitmadServerSetupProgress'), (Join-Path $env:ProgramData 'EitmadBackupState')) {
        Assert (-not (Test-Path -LiteralPath $path)) 'Existing product state found. Preserve it and use a clean lab image.'
    }
    $state | Add-Member NoteProperty catalogSha256 $ExpectedCatalogSha256.ToLowerInvariant()
    [void][IO.Directory]::CreateDirectory($root)
    $state | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $baselineFile -Encoding UTF8
} elseif ($Stage -in 'Client','Server') {
    $baseline = Get-Content -LiteralPath $baselineFile -Raw -Encoding UTF8 | ConvertFrom-Json
    $state = Snapshot
    Assert ($state.computer -ceq $baseline.computer) 'Evidence belongs to another PC.'
    if ($Stage -eq 'Client') {
        Assert ($state.sid -ceq $baseline.sid -and -not $state.elevated -and -not $state.administratorGroupPresent) 'Run Client as the original ordinary user without Administrators membership.'
        NoDevelopmentTools
        Assert ($state.runtimeState -ceq $baseline.runtimeState) 'Setup changed machine-wide runtime registration.'
        Assert (-not @(Compare-Object @($baseline.services) @($state.services)).Count) 'Setup changed installed services.'
        Assert (-not @(Compare-Object @($baseline.firewall) @($state.firewall)).Count) 'Setup changed firewall rule registrations.'
        Assert (-not @(Get-ChildItem -LiteralPath $env:ProgramData -Filter 'Eitmad*' -ErrorAction Stop).Count) 'Client installed host state or an elevated payload.'
        $packages = @(Get-AppxPackage -Name Eitmad.Desktop)
        Assert ($packages.Count -eq 1 -and $packages[0].Status -eq 'Ok') 'Per-user desktop registration is not healthy.'
        $files = @(Get-ChildItem -LiteralPath $packages[0].InstallLocation -Recurse -File)
        Assert (-not @($files | Where-Object { $_.Name -match '^(eitmad-server|postgres|initdb|psql|pg_.+|host-service|WinSW.*|pwsh)\.exe$' }).Count) 'Client contains server components.'
        $manifest = Get-AppxPackageManifest -Package $packages[0].PackageFullName
        Assert (-not @($manifest.Package.Dependencies.ChildNodes | Where-Object { $_.LocalName -ne 'TargetDeviceFamily' }).Count) 'Client requires an external package.'
        foreach ($name in 'coreclr.dll','hostfxr.dll','hostpolicy.dll','PresentationFramework.dll','vcruntime140.dll','vcruntime140_1.dll') {
            Assert ($name -in $files.Name) 'Client offline runtime is incomplete.'
        }
    } else {
        Assert $state.elevated 'Read Server service and listener evidence from a lab administrator window.'
        foreach ($kind in 'Server','Postgres') {
            $service = Get-CimInstance Win32_Service -Filter ("Name='Eitmad" + $kind + "'")
            Assert ($service -and $service.State -eq 'Running' -and $service.StartMode -eq 'Auto' -and
                $service.StartName -ceq ('NT SERVICE\Eitmad' + $kind)) 'Host service is not running automatically under its own virtual account.'
        }
        $listeners = @(Get-NetTCPConnection -State Listen -LocalPort $DatabasePort)
        Assert ($listeners.Count -gt 0 -and -not @($listeners | Where-Object { $_.LocalAddress -notin '127.0.0.1','::1' }).Count) 'PostgreSQL is not loopback-only.'
        foreach ($name in 'Eitmad-Host-Client','Eitmad-Host-Discovery') {
            $rule = Get-NetFirewallRule -PolicyStore ActiveStore -Name $name
            Assert ($rule.Enabled -eq 'True' -and $rule.Action -eq 'Allow' -and $rule.Direction -eq 'Inbound' -and
                [string]$rule.Profile -eq 'Private' -and $rule.EdgeTraversalPolicy -eq 'Block') 'Effective Eitmad firewall rule has the wrong policy.'
            Assert ((($rule | Get-NetFirewallAddressFilter).RemoteAddress -join ',') -eq 'LocalSubnet') 'Firewall source scope is not LocalSubnet.'
            Assert (($rule | Get-NetFirewallApplicationFilter).Program -ieq (Join-Path $env:ProgramData 'EitmadHost/app/eitmad-server.exe')) 'Firewall program differs from the installed host.'
            $port = $rule | Get-NetFirewallPortFilter
            $expected = if ($name -eq 'Eitmad-Host-Client') { '58443' } else { '58444' }
            Assert ([string]$port.LocalPort -eq $expected -and [string]$port.Protocol -eq $(if ($expected -eq '58443') {'TCP'} else {'UDP'})) 'Firewall port or protocol differs.'
        }
    }
} else {
    Assert ($Endpoint -and $TrustedCa -and $ServerAddress) 'Supply the retained HTTPS endpoint, independently verified CA, and current server address.'
    $uri = [Uri]$Endpoint
    Assert ($uri.Scheme -eq 'https' -and $uri.AbsolutePath -eq '/' -and -not $uri.UserInfo -and -not $uri.Query -and -not $uri.Fragment) 'Use the canonical HTTPS root endpoint.'
    $profiles = @(Get-NetConnectionProfile | Select-Object -ExpandProperty NetworkCategory)
    $code = & curl.exe --disable --noproxy '*' --ssl-revoke-best-effort --silent --output NUL --write-out '%{http_code}' --connect-timeout 3 --max-time 8 --cacert $TrustedCa ($Endpoint.TrimEnd('/') + '/readyz')
    $curlExit = $LASTEXITCODE
    if ($Stage -eq 'Private') { Assert ($curlExit -eq 0 -and $code -eq '204') 'Validated HTTPS /readyz did not return 204.' }
    else {
        # A TCP refusal/timeout is necessary: a TLS or name failure does not prove a firewall block.
        Assert (-not (Test-NetConnection $ServerAddress -Port $uri.Port -InformationLevel Quiet -WarningAction SilentlyContinue)) 'HTTPS TCP remains reachable with the server on Public.'
        Assert ($curlExit -ne 0) 'HTTPS remains reachable on Public.'
    }
    Assert (-not (Test-NetConnection $ServerAddress -Port $DatabasePort -InformationLevel Quiet -WarningAction SilentlyContinue)) 'PostgreSQL is reachable from a client.'
    $state = [pscustomobject]@{computer=$env:COMPUTERNAME; observedClientProfiles=$profiles; curlExit=$curlExit; httpStatus=[string]$code}
}
# Every file is a new observation. No stage attests UI, offline isolation, boot, DHCP or restore.
if ($Stage -ne 'Baseline') {
    Assert (Test-Path -LiteralPath $baselineFile) 'Retain the original clean baseline on this client.'
    $result = @{stage=$Stage; observedAtUtc=[DateTime]::UtcNow.ToString('o'); observations=$state; fullAcceptance=$false}
    $file = Join-Path $root ($Stage + '-' + [Guid]::NewGuid().ToString('N') + '.json')
    $result | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $file -Encoding UTF8
}
Write-Host "PASS: $Stage observations only. Complete the separate-PC operator procedure; full acceptance is not asserted."
