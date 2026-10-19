Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Invoke-Native([string]$File, [string[]]$Arguments) {
    # Preserve embedded quotes and trailing backslashes through Windows argv parsing on PS 5.1.
    $quoted = @($Arguments | ForEach-Object {
        '"' + [regex]::Replace([regex]::Replace($_, '(\\*)"', '$1$1\"'), '(\\+)$', '$1$1') + '"'
    })
    $start = New-Object Diagnostics.ProcessStartInfo
    $start.FileName = $File; $start.Arguments = $quoted -join ' '
    $start.UseShellExecute = $false; $start.CreateNoWindow = $true
    $start.RedirectStandardOutput = $true; $start.RedirectStandardError = $true
    $process = [Diagnostics.Process]::Start($start)
    try {
        $stdout = $process.StandardOutput.ReadToEndAsync(); $stderr = $process.StandardError.ReadToEndAsync()
        if (-not $process.WaitForExit(60000)) { throw 'Host tool deadline elapsed. Inspect service state before retrying.' }
        [void]$stdout.GetAwaiter().GetResult(); [void]$stderr.GetAwaiter().GetResult()
        if ($process.ExitCode -ne 0) { throw "Host tool failed: $([IO.Path]::GetFileName($File)) (exit $($process.ExitCode))." }
    } finally { $process.Dispose() }
}

function Assert-Administrator {
    $principal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
    if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        throw 'Run the service command in an elevated PowerShell window. No service was changed.'
    }
}

function Get-SafePath([string]$Path) {
    $full = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($Path).TrimEnd('\')
    if ($full.StartsWith('\\') -or $full -match '["%\r\n]' -or $full.Length -le 3) {
        throw 'Use a local directory without quotes, percent signs, or line breaks.'
    }
    for ($part = $full; $part; $part = Split-Path $part -Parent) {
        if ((Test-Path -LiteralPath $part) -and
            ((Get-Item -LiteralPath $part -Force).Attributes -band [IO.FileAttributes]::ReparsePoint)) {
            throw 'Service paths cannot contain reparse points.'
        }
    }
    return $full
}

function Assert-PlainTree([string]$Path) {
    [void](Get-SafePath $Path)
    if (@(Get-ChildItem -LiteralPath $Path -Force -Recurse | Where-Object {
        $_.Attributes -band [IO.FileAttributes]::ReparsePoint
    }).Count) { throw 'Service inputs cannot contain junctions, symlinks, or external tablespaces.' }
}

function Assert-ProtectedParents([string]$Root) {
    # DELETE_CHILD or write access on an ancestor can replace a protected service directory.
    $trusted = @('S-1-5-18', 'S-1-5-32-544',
        'S-1-5-80-956008885-3418522649-1831038044-1853292631-2271478464') # TrustedInstaller
    for ($parent = Split-Path $Root -Parent; $parent; $parent = Split-Path $parent -Parent) {
        $acl = Get-Acl -LiteralPath $parent
        if ($acl.GetOwner([Security.Principal.SecurityIdentifier]).Value -notin $trusted) {
            throw 'The host directory requires administrator-owned ancestors.'
        }
        foreach ($rule in $acl.GetAccessRules($true, $true, [Security.Principal.SecurityIdentifier])) {
            if ($rule.AccessControlType -ne 'Allow' -or $rule.IdentityReference.Value -in $trusted -or
                ($rule.PropagationFlags -band [Security.AccessControl.PropagationFlags]::InheritOnly)) { continue }
            # Creating sibling folders under ProgramData is allowed. Replacing this directory is not.
            $dangerous = [Security.AccessControl.FileSystemRights]::DeleteSubdirectoriesAndFiles -bor
                [Security.AccessControl.FileSystemRights]::Delete -bor
                [Security.AccessControl.FileSystemRights]::ChangePermissions -bor
                [Security.AccessControl.FileSystemRights]::TakeOwnership
            if ($rule.FileSystemRights -band $dangerous) { throw 'An ancestor permits untrusted replacement of service files.' }
        }
    }
}

function Assert-InstalledProtection([string]$Root, [string]$Name, [switch]$AllowMissingServerBinary) {
    $trusted = @('S-1-5-18', 'S-1-5-32-544')
    # showsid derives the stable SID even after Uninstall removes the account-name mapping.
    $sids = @{}
    foreach ($kind in 'Server', 'Postgres') {
        $output = & "$env:SystemRoot\System32\sc.exe" showsid ($Name + $kind)
        $sids[$kind] = [regex]::Match(($output -join ' '), 'S-1-5-80-(?:\d+-){4}\d+').Value
        if ($LASTEXITCODE -ne 0 -or -not $sids[$kind]) { throw 'Cannot identify the service security principal.' }
    }
    $serverSid = $sids.Server; $databaseSid = $sids.Postgres
    foreach ($relative in '', 'host.json', 'app', 'private', 'postgres', 'app/eitmad-server.exe', 'app/host-service.exe',
        'app/host-service.xml', 'postgres/host-service.exe', 'postgres/host-service.xml', 'postgres/bin/postgres.exe',
        'private/database-url', 'private/token-key',
        'private/server-key.pem', 'private/server-cert.pem', 'private/trusted-ca.pem') {
        $path = if ($relative) { Join-Path $Root $relative } else { $Root }
        if ($AllowMissingServerBinary -and $relative -eq 'app/eitmad-server.exe' -and -not (Test-Path -LiteralPath $path)) { continue }
        $acl = Get-Acl -LiteralPath $path
        if ($acl.GetOwner([Security.Principal.SecurityIdentifier]).Value -notin $trusted) {
            throw 'Installed control files require Administrators or SYSTEM ownership.'
        }
        $readers = if ($relative -eq 'host.json') { @($serverSid, $databaseSid) }
            elseif ($relative -like 'postgres*') { @($databaseSid) }
            elseif ($relative) { @($serverSid) } else { @($serverSid, $databaseSid) }
        foreach ($rule in $acl.GetAccessRules($true, $true, [Security.Principal.SecurityIdentifier])) {
            if ($rule.AccessControlType -ne 'Allow' -or $rule.IdentityReference.Value -in $trusted) { continue }
            $write = [Security.AccessControl.FileSystemRights]::Write -bor [Security.AccessControl.FileSystemRights]::Delete -bor
                [Security.AccessControl.FileSystemRights]::DeleteSubdirectoriesAndFiles -bor
                [Security.AccessControl.FileSystemRights]::ChangePermissions -bor [Security.AccessControl.FileSystemRights]::TakeOwnership
            if ($rule.IdentityReference.Value -notin $readers -or ($rule.FileSystemRights -band $write)) {
                throw 'Installed control files or secrets have unsafe access rules. No service was changed.'
            }
        }
    }
}

function Protect-Directory([string]$Path, [hashtable]$Readers = @{}) {
    [void][IO.Directory]::CreateDirectory($Path)
    $acl = New-Object Security.AccessControl.DirectorySecurity
    $acl.SetAccessRuleProtection($true, $false)
    $acl.SetOwner((New-Object Security.Principal.SecurityIdentifier('S-1-5-32-544')))
    foreach ($id in 'S-1-5-18', 'S-1-5-32-544') {
        $sid = New-Object Security.Principal.SecurityIdentifier($id)
        $acl.AddAccessRule((New-Object Security.AccessControl.FileSystemAccessRule(
            $sid, 'FullControl', 'ContainerInherit,ObjectInherit', 'None', 'Allow')))
    }
    foreach ($entry in $Readers.GetEnumerator()) {
        $account = New-Object Security.Principal.NTAccount($entry.Key)
        $acl.AddAccessRule((New-Object Security.AccessControl.FileSystemAccessRule(
            $account, $entry.Value, 'ContainerInherit,ObjectInherit', 'None', 'Allow')))
    }
    Set-Acl -LiteralPath $Path -AclObject $acl
}

function Ensure-EventSource([string]$Name) {
    # Restricted service accounts cannot register their own Event Log source on first boot.
    if ([Diagnostics.EventLog]::SourceExists($Name)) {
        if ([Diagnostics.EventLog]::LogNameFromSourceName($Name, '.') -cne 'Application') {
            throw 'The service Event Log source is registered to another log.'
        }
    } else { [Diagnostics.EventLog]::CreateEventSource($Name, 'Application') }
}

function Read-HostManifest([string]$Root, [string]$Name) {
    $file = Join-Path $Root 'host.json'
    if (-not (Test-Path -LiteralPath $file -PathType Leaf)) { throw 'No installed host manifest. Use Install with the required inputs.' }
    $settings = Get-Content -LiteralPath $file -Raw | ConvertFrom-Json
    if ($settings.version -ne 1 -or $settings.name -cne $Name -or $settings.root -cne $Root) {
        throw 'The host manifest does not match this directory and service name.'
    }
    return $settings
}

function Get-OwnedService($Settings, [string]$Kind) {
    $id = $Settings.name + $Kind
    $service = Get-Service -Name $id -ErrorAction SilentlyContinue
    if (-not $service) { return $null }
    $registry = Get-ItemProperty -LiteralPath "HKLM:\SYSTEM\CurrentControlSet\Services\$id"
    $executable = if ($Kind -eq 'Server') { Join-Path $Settings.root 'app/host-service.exe' }
        else { Join-Path $Settings.root 'postgres/host-service.exe' }
    # A matching name alone does not prove service ownership.
    if (-not $registry.ImagePath.Equals(('"' + $executable + '"'), [StringComparison]::OrdinalIgnoreCase) -or
        $registry.ObjectName -cne "NT SERVICE\$id") {
        throw "Service $id has another executable or identity. No action was taken on it."
    }
    if ($Kind -eq 'Postgres') {
        $xml = [xml](Get-Content -LiteralPath (Join-Path $Settings.root 'postgres/host-service.xml') -Raw)
        if ($xml.service.executable -ine (Join-Path $Settings.root 'postgres/bin/postgres.exe') -or
            -not $xml.service.arguments.Contains(('-D "' + (Join-Path $Settings.root 'data') + '"'))) {
            throw 'The PostgreSQL service points to another executable or data directory.'
        }
    }
    return $service
}

function Test-Endpoint([string]$Endpoint, [string]$Route, [string]$CertificateAuthority) {
    # Offline private CAs have no CRL distribution point. Keep chain/name/expiry checks;
    # Schannel still rejects a known revoked certificate. Ignore user curlrc/proxy settings.
    $code = & curl.exe --disable --noproxy '*' --ssl-revoke-best-effort --silent --output NUL --write-out '%{http_code}' --connect-timeout 2 --max-time 4 --cacert $CertificateAuthority ($Endpoint + $Route)
    return $LASTEXITCODE -eq 0 -and $code -eq '204'
}

function Close-Service($Service) {
    if ($Service -is [ServiceProcess.ServiceController]) { $Service.Dispose() }
}

function Wait-Ready($Settings) {
    $deadline = [DateTime]::UtcNow.AddSeconds(150)
    do {
        $database = Get-OwnedService $Settings 'Postgres'; $server = Get-OwnedService $Settings 'Server'
        try {
            if ($database -and $server -and $database.Status -eq 'Running' -and $server.Status -eq 'Running' -and
                (Test-Endpoint $Settings.endpoint 'readyz' (Join-Path $Settings.root 'private/trusted-ca.pem'))) { return }
        } finally { Close-Service $server; Close-Service $database }
        Start-Sleep -Milliseconds 500
    } while ([DateTime]::UtcNow -lt $deadline)
    throw 'Readiness deadline elapsed. Check status, TLS trust/name, ports, database, and private logs. Services were retained.'
}

function Stop-Host($Settings) {
    foreach ($kind in 'Server', 'Postgres') {
        $service = Get-OwnedService $Settings $kind
        try {
            if ($service -and $service.Status -ne 'Stopped') {
                $service.Stop()
                $service.WaitForStatus([ServiceProcess.ServiceControllerStatus]::Stopped, [TimeSpan]::FromSeconds(60))
            }
        } finally { Close-Service $service }
    }
}

function New-WrapperXml($Settings, $Environment, [ValidateSet('Server', 'Postgres')][string]$Kind = 'Server') {
    $xml = New-Object Xml.XmlDocument
    $service = $xml.CreateElement('service'); [void]$xml.AppendChild($service)
    $values = [ordered]@{
        id = $Settings.name + $Kind; name = $Settings.name + ' ' + $Kind
        description = 'Eitmad host component; independent of desktop login and WPF.'
        executable = Join-Path $Settings.root 'app/eitmad-server.exe'; arguments = 'serve'
        workingdirectory = $Settings.root; startmode = 'Manual'
        stoptimeout = '40 sec'; resetfailure = '1 hour'
        logpath = Join-Path $Settings.root ('logs/' + $Kind.ToLowerInvariant())
    }
    if ($Kind -eq 'Server') { $values.depend = $Settings.name + 'Postgres' }
    else {
        # Supervise the postmaster directly. pg_ctl runservice reports exit 0 after a child crash.
        $data = Join-Path $Settings.root 'data'
        $values.executable = Join-Path $Settings.root 'postgres/bin/postgres.exe'
        $values.arguments = "-D `"$data`" -h 127.0.0.1 -p $($Settings.databasePort) -c `"data_directory=$data`" -c `"hba_file=$data\pg_hba.conf`" -c `"ident_file=$data\pg_ident.conf`" -c log_statement=none -c log_min_error_statement=panic -c log_min_messages=fatal -c log_destination=stderr -c logging_collector=off"
    }
    foreach ($entry in $values.GetEnumerator()) {
        $element = $xml.CreateElement($entry.Key); $element.InnerText = $entry.Value; [void]$service.AppendChild($element)
    }
    $account = $xml.CreateElement('serviceaccount')
    foreach ($entry in @{'domain' = 'NT SERVICE'; 'user' = ($Settings.name + $Kind)}.GetEnumerator()) {
        $element = $xml.CreateElement($entry.Key); $element.InnerText = $entry.Value; [void]$account.AppendChild($element)
    }
    [void]$service.AppendChild($account)
    foreach ($delay in '10 sec', '30 sec', '') {
        $element = $xml.CreateElement('onfailure')
        $element.SetAttribute('action', $(if ($delay) { 'restart' } else { 'none' }))
        if ($delay) { $element.SetAttribute('delay', $delay) }; [void]$service.AppendChild($element)
    }
    $log = $xml.CreateElement('log'); $log.SetAttribute('mode', 'roll-by-size')
    foreach ($entry in @{'sizeThreshold' = '10240'; 'keepFiles' = '4'}.GetEnumerator()) {
        $element = $xml.CreateElement($entry.Key); $element.InnerText = $entry.Value; [void]$log.AppendChild($element)
    }
    [void]$service.AppendChild($log)
    foreach ($entry in $Environment.GetEnumerator()) {
        $element = $xml.CreateElement('env'); $element.SetAttribute('name', $entry.Key)
        $element.SetAttribute('value', $entry.Value); [void]$service.AppendChild($element)
    }
    return $xml
}

function Test-RustConfiguration([string]$Root, [string]$Executable, [string]$Command = 'check-config') {
    $xml = [xml](Get-Content -LiteralPath (Join-Path $Root 'app/host-service.xml') -Raw)
    $start = New-Object Diagnostics.ProcessStartInfo
    $start.FileName = if ($Executable) { $Executable } else { Join-Path $Root 'app/eitmad-server.exe' }
    $start.Arguments = $Command
    $start.UseShellExecute = $false; $start.CreateNoWindow = $true
    $start.RedirectStandardOutput = $true; $start.RedirectStandardError = $true
    foreach ($key in @($start.EnvironmentVariables.Keys)) {
        if ($key.StartsWith('EITMAD_SERVER_')) { $start.EnvironmentVariables.Remove($key) }
    }
    foreach ($entry in $xml.service.env) { $start.EnvironmentVariables[$entry.name] = $entry.value }
    $process = [Diagnostics.Process]::Start($start)
    try {
        $stdout = $process.StandardOutput.ReadToEndAsync(); $stderr = $process.StandardError.ReadToEndAsync()
        if (-not $process.WaitForExit($(if ($Command -eq 'migrate') { 300000 } else { 10000 }))) {
            # This is our maintenance child, never a service or a desktop process.
            # Reap it before recovery can replace binaries or restore its database.
            $process.Kill()
            if (-not $process.WaitForExit(10000)) { throw 'Rust maintenance did not stop. Inspect process state before recovery.' }
            throw 'Rust maintenance deadline elapsed. Its process stopped; preserve state and inspect private logs before retrying.'
        }
        [void]$stdout.GetAwaiter().GetResult(); [void]$stderr.GetAwaiter().GetResult()
        if ($process.ExitCode -ne 0) { throw 'Rust rejected the installed configuration. Preserve state and correct the protected settings.' }
    } finally { $process.Dispose() }
}

function Initialize-Host([string]$Root, [string]$Name, [string]$LanDirectory,
    [string]$ClusterDirectory, [string]$PostgresBin, [string]$WinSwExecutable, [string]$WinSwSha256,
    [string]$ServerExecutable) {
    foreach ($value in $LanDirectory, $ClusterDirectory, $PostgresBin, $WinSwExecutable, $WinSwSha256) {
        if (-not $value) { throw 'Initial Install requires LanDirectory, ClusterDirectory, PostgresBin, WinSwExecutable, and WinSwSha256.' }
    }
    if (Test-Path -LiteralPath $Root) { throw 'Initial Install requires a new state directory. Existing data was not changed.' }
    $lan = Get-SafePath $LanDirectory; $cluster = Get-SafePath $ClusterDirectory
    $bin = Get-SafePath $PostgresBin; $wrapper = Get-SafePath $WinSwExecutable
    foreach ($tree in $lan, $cluster, (Split-Path $bin -Parent)) {
        Assert-PlainTree $tree
        if ($Root.StartsWith($tree + '\', [StringComparison]::OrdinalIgnoreCase) -or
            $tree.StartsWith($Root + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'Source and installation directories must be separate.' }
    }
    if (Test-Path -LiteralPath (Join-Path $cluster 'postmaster.pid')) { throw 'Import requires a stopped cluster. Do not remove a PID file to bypass this check.' }
    foreach ($signal in 'standby.signal', 'recovery.signal') {
        if (Test-Path -LiteralPath (Join-Path $cluster $signal)) { throw 'Import requires a primary cluster outside recovery.' }
    }
    if ((Get-Content -LiteralPath (Join-Path $cluster 'PG_VERSION') -Raw).Trim() -ne '17' -or
        (& (Join-Path $bin 'pg_ctl.exe') --version) -notmatch '^pg_ctl \(PostgreSQL\) 17\.') { throw 'Use PostgreSQL 17 binaries and cluster.' }
    if ((Get-FileHash -LiteralPath $wrapper -Algorithm SHA256).Hash -cne $WinSwSha256.ToUpperInvariant() -or
        [Diagnostics.FileVersionInfo]::GetVersionInfo($wrapper).ProductVersion -notlike '2.12.0*') {
        throw 'Use the reviewed WinSW 2.12.0 binary with its verified SHA-256.'
    }
    $source = Get-Content -LiteralPath (Join-Path $lan 'server.json') -Raw | ConvertFrom-Json
    if ($ServerExecutable) { $source.executable = $ServerExecutable }
    $active = Get-Content -LiteralPath (Join-Path $lan 'active-tls.json') -Raw | ConvertFrom-Json
    if ($source.version -ne 1 -or $active.Version -ne 1) { throw 'Unsupported LAN settings or TLS version.' }
    [void](Get-SafePath $source.executable)
    $database = [Uri]$source.environment.EITMAD_SERVER_DATABASE_URL
    if ($database.Scheme -notin 'postgres', 'postgresql' -or $database.Host -ne '127.0.0.1' -or
        -not $database.UserInfo -or $database.Port -lt 1 -or $database.Port -eq $source.port) {
        throw 'Use a dedicated application role and explicit loopback database port, different from HTTPS.'
    }
    $endpoint = [Uri]$source.endpoint; $address = [Net.IPAddress]::Parse($source.address)
    if ($endpoint.Scheme -ne 'https' -or $endpoint.UserInfo -or $endpoint.AbsolutePath -ne '/' -or
        $endpoint.Query -or $endpoint.Fragment -or $endpoint.Port -ne $source.port -or
        $address.AddressFamily -ne [Net.Sockets.AddressFamily]::InterNetwork) { throw 'Use the retained IPv4 HTTPS endpoint and port.' }
    foreach ($relative in $active.Certificate, $active.PrivateKey) {
        $path = Get-SafePath (Join-Path $lan $relative)
        if (-not $path.StartsWith($lan + '\', [StringComparison]::OrdinalIgnoreCase) -or
            -not (Test-Path -LiteralPath $path -PathType Leaf)) { throw 'TLS files must be inside the LAN directory.' }
    }
    $manifests = Get-SafePath $source.environment.EITMAD_SERVER_UPDATE_MANIFEST_DIRECTORY
    Assert-PlainTree $manifests
    if ($Root.StartsWith($manifests + '\', [StringComparison]::OrdinalIgnoreCase) -or
        $manifests.StartsWith($Root + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'Manifest source and installation directories must be separate.' }
    foreach ($kind in 'Server', 'Postgres') {
        if (Get-Service -Name ($Name + $kind) -ErrorAction SilentlyContinue) { throw 'A service with this name already exists. No data was copied.' }
    }
    Protect-Directory $Root
    foreach ($child in 'app', 'private', 'data', 'postgres', 'manifests', 'logs/server', 'logs/postgres') { Protect-Directory (Join-Path $Root $child) }
    Copy-Item -LiteralPath $source.executable -Destination (Join-Path $Root 'app/eitmad-server.exe')
    Copy-Item -LiteralPath $wrapper -Destination (Join-Path $Root 'app/host-service.exe')
    Copy-Item -LiteralPath $wrapper -Destination (Join-Path $Root 'postgres/host-service.exe')
    foreach ($child in 'bin', 'lib', 'share') {
        Copy-Item -LiteralPath (Join-Path (Split-Path $bin -Parent) $child) -Destination (Join-Path $Root 'postgres') -Recurse
    }
    Get-ChildItem -LiteralPath $cluster -Force | Copy-Item -Destination (Join-Path $Root 'data') -Recurse
    Get-ChildItem -LiteralPath $manifests -Force | Copy-Item -Destination (Join-Path $Root 'manifests') -Recurse
    Copy-Item -LiteralPath (Join-Path $lan $active.Certificate) -Destination (Join-Path $Root 'private/server-cert.pem')
    Copy-Item -LiteralPath (Join-Path $lan $active.PrivateKey) -Destination (Join-Path $Root 'private/server-key.pem')
    Copy-Item -LiteralPath (Join-Path $lan 'trusted-ca.pem') -Destination (Join-Path $Root 'private/trusted-ca.pem')
    [IO.File]::WriteAllText((Join-Path $Root 'private/database-url'), $source.environment.EITMAD_SERVER_DATABASE_URL)
    [IO.File]::WriteAllText((Join-Path $Root 'private/token-key'), $source.environment.EITMAD_SERVER_TOKEN_KEY)
    $environment = @{
        EITMAD_SERVER_DATABASE_URL_FILE = Join-Path $Root 'private/database-url'
        EITMAD_SERVER_TOKEN_KEY_FILE = Join-Path $Root 'private/token-key'
        EITMAD_SERVER_LISTEN = "$($source.address):$($source.port)"
        EITMAD_SERVER_TLS_CERTIFICATE = Join-Path $Root 'private/server-cert.pem'
        EITMAD_SERVER_TLS_PRIVATE_KEY = Join-Path $Root 'private/server-key.pem'
        EITMAD_SERVER_UPDATE_MANIFEST_DIRECTORY = Join-Path $Root 'manifests'
    }
    foreach ($key in 'EITMAD_SERVER_UPDATE_OPERATOR_TENANT_ID', 'EITMAD_SERVER_UPDATE_KEY_ID', 'EITMAD_SERVER_UPDATE_PUBLIC_KEY', 'EITMAD_SERVER_MAX_CONNECTIONS') {
        $property = $source.environment.PSObject.Properties[$key]
        if ($property) { $environment[$key] = [string]$property.Value }
    }
    $settings = [pscustomobject]@{version = 1; name = $Name; root = $Root; endpoint = $source.endpoint
        databasePort = $database.Port; serverPort = $source.port; address = $source.address}
    (New-WrapperXml $settings $environment).Save((Join-Path $Root 'app/host-service.xml'))
    (New-WrapperXml $settings @{} 'Postgres').Save((Join-Path $Root 'postgres/host-service.xml'))
    $settings | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $Root 'host.json') -Encoding UTF8
    # Discard any explicit source ACLs on copied children; all files inherit protected parents.
    foreach ($child in 'app', 'private', 'data', 'postgres', 'manifests') {
        foreach ($item in Get-ChildItem -LiteralPath (Join-Path $Root $child) -Force -Recurse) {
            $acl = if ($item.PSIsContainer) { New-Object Security.AccessControl.DirectorySecurity }
                else { New-Object Security.AccessControl.FileSecurity }
            $acl.SetAccessRuleProtection($false, $false)
            $acl.SetOwner((New-Object Security.Principal.SecurityIdentifier('S-1-5-32-544')))
            Set-Acl -LiteralPath $item.FullName -AclObject $acl
        }
    }
    return $settings
}

function Install-Host($Settings) {
    $root = $Settings.root; $serverId = $Settings.name + 'Server'; $databaseId = $Settings.name + 'Postgres'
    Test-RustConfiguration $root
    foreach ($kind in 'Postgres', 'Server') {
        if (Get-Service -Name ($Settings.name + $kind) -ErrorAction SilentlyContinue) { throw 'A host service is already installed. Use Start or Uninstall first.' }
    }
    # Demand-start avoids booting a partly configured service.
    foreach ($kind in 'Postgres', 'Server') {
        $id = $Settings.name + $kind
        Ensure-EventSource $id
        $directory = if ($kind -eq 'Postgres') { 'postgres' } else { 'app' }
        $arguments = @('create', $id, 'binPath=', ('"' + (Join-Path $root "$directory/host-service.exe") + '"'),
            'start=', 'demand', 'obj=', "NT SERVICE\$id", 'DisplayName=', ($Settings.name + ' ' + $kind))
        if ($kind -eq 'Server') { $arguments += @('depend=', $databaseId) }
        Invoke-Native sc.exe $arguments
    }
    foreach ($id in $databaseId, $serverId) {
        Invoke-Native sc.exe @('sidtype', $id, 'unrestricted')
        Invoke-Native sc.exe @('failure', $id, 'reset=', '3600', 'actions=', 'restart/10000/restart/30000//0')
        Invoke-Native sc.exe @('failureflag', $id, '1')
    }
    Protect-Directory $root @{"NT SERVICE\$serverId" = 'ReadAndExecute'; "NT SERVICE\$databaseId" = 'ReadAndExecute'}
    Protect-Directory (Join-Path $root 'app') @{"NT SERVICE\$serverId" = 'ReadAndExecute'}
    Protect-Directory (Join-Path $root 'postgres') @{"NT SERVICE\$databaseId" = 'ReadAndExecute'}
    Protect-Directory (Join-Path $root 'private') @{"NT SERVICE\$serverId" = 'ReadAndExecute'}
    Protect-Directory (Join-Path $root 'data') @{"NT SERVICE\$databaseId" = 'Modify'}
    Protect-Directory (Join-Path $root 'manifests') @{"NT SERVICE\$serverId" = 'Modify'}
    Protect-Directory (Join-Path $root 'logs/server') @{"NT SERVICE\$serverId" = 'Modify'}
    Protect-Directory (Join-Path $root 'logs/postgres') @{"NT SERVICE\$databaseId" = 'Modify'}
    Assert-InstalledProtection $root $Settings.name
    Invoke-Native sc.exe @('config', $databaseId, 'start=', 'auto')
    Invoke-Native sc.exe @('config', $serverId, 'start=', 'delayed-auto')
}

function Test-ClientAddress([string]$Value) {
    $address = $null
    if (-not [Net.IPAddress]::TryParse($Value, [ref]$address) -or $address.ToString() -cne $Value) { return $false }
    $bytes = $address.GetAddressBytes()
    return $bytes.Length -eq 4 -and ($bytes[0] -eq 10 -or
        ($bytes[0] -eq 172 -and $bytes[1] -ge 16 -and $bytes[1] -le 31) -or
        ($bytes[0] -eq 192 -and $bytes[1] -eq 168))
}

function Get-ClientFirewallRule($Settings) {
    $rule = Get-NetFirewallRule -PolicyStore PersistentStore -Name ($Settings.name + '-Host-Client') -ErrorAction SilentlyContinue
    if ($rule) {
        $application = $rule | Get-NetFirewallApplicationFilter
        if ($rule.Group -cne ('Eitmad host: ' + $Settings.root) -or
            $application.Program -ine (Join-Path $Settings.root 'app/eitmad-server.exe')) {
            throw 'The client firewall rule belongs to another installation. No rule was changed.'
        }
    }
    return $rule
}

function Set-ClientFirewall {
    [CmdletBinding(SupportsShouldProcess)]
    param($Settings, [string]$ClientAddress)
    if (-not $ClientAddress) {
        do {
            $ClientAddress = (Read-Host 'Second PC IPv4 address (for example 192.168.1.20; Enter to cancel)').Trim()
            if (-not $ClientAddress) { Write-Host 'Cancelled. No firewall rule was changed.'; return }
            if (Test-ClientAddress $ClientAddress) { break }
            Write-Host 'Enter one private IPv4 address. Address ranges and subnets are not accepted.'
        } while ($true)
    }
    if (-not (Test-ClientAddress $ClientAddress)) { throw 'Use one exact private IPv4 client address.' }
    if (-not (Test-ClientAddress $Settings.address)) { throw 'The installed host requires a private IPv4 listener before client access can be configured.' }
    $rule = Get-ClientFirewallRule $Settings
    $arguments = @{
        PolicyStore = 'PersistentStore'; Name = $Settings.name + '-Host-Client'
        Enabled = 'True'; Direction = 'Inbound'; Action = 'Allow'; Profile = 'Private'
        Protocol = 'TCP'; LocalAddress = $Settings.address; LocalPort = $Settings.serverPort
        RemoteAddress = $ClientAddress; Program = Join-Path $Settings.root 'app/eitmad-server.exe'
        EdgeTraversalPolicy = 'Block'
    }
    if ($PSCmdlet.ShouldProcess($ClientAddress, 'Allow this client only in the installed host HTTPS rule (replace the previous client)')) {
        if ($rule) { Set-NetFirewallRule @arguments | Out-Null }
        else { New-NetFirewallRule @arguments -DisplayName 'Eitmad host client HTTPS' -Group ('Eitmad host: ' + $Settings.root) | Out-Null }
        Write-Host "Client firewall rule saved for $ClientAddress on Private networks. Other firewall rules still apply."
        Write-Host "Server URL for the second PC: $($Settings.endpoint)"
        Write-Host 'Use the public CA and your own account in the app connection setup. See docs/operations/run-windows-host-services.md.'
    }
}

function Set-HostRelease($Settings, $Release, [string]$Directory, [string]$Action,
    [string]$BackupGeneration, [bool]$RestoreVerified) {
    $root = $Settings.root
    $previousFile = Join-Path $root 'app/release.json'
    $pendingFile = Join-Path $root 'release-pending.json'
    if (-not (Test-Path -LiteralPath $previousFile) -and -not (Test-Path -LiteralPath $pendingFile)) {
        throw 'Installed release inventory is missing. Do not guess a migration baseline.'
    }
    $pending = if (Test-Path -LiteralPath $pendingFile) { Get-Content -LiteralPath $pendingFile -Raw -Encoding UTF8 | ConvertFrom-Json } else { $null }
    $previous = if ($pending) { $pending.previousRelease } else { Get-Content -LiteralPath $previousFile -Raw -Encoding UTF8 | ConvertFrom-Json }
    if ($pending) {
        if ($Release.version -cne $pending.version -or $Release.files.'eitmad-server.exe' -cne $pending.binarySha256) {
            throw 'Repair requires the exact interrupted release. Preserve the pre-migration recovery point.'
        }
    } elseif ($Action -eq 'Repair') {
        if ($Release.version -cne $previous.version -or $Release.files.'eitmad-server.exe' -cne $previous.files.'eitmad-server.exe') {
            throw 'Repair requires the original release and digest. Use Upgrade for a different release.'
        }
    } elseif ([version]$Release.windowsVersion -le [version]$previous.windowsVersion) {
        throw 'Upgrade requires a higher Windows package version. Downgrade requires explicit backup recovery.'
    }
    Assert-AppendOnlyMigrations $previous $Release
    if ($Action -eq 'Upgrade' -and -not $pending -and
        (Get-FileHash -LiteralPath (Join-Path $root 'app/eitmad-server.exe') -Algorithm SHA256).Hash -ine $previous.files.'eitmad-server.exe') {
        throw 'Installed binary differs from its release. Repair the original package before upgrading.'
    }
    foreach ($kind in 'Server', 'Postgres') {
        $service = Get-OwnedService $Settings $kind
        try {
            if (-not $service -or $service.Status -ne 'Stopped') { throw 'Stop both owned services before repair or upgrade.' }
        } finally { Close-Service $service }
    }
    if ($Action -eq 'Upgrade' -and -not $pending) {
        if (-not $BackupGeneration -or -not $RestoreVerified) {
            throw 'Upgrade requires BackupGeneration and RestoreVerified after a successful isolated restore drill.'
        }
        & (Join-Path $Directory 'restore-backup.ps1') -Generation $BackupGeneration -PostgresBin (Join-Path $root 'postgres/bin')
        if (-not $?) { throw 'Recovery generation validation failed.' }
        $marker = Get-Content -LiteralPath (Join-Path $BackupGeneration 'complete.json') -Raw | ConvertFrom-Json
        $hostNode = @($marker.nodes | Where-Object { $_.kind -eq 'server' -and $_.computer -ieq $env:COMPUTERNAME -and $root -in $_.requiredPaths })
        if ($hostNode.Count -ne 1) { throw 'The recovery generation does not contain this installed host.' }
    }
    # Execute only an authenticated copy inside the administrator-owned app tree.
    $source = Join-Path $root 'app/eitmad-server.pending.exe'
    Copy-Item -LiteralPath (Join-Path $Directory 'eitmad-server.exe') -Destination $source
    if ((Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash -ine $Release.files.'eitmad-server.exe') {
        throw 'Staged server digest differs. No installed binary was replaced.'
    }
    Test-RustConfiguration $root $source
    # Retain the exact prior binary and inventory. Never replace the PostgreSQL
    # distribution or settings as part of an application upgrade.
    $history = Join-Path $root ('releases/' + $previous.windowsVersion)
    if (-not (Test-Path -LiteralPath $history)) {
        Protect-Directory (Join-Path $root 'releases')
        Protect-Directory $history
        Copy-Item -LiteralPath $previousFile -Destination (Join-Path $history 'release.json')
        $archiveSource = if ($Action -eq 'Repair' -and -not $pending) { $source } else { Join-Path $root 'app/eitmad-server.exe' }
        Copy-Item -LiteralPath $archiveSource -Destination (Join-Path $history 'eitmad-server.exe')
    }
    if (-not $pending) {
        # Keep reboot and SCM recovery from starting a partly installed release.
        foreach ($kind in 'Server', 'Postgres') { Invoke-Native sc.exe @('config', ($Settings.name + $kind), 'start=', 'demand') }
        $markerStage = Join-Path $root 'release-pending.new.json'
        [pscustomobject]@{version = $Release.version; binarySha256 = $Release.files.'eitmad-server.exe';
            previousVersion = $previous.version; previousRelease = $previous; backupGeneration = $BackupGeneration} |
            ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $markerStage -Encoding UTF8
        [IO.File]::Move($markerStage, $pendingFile)
    }
    $destination = Join-Path $root 'app/eitmad-server.exe'
    if (Test-Path -LiteralPath $destination) { [IO.File]::Replace($source, $destination, [NullString]::Value) }
    else { [IO.File]::Move($source, $destination) }
    try {
        $database = Get-OwnedService $Settings 'Postgres'
        try {
            if (@(Get-NetTCPConnection -State Listen -LocalPort $Settings.databasePort -ErrorAction SilentlyContinue).Count) {
                throw 'Database port is already occupied. No unrelated process was stopped.'
            }
            $database.Start()
            $database.WaitForStatus([ServiceProcess.ServiceControllerStatus]::Running, [TimeSpan]::FromSeconds(60))
        } finally { Close-Service $database }
        # Rust owns migration validation and application. A failure is never hidden
        # by putting an older binary over a database that might have migrated.
        Test-RustConfiguration $root (Join-Path $root 'app/eitmad-server.exe') 'migrate'
        $server = Get-OwnedService $Settings 'Server'
        try {
            if (@(Get-NetTCPConnection -State Listen -LocalPort $Settings.serverPort -ErrorAction SilentlyContinue).Count) {
                throw 'HTTPS port is already occupied. No unrelated process was stopped.'
            }
            $server.Start()
            $server.WaitForStatus([ServiceProcess.ServiceControllerStatus]::Running, [TimeSpan]::FromSeconds(60))
        } finally { Close-Service $server }
        Wait-Ready $Settings
        $inventoryStage = Join-Path $root 'app/release.pending.json'
        $Release | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $inventoryStage -Encoding UTF8
        if (Test-Path -LiteralPath $previousFile) { [IO.File]::Replace($inventoryStage, $previousFile, [NullString]::Value) }
        else { [IO.File]::Move($inventoryStage, $previousFile) }
        Invoke-Native sc.exe @('config', ($Settings.name + 'Postgres'), 'start=', 'auto')
        Invoke-Native sc.exe @('config', ($Settings.name + 'Server'), 'start=', 'delayed-auto')
        Remove-Item -LiteralPath $pendingFile
        Write-Host 'Release installed; explicit Rust migrations and verified HTTPS readiness passed. Verify authenticated client work before reopening access.'
    } catch {
        Stop-Host $Settings
        throw 'Release failed. Services are stopped; pending recovery details and all data are retained. Repair the exact release or restore the complete recovery point.'
    }
}

function Invoke-EitmadHost {
    [CmdletBinding()]
    param(
        [ValidateSet('Install', 'Start', 'Stop', 'Status', 'Uninstall', 'Firewall', 'Repair', 'Upgrade')][string]$Action = 'Status',
        [string]$StateDirectory = "$env:ProgramData\EitmadHost",
        [ValidatePattern('^[A-Za-z][A-Za-z0-9]{0,31}$')][string]$Name = 'Eitmad',
        [string]$ClientAddress,
        [switch]$WhatIf,
        [string]$LanDirectory, [string]$ClusterDirectory, [string]$PostgresBin,
        [string]$WinSwExecutable, [string]$WinSwSha256, [string]$ServerExecutable,
        [string]$ReleaseDirectory, [string]$ExpectedReleaseSha256, [string]$Publisher,
        [switch]$AllowUnsignedValidation, [string]$BackupGeneration, [switch]$RestoreVerified
    )
    if ($WhatIf -and $Action -ne 'Firewall') { throw 'WhatIf is supported only for the Firewall action.' }
    if ($ClientAddress -and $Action -ne 'Firewall') { throw 'ClientAddress is supported only for the Firewall action.' }
    $root = Get-SafePath $StateDirectory
    Assert-Administrator
    Assert-ProtectedParents $root
    $release = $null
    if ($ReleaseDirectory) {
        Import-Module (Join-Path $ReleaseDirectory 'package.psm1') -Force
        $release = Read-EitmadPackage -Directory $ReleaseDirectory -Target windows-server `
            -ExpectedReleaseSha256 $ExpectedReleaseSha256 -Publisher $Publisher -AllowUnsignedValidation:$AllowUnsignedValidation
        Assert-WindowsPrerequisites
        $ServerExecutable = Join-Path $ReleaseDirectory 'eitmad-server.exe'
    }
    if ($Action -eq 'Install' -and -not (Test-Path -LiteralPath (Join-Path $root 'host.json'))) {
        $settings = Initialize-Host $root $Name $LanDirectory $ClusterDirectory $PostgresBin $WinSwExecutable $WinSwSha256 $ServerExecutable
    } else {
        $allowMissing = $Action -in 'Status', 'Stop', 'Uninstall' -or ($Action -eq 'Repair' -and $null -ne $release)
        Assert-InstalledProtection $root $Name -AllowMissingServerBinary:$allowMissing
        $settings = Read-HostManifest $root $Name
    }
    Assert-PlainTree $root
    $lock = [IO.File]::Open((Join-Path $root 'host.lock'), 'OpenOrCreate', 'ReadWrite', 'None')
    try {
        $pending = Join-Path $root 'release-pending.json'
        if ((Test-Path -LiteralPath $pending) -and $Action -notin 'Status', 'Stop', 'Uninstall', 'Repair') {
            throw 'An interrupted release is pending. Preserve state, inspect release-pending.json, then Repair with that exact package.'
        }
        if ($Action -in 'Repair', 'Upgrade') {
            if (-not $release) { throw 'Use install-server.ps1 with the verified release inventory.' }
            Set-HostRelease $settings $release $ReleaseDirectory $Action $BackupGeneration $RestoreVerified
        } elseif ($Action -eq 'Firewall') {
            Set-ClientFirewall $settings $ClientAddress -WhatIf:$WhatIf
        } elseif ($Action -eq 'Install') {
            if ($release) {
                $digest = (Get-FileHash -LiteralPath (Join-Path $root 'app/eitmad-server.exe') -Algorithm SHA256).Hash
                if ($digest -ine $release.files.'eitmad-server.exe') { throw 'Retained binary differs. Use Repair; no data was replaced.' }
            }
            Install-Host $settings
            if ($release) { $release | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $root 'app/release.json') -Encoding UTF8 }
            Write-Host 'Services installed for unattended boot. Run Start to verify TLS readiness. Data and source inputs are retained.'
        } elseif ($Action -eq 'Status') {
            $database = Get-OwnedService $settings 'Postgres'; $server = Get-OwnedService $settings 'Server'
            try { [pscustomobject]@{
                PostgreSQL = $(if ($database) { [string]$database.Status } else { 'NotInstalled' })
                Server = $(if ($server) { [string]$server.Status } else { 'NotInstalled' })
                Live = [bool]($server -and $server.Status -eq 'Running' -and (Test-Endpoint $settings.endpoint 'livez' (Join-Path $root 'private/trusted-ca.pem')))
                Ready = [bool]($database -and $server -and $database.Status -eq 'Running' -and $server.Status -eq 'Running' -and (Test-Endpoint $settings.endpoint 'readyz' (Join-Path $root 'private/trusted-ca.pem')))
                DataDirectory = Join-Path $root 'data'
            } } finally { Close-Service $server; Close-Service $database }
        } elseif ($Action -eq 'Start') {
            foreach ($kind in 'Postgres', 'Server') {
                $service = Get-OwnedService $settings $kind
                if (-not $service) { throw 'A service is not installed. Run Install.' }
                try { if ($service.Status -eq 'Stopped') {
                    $port = if ($kind -eq 'Postgres') { $settings.databasePort } else { $settings.serverPort }
                    if (@(Get-NetTCPConnection -State Listen -LocalPort $port -ErrorAction SilentlyContinue).Count) {
                        throw "Port $port is already in use. No unrelated process was stopped."
                    }
                    $service.Start()
                    $service.WaitForStatus([ServiceProcess.ServiceControllerStatus]::Running, [TimeSpan]::FromSeconds(60))
                } } finally { Close-Service $service }
            }
            Wait-Ready $settings
            Write-Host 'Host ready: services are running and HTTPS /readyz returned 204.'
        } elseif ($Action -eq 'Stop') {
            Stop-Host $settings
            Write-Host 'Services stopped in dependency order. Data is retained.'
        } elseif ($Action -eq 'Uninstall') {
            $clientRule = Get-ClientFirewallRule $settings
            Stop-Host $settings
            if ($clientRule) { $clientRule | Remove-NetFirewallRule }
            foreach ($kind in 'Server', 'Postgres') {
                $service = Get-OwnedService $settings $kind
                if ($service) {
                    Close-Service $service
                    Invoke-Native sc.exe @('delete', ($settings.name + $kind))
                }
            }
            Write-Host 'Services uninstalled. All data, credentials, certificates, manifests, binaries, and logs are retained.'
        }
    } finally { $lock.Dispose() }
}

Export-ModuleMember -Function Invoke-EitmadHost
