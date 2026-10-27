Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$script:HostAdapter = Import-Module (Join-Path $PSScriptRoot 'host.psm1') -Force -PassThru

function Invoke-InstallationCheck([int]$Code, [scriptblock]$Check) {
    try { & $Check }
    catch {
        if (-not $_.Exception.Data.Contains('EitmadSetupFailure')) { $_.Exception.Data['EitmadSetupFailure'] = $Code }
        throw
    }
}

function Get-InstallationIdentity {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($identity)
    return [pscustomobject]@{Sid = $identity.User.Value;
        Elevated = $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)}
}

function Assert-DesktopUser($Request) {
    $identity = Get-InstallationIdentity
    if ($identity.Sid -cne $Request.intendedUserSid -or $identity.Elevated) {
        throw 'Desktop registration requires the intended non-elevated Windows user.'
    }
}

function Get-InstallPackage($Package, [string]$Target, [switch]$PlatformOnly) {
    Import-Module (Join-Path $Package.directory 'package.psm1') -Force
    $release = Read-EitmadPackage -Directory $Package.directory -Target $Target `
        -ExpectedReleaseSha256 $Package.releaseSha256 -Publisher $Package.publisher
    Assert-WindowsPrerequisites -PlatformOnly:$PlatformOnly
    return $release
}

function Assert-FreshDesktop($Request) {
    Invoke-InstallationCheck 10 { Assert-DesktopUser $Request }
    $installed = @(Get-AppxPackage -Name Eitmad.Desktop)
    if ($installed.Count) { Invoke-InstallationCheck 15 { Assert-RetainedDesktop $Request $installed } }
    # Verify checks runtime and inventory. The registration path also checks MSIX signature.
    & (Join-Path $Request.desktop.directory 'install-client.ps1') -Action Verify `
        -ExpectedReleaseSha256 $Request.desktop.releaseSha256 -Publisher $Request.desktop.publisher *> $null
    if (-not $?) { throw 'Desktop verification failed.' }
}

function Assert-RetainedDesktop($Request, $Installed) {
    $release = Get-InstallPackage $Request.desktop 'windows-desktop'
    if ($Installed.Count -ne 1 -or $Installed[0].Publisher -cne $release.publisher -or
        $Installed[0].Version -ne [version]$release.windowsVersion -or $Installed[0].Status -ne 'Ok') {
        throw 'The registered desktop differs. Preserve data and use the exact release repair procedure.'
    }
    $inventory = Join-Path $Installed[0].InstallLocation 'release.json'
    if ((Get-FileHash -LiteralPath $inventory -Algorithm SHA256).Hash -ine $release.payloadInventorySha256) {
        throw 'The registered desktop inventory differs.'
    }
}

function Assert-RetainedHost($Request, $Release) {
    & $script:HostAdapter {
        param($r, $release)
        $root = Get-SafePath $r.serviceStateDirectory
        Assert-InstalledProtection $root $r.serviceName
        Assert-PlainTree $root
        $settings = Read-HostManifest $root $r.serviceName
        if ($settings.endpoint -cne ('https://' + $r.preparation.serverName + ':' + $r.preparation.serverPort + '/') -or
            $settings.databasePort -ne $r.preparation.databasePort -or $settings.serverPort -ne $r.preparation.serverPort -or
            $settings.address -cne $r.preparation.listenAddress -or (Test-Path -LiteralPath (Join-Path $root 'release-pending.json'))) {
            throw 'Retained host settings differ or a release repair is pending.'
        }
        $lan = $r.preparation.stateDirectory
        Assert-PlainTree $lan
        Assert-ProtectedParents $lan
        foreach ($pair in @(@('private/trusted-ca.pem','trusted-ca.pem'), @('private/token-key','token-key'))) {
            if ((Get-FileHash -LiteralPath (Join-Path $root $pair[0])).Hash -cne
                (Get-FileHash -LiteralPath (Join-Path $lan $pair[1])).Hash) { throw 'Retained installation identity differs.' }
        }
        if ((Get-FileHash -LiteralPath (Join-Path $root 'app/eitmad-server.exe')).Hash -ine $release.files.'eitmad-server.exe' -or
            (Get-FileHash -LiteralPath (Join-Path $root 'data/PG_VERSION')).Hash -cne
            (Get-FileHash -LiteralPath (Join-Path $lan 'cluster/PG_VERSION')).Hash) { throw 'Retained host release or cluster differs.' }
        $source = Get-Content -LiteralPath (Join-Path $lan 'server.json') -Raw | ConvertFrom-Json
        if ([IO.File]::ReadAllText((Join-Path $root 'private/database-url')) -cne $source.environment.EITMAD_SERVER_DATABASE_URL) {
            throw 'Retained database route differs.'
        }
        foreach ($relative in 'app/host-service.exe','postgres/host-service.exe') {
            if ((Get-FileHash -LiteralPath (Join-Path $root $relative)).Hash -ine $r.winswSha256) { throw 'Retained service wrapper differs.' }
        }
        $xml = [xml](Get-Content -LiteralPath (Join-Path $root 'app/host-service.xml') -Raw)
        if ($xml.service.executable -ine (Join-Path $root 'app/eitmad-server.exe') -or $xml.service.arguments -cne 'serve') {
            throw 'Retained server service points to another executable.'
        }
        foreach ($kind in 'Postgres','Server') { $service = Get-OwnedService $settings $kind; Close-Service $service }
    } $Request $Release
}

function Install-OfflineServerPrerequisites([string]$Backend) {
    $root = Split-Path (Split-Path (Split-Path $Backend -Parent) -Parent) -Parent
    if (-not (Test-Path -LiteralPath (Join-Path $root 'offline.cat') -PathType Leaf)) {
        throw 'The complete authenticated offline bundle is required.'
    }
    $vc = Get-ItemProperty -LiteralPath 'HKLM:\SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\x64' -ErrorAction SilentlyContinue
    $framework = Get-ItemProperty -LiteralPath 'HKLM:\SOFTWARE\Microsoft\NET Framework Setup\NDP\v4\Full' -ErrorAction SilentlyContinue
    $installers = @()
    if (-not $vc -or $vc.Installed -ne 1 -or [version]$vc.Version.TrimStart('v') -lt [version]'14.40.0.0') {
        $installers += @{path='server/vc-redist/vc_redist.x64.exe'; arguments='/install /quiet /norestart'}
    }
    if (-not $framework -or $framework.Release -lt 528040) {
        $installers += @{path='server/netfx/ndp48-x86-x64-allos-enu.exe'; arguments='/q /norestart'}
    }
    foreach ($installer in $installers) {
        $child = Start-Process -FilePath (Join-Path $root $installer.path) -ArgumentList $installer.arguments -PassThru -WindowStyle Hidden
        try {
            if (-not $child.WaitForExit(1200000)) { throw 'Prerequisite installer deadline elapsed. Inspect its state before retrying.' }
            if ($child.ExitCode -eq 3010) { throw 'A prerequisite requires a Windows restart. Preserve setup state and restart before recovery.' }
            if ($child.ExitCode -ne 0) { throw 'Offline prerequisite installation failed.' }
        } finally { $child.Dispose() }
    }
}

function Assert-InstallationPreparationTools($Request) {
    $expectedScript = [IO.Path]::GetFullPath((Join-Path $Request.server.directory 'deploy/windows/prepare-workshop.ps1'))
    $expectedCertificate = [IO.Path]::GetFullPath((Join-Path $Request.server.directory 'tools/lan-certificate/bin/Release/net8.0-windows/Eitmad.LanCertificate.exe'))
    if ([IO.Path]::GetFullPath($Request.preparation.platformScript) -ine $expectedScript -or
        [IO.Path]::GetFullPath($Request.preparation.certificateExecutable) -ine $expectedCertificate) {
        throw 'Preparation tools must come from the verified server package.'
    }
}

function Invoke-EitmadInstallationAction {
    param([string]$Action, [string]$Backend, $Request)
    if ($Action -in 'ClientPreflight', 'Desktop') {
        Invoke-InstallationCheck 11 { Assert-FreshDesktop $Request }
        if ($Action -eq 'Desktop') {
            Invoke-InstallationCheck 10 { Assert-DesktopUser $Request }
            $installed = @(Get-AppxPackage -Name Eitmad.Desktop)
            if ($installed.Count) { Assert-RetainedDesktop $Request $installed; return }
            & (Join-Path $Request.desktop.directory 'install-client.ps1') -Action Install `
                -ExpectedReleaseSha256 $Request.desktop.releaseSha256 -Publisher $Request.desktop.publisher *> $null
            if (-not $?) { throw 'Desktop registration failed.' }
        }
        return
    }
    & $script:HostAdapter { Assert-Administrator }
    if ($Action -eq 'ServerPreflight') {
        foreach ($path in $Request.serviceStateDirectory, $Request.serverProgressDirectory, $Request.preparation.stateDirectory) {
            Invoke-InstallationCheck 13 { & $script:HostAdapter { param($value) [void](Get-SafePath $value); Assert-ProtectedParents $value } $path }
            if (Test-Path -LiteralPath $path) { & $script:HostAdapter { param($value) Assert-PlainTree $value } $path }
        }
        foreach ($kind in 'Server', 'Postgres') {
            if (-not (Test-Path -LiteralPath $Request.serviceStateDirectory) -and
                (Get-Service -Name ($Request.serviceName + $kind) -ErrorAction SilentlyContinue)) {
                throw 'Service identity already exists.'
            }
        }
        # The protected copy was authenticated before this backend started. Standalone
        # operator installs keep their existing externally prepared prerequisite path.
        $release = Invoke-InstallationCheck 12 { Get-InstallPackage $Request.server 'windows-server' -PlatformOnly }
        if (Test-Path -LiteralPath $Request.serviceStateDirectory) { Invoke-InstallationCheck 15 { Assert-RetainedHost $Request $release } }
        $packagedBackend = Join-Path $Request.server.directory 'eitmad-server.exe'
        if ((Get-FileHash -LiteralPath $Backend).Hash -cne (Get-FileHash -LiteralPath $packagedBackend).Hash) {
            throw 'The running installer must match the verified server package.'
        }
        Assert-InstallationPreparationTools $Request
        if ((Get-FileHash -LiteralPath $Request.winswExecutable -Algorithm SHA256).Hash -ine $Request.winswSha256) {
            throw 'WinSW digest differs.'
        }
        if ((Split-Path $Backend -Parent | Split-Path -Leaf) -eq 'application') {
            Invoke-InstallationCheck 14 { Install-OfflineServerPrerequisites $Backend }
        }
        Invoke-InstallationCheck 14 { Assert-WindowsPrerequisites }
        return
    }
    if ($Action -eq 'ResumeInstall') {
        $release = Get-InstallPackage $Request.server 'windows-server'
        Assert-RetainedHost $Request $release
        & $script:HostAdapter {
            param($r, $release)
            $root = Get-SafePath $r.serviceStateDirectory
            $lock = [IO.File]::Open((Join-Path $root 'host.lock'), 'OpenOrCreate', 'ReadWrite', 'None')
            try {
                $settings = Read-HostManifest $root $r.serviceName
                Install-Host $settings -ResumeExisting
                $release | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $root 'app/release.json') -Encoding UTF8
            } finally { $lock.Dispose() }
        } $Request $release
        return
    }
    $arguments = @{Action = $Action; StateDirectory = $Request.serviceStateDirectory; Name = $Request.serviceName}
    if ($Action -eq 'Install') {
        $arguments.ExpectedReleaseSha256 = $Request.server.releaseSha256
        $arguments.Publisher = $Request.server.publisher
        $arguments.LanDirectory = $Request.preparation.stateDirectory
        $arguments.ClusterDirectory = Join-Path $Request.preparation.stateDirectory 'cluster'
        $arguments.PostgresBin = $Request.preparation.postgresBin
        $arguments.WinSwExecutable = $Request.winswExecutable
        $arguments.WinSwSha256 = $Request.winswSha256
    } elseif ($Action -eq 'Firewall') { $arguments.PrivateLan = $true }
    & (Join-Path $Request.server.directory 'install-server.ps1') @arguments *> $null
    if (-not $?) { throw 'Host action failed.' }
}

function Initialize-InstallationChannel {
    if (-not ('EitmadInstallationChannel' -as [type])) {
        Add-Type -Path (Join-Path $PSScriptRoot 'installation-channel.cs')
    }
}

function Grant-PostgresToolReadAccess([string]$Bundle) {
    # initdb drops the Administrators group in its child token. Keep its tool DLLs
    # readable through the launching user's SID, without exposing server controls.
    $postgres = Join-Path $Bundle 'server/postgresql'
    if (-not (Test-Path -LiteralPath $postgres -PathType Container)) { return }
    $acl = Get-Acl -LiteralPath $postgres
    $acl.AddAccessRule((New-Object Security.AccessControl.FileSystemAccessRule(
        [Security.Principal.WindowsIdentity]::GetCurrent().User, 'ReadAndExecute',
        'ContainerInherit,ObjectInherit', 'None', 'Allow')))
    Set-Acl -LiteralPath $postgres -AclObject $acl
}

function Test-InstallationProofRequest([string]$Line) {
    $event = $Line | ConvertFrom-Json
    # Route private input only; Rust validates the ordered progress and proof.
    return ($event.version -eq 1 -and $event.PSObject.Properties['stage'] -and
        $event.stage -ceq 'enrollDesktop' -and $event.status -ceq 'running')
}

function New-ProtectedInstallationPayload([string]$Source, [string]$Stage) {
    if (Test-Path -LiteralPath $Stage) { throw 'Protected payload requires a new directory.' }
    & $script:HostAdapter { param($p) [void](Get-SafePath $p); Assert-ProtectedParents $p } $Stage
    $acl = New-Object Security.AccessControl.DirectorySecurity
    $acl.SetAccessRuleProtection($true, $false)
    $acl.SetOwner((New-Object Security.Principal.SecurityIdentifier('S-1-5-32-544')))
    foreach ($sid in 'S-1-5-18', 'S-1-5-32-544') {
        $acl.AddAccessRule((New-Object Security.AccessControl.FileSystemAccessRule(
            (New-Object Security.Principal.SecurityIdentifier($sid)), 'FullControl', 'ContainerInherit,ObjectInherit', 'None', 'Allow')))
    }
    # .NET Framework creates the directory with the protected DACL atomically.
    [void][IO.Directory]::CreateDirectory($Stage, $acl)
    Get-ChildItem -LiteralPath $Source -Force | Copy-Item -Destination $Stage -Recurse -Force
    # Match the service adapter's copy policy: discard any source ACLs and
    # inherit only the protected staging root, including on copied directories.
    foreach ($item in Get-ChildItem -LiteralPath $Stage -Force -Recurse) {
        $inherited = if ($item.PSIsContainer) { New-Object Security.AccessControl.DirectorySecurity }
            else { New-Object Security.AccessControl.FileSecurity }
        $inherited.SetAccessRuleProtection($false, $false)
        $inherited.SetOwner((New-Object Security.Principal.SecurityIdentifier('S-1-5-32-544')))
        Set-Acl -LiteralPath $item.FullName -AclObject $inherited
    }
    Grant-PostgresToolReadAccess $Stage
}

function Invoke-EitmadInstallationWorker {
    param([string]$Backend, [string]$PipeName, [int]$BrokerId)
    & $script:HostAdapter { Assert-Administrator }
    Initialize-InstallationChannel
    $pipe = [EitmadInstallationChannel]::Connect($PipeName, $BrokerId)
    $stage = $null
    try {
        # The initiating process retains the source. Never execute elevated payloads
        # from a user-writable extraction directory, including DLL search paths.
        $source = Split-Path (Split-Path (Split-Path $Backend -Parent) -Parent) -Parent
        $verifier = Join-Path $PSScriptRoot 'verify-offline.ps1'
        $systemPowerShell = "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe"
        @{root=$source; anchor=$Backend; publisher=''} | ConvertTo-Json -Compress |
            & $systemPowerShell -NoProfile -NonInteractive -File $verifier *> $null
        if ($LASTEXITCODE -ne 0) { throw 'Offline source authentication failed after elevation.' }
        $stage = Join-Path $env:ProgramData ('EitmadSetupPayload-' + [Guid]::NewGuid().ToString('N'))
        New-ProtectedInstallationPayload $source $stage
        $protectedBackend = Join-Path $stage 'server/application/eitmad-server.exe'
        @{root=$stage; anchor=$Backend; publisher=''} | ConvertTo-Json -Compress |
            & $systemPowerShell -NoProfile -NonInteractive -File $verifier *> $null
        if ($LASTEXITCODE -ne 0) { throw 'Protected offline copy authentication failed.' }
        # Retain the authenticated backend used by TLS renewal.
        $code = [EitmadInstallationChannel]::RunWorker($pipe, $protectedBackend, 'install-windows-server',
            [Func[string,bool]]{ param($line) return (Test-InstallationProofRequest $line) })
        return $code
    }
    finally {
        $pipe.Dispose()

    }
}

function Invoke-EitmadInstallationElevation {
    param([string]$Backend, [string]$PrivateInput)
    Initialize-InstallationChannel
    $name = 'eitmad-install-' + [Guid]::NewGuid().ToString('N')
    $pipe = [EitmadInstallationChannel]::Create($name)
    $child = $null
    try {
        # All argv values are public. Reuse the adapter's Windows quoting policy.
        $script = Join-Path $PSScriptRoot 'installation.ps1'
        foreach ($path in $script, $Backend) { & $script:HostAdapter { param($value) [void](Get-SafePath $value) } $path }
        $arguments = @('-NoProfile', '-NonInteractive', '-File', $script, '-Action', 'Worker', '-Backend', $Backend,
            '-PipeName', $name, '-BrokerId', [string]$PID) | ForEach-Object {
                '"' + [regex]::Replace([regex]::Replace($_, '(\\*)"', '$1$1\"'), '(\\+)$', '$1$1') + '"'
            }
        $child = Start-Process -FilePath "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe" `
            -ArgumentList ($arguments -join ' ') -Verb RunAs -PassThru -WindowStyle Hidden
        [EitmadInstallationChannel]::Accept($pipe, $child.Id)
        [EitmadInstallationChannel]::Send($pipe, $PrivateInput)
        $PrivateInput = $null
        while ($true) {
            $event = [EitmadInstallationChannel]::Receive($pipe, 32768)
            if (-not $event) { break }
            [Console]::Out.WriteLine($event)
            [Console]::Out.Flush()
            if (Test-InstallationProofRequest $event) {
                $proof = [EitmadInstallationChannel]::ReadPrivateInput([Console]::In)
                try { [EitmadInstallationChannel]::Send($pipe, $proof) }
                finally { $proof = $null }
            }
        }
        $result = [EitmadInstallationChannel]::Receive($pipe, 16)
        if (-not $child.WaitForExit(10000) -or $result -cne 'success' -or $child.ExitCode -ne 0) { return 1 }
        return 0
    } finally {
        $PrivateInput = $null
        $pipe.Dispose()
        if ($child) { $child.Dispose() }
    }
}

Export-ModuleMember -Function Invoke-EitmadInstallationAction, Invoke-EitmadInstallationElevation, Invoke-EitmadInstallationWorker
