Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$script:HostAdapter = Import-Module (Join-Path $PSScriptRoot 'host.psm1') -Force -PassThru

function Assert-RemovalTree([string]$Path, [string]$Parent) {
    $root = [IO.Path]::GetFullPath($Parent).TrimEnd('\')
    $full = [IO.Path]::GetFullPath($Path).TrimEnd('\')
    if (-not $full.StartsWith($root + '\', [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Removal target is outside its expected parent.'
    }
    & $script:HostAdapter { param($p) [void](Get-SafePath $p); if (Test-Path -LiteralPath $p) { Assert-PlainTree $p } } $full
    return $full
}

function Assert-RemovalOwner([string]$Path) {
    & $script:HostAdapter { param($p) Assert-ProtectedParents $p } $Path
    if (Test-Path -LiteralPath $Path) {
        $acl = Get-Acl -LiteralPath $Path
        $owner = $acl.GetOwner([Security.Principal.SecurityIdentifier]).Value
        if ($owner -notin 'S-1-5-18', 'S-1-5-32-544') { throw 'Machine removal requires an administrator-owned directory.' }
        $write = [Security.AccessControl.FileSystemRights]::Write -bor [Security.AccessControl.FileSystemRights]::Delete -bor
            [Security.AccessControl.FileSystemRights]::DeleteSubdirectoriesAndFiles -bor
            [Security.AccessControl.FileSystemRights]::ChangePermissions -bor [Security.AccessControl.FileSystemRights]::TakeOwnership
        foreach ($rule in $acl.GetAccessRules($true, $true, [Security.Principal.SecurityIdentifier])) {
            if ($rule.AccessControlType -eq 'Allow' -and $rule.IdentityReference.Value -notin 'S-1-5-18', 'S-1-5-32-544' -and
                -not ($rule.PropagationFlags -band [Security.AccessControl.PropagationFlags]::InheritOnly) -and
                ($rule.FileSystemRights -band $write)) { throw 'Machine removal root permits untrusted changes.' }
        }
    }
}

function New-RetentionDirectory([string]$Parent, [bool]$Machine) {
    $root = Assert-RemovalTree (Join-Path $Parent 'EitmadRetained') $Parent
    if ($Machine) {
        Assert-RemovalOwner $root
        & $script:HostAdapter { param($p) Protect-Directory $p } $root
    } else {
        # Create private retention storage atomically under the original user.
        $acl = New-Object Security.AccessControl.DirectorySecurity
        $acl.SetAccessRuleProtection($true, $false)
        $user = [Security.Principal.WindowsIdentity]::GetCurrent().User
        $acl.SetOwner($user)
        foreach ($sid in @($user, (New-Object Security.Principal.SecurityIdentifier('S-1-5-18')))) {
            $acl.AddAccessRule((New-Object Security.AccessControl.FileSystemAccessRule($sid, 'FullControl', 'ContainerInherit,ObjectInherit', 'None', 'Allow')))
        }
        if (-not (Test-Path -LiteralPath $root)) { [void][IO.Directory]::CreateDirectory($root, $acl) }
    }
    $destination = Assert-RemovalTree (Join-Path $root ([Guid]::NewGuid().ToString('N'))) $root
    if ($Machine) { & $script:HostAdapter { param($p) Protect-Directory $p } $destination }
    else { [void][IO.Directory]::CreateDirectory($destination, $acl) }
    return $destination
}

function Move-RetainedTree([string]$Source, [string]$Parent, [string]$Destination) {
    $sourcePath = Assert-RemovalTree $Source $Parent
    $targetPath = Assert-RemovalTree (Join-Path $Destination ([IO.Path]::GetFileName($sourcePath))) $Destination
    if (Test-Path -LiteralPath $sourcePath) {
        # Rename on the same volume. Never merge, overwrite, copy a live database,
        # follow junctions, or delete an existing retained data set.
        Move-Item -LiteralPath $sourcePath -Destination $targetPath -ErrorAction Stop
    }
}

function Assert-DesktopStopped {
    if (Get-Process -Name Eitmad.WindowsShell, eitmad-engine-cli, EitmadSetup -ErrorAction SilentlyContinue) {
        throw 'Close Eitmad and guided setup before uninstalling.'
    }
}

function Remove-DesktopRegistration([string]$Publisher) {
    $packages = @(Get-AppxPackage -Name Eitmad.Desktop)
    if ($packages.Count -gt 1) { throw 'More than one desktop package is registered for this user.' }
    if ($packages.Count -eq 1 -and $packages[0].Publisher -cne $Publisher) { throw 'Installed desktop publisher differs. No package was removed.' }
    foreach ($package in $packages) { Remove-AppxPackage -Package $package.PackageFullName -ErrorAction Stop }
    if (Get-AppxPackage -Name Eitmad.Desktop) { throw 'Desktop package removal did not complete.' }
}

function Remove-UserInstallation([string]$Parent = [Environment]::GetFolderPath('LocalApplicationData'), [string]$Publisher) {
    Assert-DesktopStopped
    $targets = @((Join-Path $parent 'Eitmad'), (Join-Path $parent 'EitmadSetupProgress'))
    $targets += @(Get-ChildItem -LiteralPath $parent -Directory -Force | Where-Object {
        $_.Name -cmatch '^EitmadSetupUpdate-(server|client)-[a-fA-F0-9]{64}-[a-fA-F0-9]{64}$'
    } | Select-Object -ExpandProperty FullName)
    foreach ($target in $targets) { [void](Assert-RemovalTree $target $parent) }
    Remove-DesktopRegistration $Publisher
    $retained = New-RetentionDirectory $parent $false
    # This dedicated directory contains executable installer caches, never the
    # authority store. Keep user records and credential references together.
    $cache = Assert-RemovalTree (Join-Path $parent 'Eitmad/Installers') (Join-Path $parent 'Eitmad')
    if (Test-Path -LiteralPath $cache) { Remove-Item -LiteralPath $cache -Recurse -Force -ErrorAction Stop }
    foreach ($target in $targets) { Move-RetainedTree $target $parent $retained }
    # Native credentials remain usable only by references in the retained data.
    # A fresh install creates new references; do not erase credentials needed to
    # recover user data, and never enumerate or remove another application's keys.
}

function Remove-HostRegistrations([string]$Root) {
    & $script:HostAdapter {
        param($root)
        $settings = [pscustomobject]@{name='Eitmad'; root=$root}
        if (Test-Path -LiteralPath (Join-Path $root 'host.json')) { $settings = Read-HostManifest $root 'Eitmad' }
        # Validate every registration before the first stop or deletion. This
        # also supports an interrupted install with no completed host manifest.
        foreach ($kind in 'Server', 'Postgres') { $service = Get-OwnedService $settings $kind; Close-Service $service }
        $clientRule = Get-ClientFirewallRule $settings
        $discoveryRule = Get-DiscoveryFirewallRule $settings
        $task = Get-ScheduledTask -TaskPath '\' -TaskName 'Eitmad-TlsRenewal' -ErrorAction SilentlyContinue
        if ($task -and (@($task.Actions).Count -ne 1 -or
            $task.Actions.Execute -cne (Join-Path $root 'app/eitmad-server.exe') -or
            $task.Actions.Arguments -cne ('windows-tls tick "' + $root + '"') -or
            $task.Principal.UserId -notin 'SYSTEM', 'S-1-5-18')) { throw 'The renewal task belongs to another installation.' }
        if ($task) {
            Stop-ScheduledTask -TaskPath '\' -TaskName 'Eitmad-TlsRenewal' -ErrorAction Stop
            $deadline = [DateTime]::UtcNow.AddSeconds(30)
            while ((Get-ScheduledTask -TaskPath '\' -TaskName 'Eitmad-TlsRenewal' -ErrorAction Stop).State -eq 'Running') {
                if ([DateTime]::UtcNow -ge $deadline) { throw 'TLS maintenance has not stopped. Retry removal after it exits.' }
                Start-Sleep -Milliseconds 200
            }
            Unregister-ScheduledTask -TaskPath '\' -TaskName 'Eitmad-TlsRenewal' -Confirm:$false -ErrorAction Stop
        }
        Stop-Host $settings
        if ($clientRule) { $clientRule | Remove-NetFirewallRule -ErrorAction Stop }
        if ($discoveryRule) { $discoveryRule | Remove-NetFirewallRule -ErrorAction Stop }
        foreach ($kind in 'Server', 'Postgres') {
            $service = Get-OwnedService $settings $kind
            if ($service) {
                Close-Service $service
                Invoke-Native sc.exe @('delete', ('Eitmad' + $kind))
                $deadline = [DateTime]::UtcNow.AddSeconds(30)
                do {
                    $pending = Get-Service -Name ('Eitmad' + $kind) -ErrorAction SilentlyContinue
                    if (-not $pending) { break }
                    Close-Service $pending
                    if ([DateTime]::UtcNow -ge $deadline) { throw 'Windows service deletion is pending. Restart Windows before retrying removal.' }
                    Start-Sleep -Milliseconds 200
                } while ($true)
            }
            # SCM removes its exact service registry key. Do not delete Services
            # keys by hand or alter shared runtime/vendor registry entries.
            $key = 'HKLM:\SYSTEM\CurrentControlSet\Services\Eitmad' + $kind
            if (Test-Path -LiteralPath $key) { throw 'Windows service registry removal is pending.' }
            $source = 'Eitmad' + $kind
            if ([Diagnostics.EventLog]::SourceExists($source)) {
                if ([Diagnostics.EventLog]::LogNameFromSourceName($source, '.') -cne 'Application') { throw 'Event source belongs to another log.' }
                [Diagnostics.EventLog]::DeleteEventSource($source)
            }
        }
    } $Root
}

function Assert-ClusterStopped([string]$Cluster) {
    $pidFile = Join-Path $Cluster 'postmaster.pid'
    if (-not (Test-Path -LiteralPath $pidFile)) { return }
    $line = Get-Content -LiteralPath $pidFile -TotalCount 1
    if ($line -notmatch '^\d{1,10}$') { throw 'PostgreSQL process state is unreadable.' }
    if (Get-CimInstance Win32_Process -Filter ('ProcessId=' + $line)) {
        throw 'A PostgreSQL cluster still has a live process. No database directory was moved.'
    }
}

function Stop-PreparedDatabase([string]$Prepared, [string]$BundleDirectory) {
    $cluster = Join-Path $Prepared 'cluster'
    $pidFile = Join-Path $cluster 'postmaster.pid'
    if (-not (Test-Path -LiteralPath $pidFile)) { return }
    $lines = [IO.File]::ReadAllLines($pidFile)
    if ($lines.Length -eq 0 -or $lines[0] -notmatch '^\d+$') { throw 'Prepared PostgreSQL state is unreadable.' }
    $process = Get-CimInstance Win32_Process -Filter ('ProcessId=' + $lines[0])
    if (-not $process) { return }
    $programData = [Environment]::GetFolderPath('CommonApplicationData')
    $relative = $process.ExecutablePath.Substring([Math]::Min($process.ExecutablePath.Length, $programData.Length))
    if (-not $process.ExecutablePath.StartsWith($programData + '\', [StringComparison]::OrdinalIgnoreCase) -or
        $relative -notmatch '^\\EitmadSetupPayload-[a-f0-9]{32}\\server\\postgresql\\bin\\postgres[.]exe$' -or
        -not $process.CommandLine.Replace('/', '\').Contains(('"' + $cluster + '"'))) {
        throw 'A process uses the prepared database PID. Inspect its ownership before removal.'
    }
    $protected = $process.ExecutablePath
    for ($level = 0; $level -lt 4; $level++) { $protected = Split-Path $protected -Parent }
    [void](Assert-RemovalTree $protected $programData)
    Assert-RemovalOwner $protected
    # Authenticate the old protected release before executing its matching
    # PostgreSQL stop tool. A newly supplied bundle is not the running runtime.
    $request = @{root=$protected; anchor=(Join-Path $PSScriptRoot 'uninstall.ps1'); publisher=''} | ConvertTo-Json -Compress
    $systemPowerShell = Join-Path $env:SystemRoot 'System32/WindowsPowerShell/v1.0/powershell.exe'
    $request | & $systemPowerShell -NoProfile -NonInteractive -ExecutionPolicy AllSigned -File (Join-Path $PSScriptRoot 'verify-offline.ps1') *> $null
    if ($LASTEXITCODE -ne 0) { throw 'Prepared PostgreSQL release verification failed.' }
    & $script:HostAdapter { param($bin, $data) Invoke-Native (Join-Path $bin 'pg_ctl.exe') @('-D', $data, '-m', 'fast', '-w', '-t', '60', 'stop') } (Split-Path $process.ExecutablePath -Parent) $cluster
}

function Remove-MachineInstallation([string]$BundleDirectory, [string]$PublisherCertificateSha256,
    [string]$Parent = [Environment]::GetFolderPath('CommonApplicationData')) {
    & $script:HostAdapter { Assert-Administrator }
    $targets = @('EitmadHost', 'EitmadPreparedWorkshop', 'EitmadServerSetupProgress') | ForEach-Object { Join-Path $parent $_ }
    foreach ($target in $targets) { [void](Assert-RemovalTree $target $parent); Assert-RemovalOwner $target }
    $hostRoot = Join-Path $parent 'EitmadHost'
    Remove-HostRegistrations $hostRoot
    Stop-PreparedDatabase (Join-Path $parent 'EitmadPreparedWorkshop') $BundleDirectory
    Assert-ClusterStopped (Join-Path $hostRoot 'data')
    Assert-ClusterStopped (Join-Path $parent 'EitmadPreparedWorkshop/cluster')
    $retained = New-RetentionDirectory $parent $true
    foreach ($target in $targets) { Move-RetainedTree $target $parent $retained }
    # Retain records, private keys, settings, migration inventories, and logs.
    # Remove only the installed executable trees after the data rename succeeds.
    $oldHost = Join-Path $retained 'EitmadHost'
    $release = Join-Path $oldHost 'app/release.json'
    if (Test-Path -LiteralPath $release) { Move-Item -LiteralPath $release -Destination (Join-Path $oldHost 'release.json') -ErrorAction Stop }
    foreach ($relative in 'app', 'postgres', 'releases', 'renewal/tool', 'renewal/scripts') {
        $path = Assert-RemovalTree (Join-Path $oldHost $relative) $retained
        if (Test-Path -LiteralPath $path) { Remove-Item -LiteralPath $path -Recurse -Force -ErrorAction Stop }
    }
    # Elevated setup copies contain only executable payloads. Authenticate their
    # catalog and restrict cleanup to the exact generated name and protected root.
    foreach ($item in Get-ChildItem -LiteralPath $parent -Directory -Force | Where-Object { $_.Name -cmatch '^EitmadSetupPayload-[a-f0-9]{32}$' }) {
        $path = Assert-RemovalTree $item.FullName $parent
        Assert-RemovalOwner $path
        $catalog = Join-Path $path 'offline.cat'
        if (-not (Test-Path -LiteralPath $catalog)) { continue }
        $signature = Get-AuthenticodeSignature -LiteralPath $catalog
        if ($signature.Status -ne 'Valid') { throw 'Retained installer payload is not trusted.' }
        $hash = [Security.Cryptography.SHA256]::Create()
        try { $fingerprint = [BitConverter]::ToString($hash.ComputeHash($signature.SignerCertificate.RawData)).Replace('-', '') }
        finally { $hash.Dispose() }
        if ($fingerprint -ine $PublisherCertificateSha256) { throw 'Retained installer publisher differs.' }
        Remove-Item -LiteralPath $path -Recurse -Force -ErrorAction Stop
    }
}

function Invoke-EitmadUninstall {
    param(
        [ValidateSet('server', 'client')][string]$Role,
        [ValidateSet('CheckUser', 'User', 'Machine')][string]$Phase,
        [string]$BundleDirectory,
        [string]$PublisherCertificateSha256,
        [string]$Publisher
    )
    if ($Phase -eq 'Machine') {
        if ($Role -ne 'server') { throw 'Client removal cannot change machine installation state.' }
        Remove-MachineInstallation $BundleDirectory $PublisherCertificateSha256
    } else {
        $identity = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
        if ($identity.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) { throw 'User removal must run as the original non-elevated Windows user.' }
        Assert-DesktopStopped
        if ($Phase -eq 'User') { Remove-UserInstallation -Publisher $Publisher }
    }
}

Export-ModuleMember -Function Invoke-EitmadUninstall
