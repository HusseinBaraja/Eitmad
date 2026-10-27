Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
Import-Module (Join-Path $PSScriptRoot 'host.psm1') -Force
$script:HostTools = Get-Module host

function Assert-TlsControlPath([string]$Path) {
    $full = [IO.Path]::GetFullPath($Path)
    for ($part = $full; $part; $part = Split-Path $part -Parent) {
        $item = Get-Item -LiteralPath $part -Force
        if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'TLS control paths cannot use reparse points.' }
        $acl = Get-Acl -LiteralPath $part
        $trusted = @('S-1-5-18', 'S-1-5-32-544', [Security.Principal.WindowsIdentity]::GetCurrent().User.Value,
            'S-1-5-80-956008885-3418522649-1831038044-1853292631-2271478464')
        if ($acl.GetOwner([Security.Principal.SecurityIdentifier]).Value -notin $trusted) { throw 'Untrusted TLS control owner.' }
        foreach ($rule in $acl.GetAccessRules($true, $true, [Security.Principal.SecurityIdentifier])) {
            if ($rule.AccessControlType -ne 'Allow' -or $rule.IdentityReference.Value -in $trusted -or
                ($rule.PropagationFlags -band [Security.AccessControl.PropagationFlags]::InheritOnly)) { continue }
            $write = [Security.AccessControl.FileSystemRights]::Delete -bor
                [Security.AccessControl.FileSystemRights]::DeleteSubdirectoriesAndFiles -bor
                [Security.AccessControl.FileSystemRights]::ChangePermissions -bor [Security.AccessControl.FileSystemRights]::TakeOwnership
            if ($part -eq $full -or ($rule.FileSystemRights -band $write)) { throw 'TLS controls or their parents permit untrusted access.' }
        }
    }
}

function Assert-TlsControl([string]$Path) {
    # Unlike the service's readable private directory, CA material permits no service readers.
    Assert-TlsControlPath $Path
    & $script:HostTools { param($p) Assert-PlainTree $p } $Path
    foreach ($item in @(Get-Item -LiteralPath $Path -Force) + @(Get-ChildItem -LiteralPath $Path -Recurse -Force)) {
        $acl = Get-Acl -LiteralPath $item.FullName
        foreach ($rule in $acl.GetAccessRules($true, $true, [Security.Principal.SecurityIdentifier])) {
            if ($rule.AccessControlType -eq 'Allow' -and $rule.IdentityReference.Value -notin 'S-1-5-18','S-1-5-32-544') {
                throw 'TLS control requires Administrators and SYSTEM only.'
            }
        }
    }
}

function Invoke-TlsTool([string]$Root, [string]$Action, $Payload) {
    $executable = Join-Path $Root 'renewal/tool/Eitmad.LanCertificate.exe'
    $start = New-Object Diagnostics.ProcessStartInfo
    $start.FileName = $executable
    $start.Arguments = 'mechanic ' + $Action + ' "' + (Join-Path $Root 'renewal/authority') + '"'
    $start.UseShellExecute = $false; $start.CreateNoWindow = $true
    $start.RedirectStandardInput = $true; $start.RedirectStandardOutput = $true; $start.RedirectStandardError = $true
    $process = [Diagnostics.Process]::Start($start)
    try {
        $stdout = $process.StandardOutput.ReadToEndAsync()
        $stderr = $process.StandardError.BaseStream.CopyToAsync([IO.Stream]::Null)
        $process.StandardInput.Write(($Payload | ConvertTo-Json -Depth 8 -Compress))
        $process.StandardInput.Close()
        if (-not $process.WaitForExit(15000)) {
            $process.Kill(); [void]$process.WaitForExit(10000)
            throw 'TLS tool deadline elapsed.'
        }
        $response = $stdout.GetAwaiter().GetResult()
        [void]$stderr.GetAwaiter().GetResult()
        if ($response.Length -gt 16384) { throw 'Invalid TLS observation.' }
        $result = $response | ConvertFrom-Json
        if ($result.outcome -eq 'credentialUnavailable') { throw 'credentialUnavailable' }
        if ($process.ExitCode -ne 0 -or $result.outcome -ne 'succeeded') { throw 'TLS mechanic failed.' }
        return $result.result
    } finally { $process.Dispose() }
}

function Get-TlsPairPath([string]$Root, [string]$Relative) {
    if ($Relative -cnotmatch '^private/(?:tls/[a-f0-9]{32}/)?server-(?:cert|key)\.pem$') { throw 'Invalid TLS pair path.' }
    $path = Join-Path $Root $Relative
    & $script:HostTools { param($p) [void](Get-SafePath $p) } $path
    return $path
}

function Read-TlsPair([string]$Root, [string]$Certificate, [string]$Key) {
    $cert = Get-TlsPairPath $Root $Certificate; $private = Get-TlsPairPath $Root $Key
    $validity = Invoke-TlsTool $Root 'inspect' @{certificate=$cert; privateKey=$private; authority=(Join-Path $Root 'private/trusted-ca.pem')}
    return @{certificate=$Certificate; privateKey=$Key; notBefore=$validity.NotBefore; notAfter=$validity.NotAfter
        caNotAfter=$validity.CaNotAfter; fingerprint=$validity.Fingerprint}
}

function Set-TlsPair([string]$Root, $Pair) {
    $cert = Get-TlsPairPath $Root $Pair.certificate; $key = Get-TlsPairPath $Root $Pair.privateKey
    $validated = Read-TlsPair $Root $Pair.certificate $Pair.privateKey
    if ($validated.fingerprint -cne $Pair.fingerprint) { throw 'TLS pair changed.' }
    $path = Join-Path $Root 'app/host-service.xml'
    $xml = [xml](Get-Content -LiteralPath $path -Raw)
    foreach ($name in 'EITMAD_SERVER_TLS_CERTIFICATE','EITMAD_SERVER_TLS_PRIVATE_KEY') {
        $entry = @($xml.service.env | Where-Object name -CEQ $name)
        if ($entry.Count -ne 1) { throw 'Invalid TLS environment.' }
        $value = if ($name -eq 'EITMAD_SERVER_TLS_CERTIFICATE') { $cert } else { $key }
        $entry[0].SetAttribute('value', $value)
    }
    # Both references change in one same-volume replacement. Immutable pairs are never overwritten.
    $temporary = Join-Path $Root ('app/' + [Guid]::NewGuid().ToString('N') + '.xml')
    $stream = [IO.File]::Open($temporary, 'CreateNew', 'Write', 'None')
    try { $xml.Save($stream); $stream.Flush($true) } finally { $stream.Dispose() }
    [IO.File]::Replace($temporary, $path, [NullString]::Value)
}

function Initialize-TlsControl([string]$Root, $Settings, $Request) {
    $source = & $script:HostTools { param($p) Get-SafePath $p } $Request.lanDirectory
    & $script:HostTools { param($p) Assert-PlainTree $p } $source
    $identity = Get-Content -LiteralPath (Join-Path $source 'identity.json') -Raw | ConvertFrom-Json
    if ($identity.Version -ne 1 -or $Settings.endpoint -cne "https://$($identity.ServerName):$($Settings.serverPort)/") {
        throw 'The retained identity differs from the installed endpoint.'
    }
    $trust = [IO.File]::ReadAllText((Join-Path $Root 'private/trusted-ca.pem'))
    if ($trust -cne [IO.File]::ReadAllText((Join-Path $source 'trusted-ca.pem'))) { throw 'Retained CA differs from installed trust.' }
    $control = Join-Path $Root 'renewal'
    if (-not (Test-Path -LiteralPath $control)) { & $script:HostTools { param($p) Protect-Directory $p } $control }
    Assert-TlsControl $control
    foreach ($child in 'authority','tool') {
        $path = Join-Path $control $child
        if (-not (Test-Path -LiteralPath $path)) { & $script:HostTools { param($p) Protect-Directory $p } $path }
    }
    $authority = Join-Path $control 'authority'
    foreach ($name in 'identity.json','trusted-ca.pem','ca-key.encrypted.pem') {
        $target = Join-Path $authority $name
        $bytes = [IO.File]::ReadAllBytes((Join-Path $source $name))
        if (Test-Path -LiteralPath $target) {
            if ((Get-FileHash -LiteralPath $target).Hash -cne (Get-FileHash -LiteralPath (Join-Path $source $name)).Hash) {
                throw 'Retained authority cannot be replaced.'
            }
        } else { [IO.File]::WriteAllBytes($target, $bytes) }
    }
    $tool = & $script:HostTools { param($p) Get-SafePath $p } $Request.certificateExecutable
    if ([IO.Path]::GetFileName($tool) -cne 'Eitmad.LanCertificate.exe') { throw 'Use the reviewed certificate executable.' }
    Assert-TlsControlPath $tool
    # Retain the existing .NET implementation and its runtime metadata under protected control.
    $toolDirectory = Split-Path $tool -Parent
    & $script:HostTools { param($p) Assert-PlainTree $p } $toolDirectory
    if ([IO.Path]::GetFullPath($toolDirectory) -ine [IO.Path]::GetFullPath((Join-Path $control 'tool'))) {
        # Offline packages are self-contained. Keep their complete app-local runtime too.
        Get-ChildItem -LiteralPath $toolDirectory -Force | Copy-Item -Destination (Join-Path $control 'tool') -Recurse -Force
    }
    $password = $null
    try {
        $property = $Request.PSObject.Properties['password']
        if ($property) { $password = $property.Value }
        else {
            try { $password = [IO.File]::ReadAllText((Join-Path $source 'certificate-password')) }
            catch { throw 'credentialUnavailable' }
        }
        if (-not $password) { throw 'credentialUnavailable' }
        [void](Invoke-TlsTool $Root 'retain' @{password=$password})
    } finally { $password = $null }
    if (-not (Test-Path -LiteralPath (Join-Path $authority 'unlock.dpapi'))) { throw 'credentialUnavailable' }
    foreach ($item in Get-ChildItem -LiteralPath $control -Recurse -Force) {
        $acl = if ($item.PSIsContainer) { New-Object Security.AccessControl.DirectorySecurity }
            else { New-Object Security.AccessControl.FileSecurity }
        $acl.SetAccessRuleProtection($false, $false)
        $acl.SetOwner((New-Object Security.Principal.SecurityIdentifier('S-1-5-32-544')))
        Set-Acl -LiteralPath $item.FullName -AclObject $acl
    }
    # The scheduled backend and mechanics must survive removal of the package staging directory.
    $scripts = Join-Path $Root 'app/deploy/windows'
    & $script:HostTools { param($p) Protect-Directory $p } $scripts
    foreach ($name in 'tls-renewal.ps1','tls-renewal.psm1','host.psm1') {
        $target = Join-Path $scripts $name
        if ([IO.Path]::GetFullPath((Join-Path $PSScriptRoot $name)) -ine [IO.Path]::GetFullPath($target)) {
            Copy-Item -LiteralPath (Join-Path $PSScriptRoot $name) -Destination $target -Force
        }
        $acl = New-Object Security.AccessControl.FileSecurity
        $acl.SetAccessRuleProtection($false, $false)
        $acl.SetOwner((New-Object Security.Principal.SecurityIdentifier('S-1-5-32-544')))
        Set-Acl -LiteralPath $target -AclObject $acl
    }
    Assert-TlsControl $scripts
    Assert-TlsControl $control
}

function Register-TlsSchedule([string]$Root, $Settings, [int]$IntervalSeconds) {
    $backend = Join-Path $Root 'app/eitmad-server.exe'
    $name = $Settings.name + '-TlsRenewal'
    $arguments = 'windows-tls tick "' + $Root + '"'
    $existing = Get-ScheduledTask -TaskName $name -ErrorAction SilentlyContinue
    if ($existing) {
        if (@($existing.Actions).Count -ne 1 -or $existing.Actions.Execute -cne $backend -or
            $existing.Actions.Arguments -cne $arguments -or $existing.Principal.UserId -notin 'SYSTEM','S-1-5-18') {
            throw 'The renewal task belongs to another configuration.'
        }
    }
    # Rust supplies the renewal check cadence.
    $action = New-ScheduledTaskAction -Execute $backend -Argument $arguments
    $trigger = New-ScheduledTaskTrigger -Once -At (Get-Date).AddMinutes(1) -RepetitionInterval (New-TimeSpan -Seconds $IntervalSeconds)
    $taskSettings = New-ScheduledTaskSettingsSet -StartWhenAvailable -MultipleInstances IgnoreNew -ExecutionTimeLimit (New-TimeSpan -Minutes 15)
    Register-ScheduledTask -TaskName $name -Action $action -Trigger $trigger -Settings $taskSettings -User SYSTEM -RunLevel Highest -Force | Out-Null
}

function Read-TlsSchedule([string]$Root, $Settings, [int]$IntervalSeconds) {
    $task = Get-ScheduledTask -TaskName ($Settings.name + '-TlsRenewal') -ErrorAction SilentlyContinue
    if (-not $task -or @($task.Triggers).Count -ne 1) { throw 'Renewal task is missing or changed.' }
    $interval = [Xml.XmlConvert]::ToTimeSpan($task.Triggers[0].Repetition.Interval).TotalSeconds
    if (-not $task -or @($task.Actions).Count -ne 1 -or @($task.Triggers).Count -ne 1 -or
        $task.Actions.Execute -cne (Join-Path $Root 'app/eitmad-server.exe') -or
        $task.Actions.Arguments -cne ('windows-tls tick "' + $Root + '"') -or
        $task.Principal.UserId -notin 'SYSTEM','S-1-5-18' -or $task.State -eq 'Disabled' -or
        -not $task.Settings.StartWhenAvailable -or $task.Settings.MultipleInstances -ne 'IgnoreNew' -or
        $interval -ne $IntervalSeconds) { throw 'Renewal schedule is missing, disabled, or changed.' }
}

function Restart-TlsHost($Settings) {
    & $script:HostTools { param($s)
        $service = Get-OwnedService $s 'Server'
        if (-not $service) { throw 'The owned service is missing.' }
        try {
            if ($service.Status -ne 'Stopped') {
                $service.Stop()
                $service.WaitForStatus([ServiceProcess.ServiceControllerStatus]::Stopped, [TimeSpan]::FromSeconds(60))
            }
        } finally { Close-Service $service }
        Start-HostService $s 'Server'
    } $Settings
}

function Get-TlsProbeAddress([string]$Root, $Settings) {
    $xml = [xml](Get-Content -LiteralPath (Join-Path $Root 'app/host-service.xml') -Raw)
    $listen = @($xml.service.env | Where-Object name -CEQ 'EITMAD_SERVER_LISTEN')
    if ($listen.Count -ne 1 -or $listen[0].value -cnotmatch '^([0-9.]+):([0-9]{1,5})$') {
        throw 'Invalid installed HTTPS listener.'
    }
    $address = $null
    if (-not [Net.IPAddress]::TryParse($Matches[1], [ref]$address) -or
        $address.AddressFamily -ne [Net.Sockets.AddressFamily]::InterNetwork -or
        $address.ToString() -cne $Matches[1] -or [int]$Matches[2] -ne $Settings.serverPort -or
        [int]$Matches[2] -ne ([Uri]$Settings.endpoint).Port) { throw 'Invalid installed HTTPS listener.' }
    # The installed wildcard listener is reachable locally after DHCP changes. This route
    # does not replace the retained endpoint used for TLS authentication and HTTP Host.
    if ($address.Equals([Net.IPAddress]::Any)) { return '127.0.0.1' }
    if ([Net.IPAddress]::IsLoopback($address)) { return $address.ToString() }
    foreach ($adapter in [Net.NetworkInformation.NetworkInterface]::GetAllNetworkInterfaces()) {
        foreach ($local in $adapter.GetIPProperties().UnicastAddresses) {
            if ($local.Address.Equals($address)) { return $address.ToString() }
        }
    }
    throw 'The installed HTTPS listener is not a current local address.'
}

function Invoke-TlsMechanic([string]$Action, [string]$StateDirectory, $Request, [string]$Backend) {
    $root = & $script:HostTools { param($p) Get-SafePath $p } $StateDirectory
    $settings = & $script:HostTools { param($p)
        Assert-Administrator; Assert-ProtectedParents $p; Assert-PlainTree $p
        $manifest = Get-Content -LiteralPath (Join-Path $p 'host.json') -Raw | ConvertFrom-Json
        if ($manifest.name -cnotmatch '^[A-Za-z][A-Za-z0-9]{0,31}$') { throw 'Invalid service name.' }
        Assert-InstalledProtection $p $manifest.name
        Read-HostManifest $p $manifest.name
    } $root
    if (Test-Path -LiteralPath (Join-Path $root 'release-pending.json')) { throw 'Resolve the pending release first.' }
    if (Test-Path -LiteralPath (Join-Path $root 'renewal')) { Assert-TlsControl (Join-Path $root 'renewal') }
    if ($Action -eq 'Guard') { return @{} }
    switch ($Action) {
        Configure {
            if (-not $Backend -or (Get-FileHash -LiteralPath $Backend).Hash -cne
                (Get-FileHash -LiteralPath (Join-Path $root 'app/eitmad-server.exe')).Hash) {
                throw 'Install the matching current Rust server before enabling renewal.'
            }
            Initialize-TlsControl $root $settings $Request; return @{}
        }
        Schedule { Register-TlsSchedule $root $settings $Request.intervalSeconds; return @{} }
        ScheduleStatus { Read-TlsSchedule $root $settings $Request.intervalSeconds; return @{} }
        Observe {
            $xml = [xml](Get-Content -LiteralPath (Join-Path $root 'app/host-service.xml') -Raw)
            $paths = @{}
            foreach ($name in 'EITMAD_SERVER_TLS_CERTIFICATE','EITMAD_SERVER_TLS_PRIVATE_KEY') {
                $entries = @($xml.service.env | Where-Object name -CEQ $name)
                if ($entries.Count -ne 1 -or -not $entries[0].value.StartsWith($root + '\', [StringComparison]::OrdinalIgnoreCase)) {
                    throw 'Invalid installed TLS configuration.'
                }
                $paths[$name] = $entries[0].value.Substring($root.Length + 1).Replace('\','/')
            }
            $pair = Read-TlsPair $root $paths.EITMAD_SERVER_TLS_CERTIFICATE $paths.EITMAD_SERVER_TLS_PRIVATE_KEY
            $running = & $script:HostTools { param($s)
                $service = Get-OwnedService $s 'Server'
                try { return [bool]($service -and $service.Status -eq 'Running') } finally { Close-Service $service }
            } $settings
            return @{pair=$pair; running=$running}
        }
        Stage {
            $active = Invoke-TlsTool $root 'stage' @{}
            if ($active.Version -ne 1 -or $active.Certificate -cnotmatch '^tls/[a-f0-9]{32}/server-cert.pem$' -or
                $active.PrivateKey -cne $active.Certificate.Replace('server-cert.pem','server-key.pem')) { throw 'Invalid staged bundle.' }
            $bundle = Split-Path $active.Certificate -Parent
            $target = Join-Path $root ('private/' + $bundle)
            & $script:HostTools { param($p,$n) Protect-Directory $p @{("NT SERVICE\" + $n + 'Server')='ReadAndExecute'} } $target $settings.name
            foreach ($relative in $active.Certificate,$active.PrivateKey) {
                $bytes = [IO.File]::ReadAllBytes((Join-Path $root ('renewal/authority/' + $relative)))
                $stream = [IO.File]::Open((Join-Path $root ('private/' + $relative)), 'CreateNew', 'Write', 'None')
                try { $stream.Write($bytes, 0, $bytes.Length); $stream.Flush($true) } finally { $stream.Dispose() }
            }
            return (Read-TlsPair $root ('private/' + $active.Certificate) ('private/' + $active.PrivateKey))
        }
        Activate { Set-TlsPair $root $Request.pair; return @{} }
        Restart {
            Restart-TlsHost $settings
            return @{}
        }
        Ready {
            # Bounded retry permits Rust/database startup. Every attempt validates CA, name,
            # expiry, the exact leaf fingerprint, and HTTP 204. There is no validation bypass.
            $address = Get-TlsProbeAddress $root $settings
            $deadline = [DateTime]::UtcNow.AddSeconds(150)
            do {
                try {
                    [void](Invoke-TlsTool $root 'probe' @{endpoint=$settings.endpoint; address=$address
                        authority=(Join-Path $root 'private/trusted-ca.pem'); fingerprint=$Request.pair.fingerprint})
                    return @{}
                } catch { Start-Sleep -Milliseconds 500 }
            } while ([DateTime]::UtcNow -lt $deadline)
            throw 'Validated HTTPS readiness failed.'
        }
        default { throw 'Unknown TLS mechanic.' }
    }
}

Export-ModuleMember -Function Invoke-TlsMechanic
