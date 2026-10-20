Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
Import-Module (Join-Path $PSScriptRoot 'host.psm1') -Force
$script:HostAdapter = Get-Module host

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

function Get-InstallPackage($Package, [string]$Target) {
    Import-Module (Join-Path $Package.directory 'package.psm1') -Force
    $release = Read-EitmadPackage -Directory $Package.directory -Target $Target `
        -ExpectedReleaseSha256 $Package.releaseSha256 -Publisher $Package.publisher
    Assert-WindowsPrerequisites
    return $release
}

function Assert-FreshDesktop($Request) {
    Assert-DesktopUser $Request
    if (@(Get-AppxPackage -Name Eitmad.Desktop).Count) {
        throw 'Desktop already exists. Preserve data and use the separate repair or upgrade procedure.'
    }
    # Verify checks runtime and inventory. The registration path also checks MSIX signature.
    & (Join-Path $Request.desktop.directory 'install-client.ps1') -Action Verify `
        -ExpectedReleaseSha256 $Request.desktop.releaseSha256 -Publisher $Request.desktop.publisher *> $null
    if (-not $?) { throw 'Desktop verification failed.' }
}

function Invoke-EitmadInstallationAction {
    param([string]$Action, [string]$Backend, $Request)
    if ($Action -in 'ClientPreflight', 'Desktop') {
        Assert-FreshDesktop $Request
        if ($Action -eq 'Desktop') {
            Assert-DesktopUser $Request
            & (Join-Path $Request.desktop.directory 'install-client.ps1') -Action Install `
                -ExpectedReleaseSha256 $Request.desktop.releaseSha256 -Publisher $Request.desktop.publisher *> $null
            if (-not $?) { throw 'Desktop registration failed.' }
        }
        return
    }
    & $script:HostAdapter { Assert-Administrator }
    if ($Action -eq 'ServerPreflight') {
        foreach ($path in $Request.serviceStateDirectory, $Request.serverProgressDirectory, $Request.preparation.stateDirectory) {
            & $script:HostAdapter { param($value) [void](Get-SafePath $value); Assert-ProtectedParents $value } $path
            if (Test-Path -LiteralPath $path) { throw 'Existing installation state is retained.' }
        }
        foreach ($kind in 'Server', 'Postgres') {
            if (Get-Service -Name ($Request.serviceName + $kind) -ErrorAction SilentlyContinue) {
                throw 'Service identity already exists.'
            }
        }
        [void](Get-InstallPackage $Request.server 'windows-server')
        $packagedBackend = Join-Path $Request.server.directory 'eitmad-server.exe'
        if ((Get-FileHash -LiteralPath $Backend).Hash -cne (Get-FileHash -LiteralPath $packagedBackend).Hash) {
            throw 'The running installer must match the verified server package.'
        }
        $expectedScript = [IO.Path]::GetFullPath((Join-Path $Request.server.directory 'deploy/windows/prepare-workshop.ps1'))
        $expectedCertificate = [IO.Path]::GetFullPath((Join-Path $Request.server.directory 'tools/lan-certificate/bin/Release/net8.0-windows/Eitmad.LanCertificate.exe'))
        if ($Request.preparation.platformScript -ine $expectedScript -or $Request.preparation.certificateExecutable -ine $expectedCertificate) {
            throw 'Preparation tools must come from the verified server package.'
        }
        if ((Get-FileHash -LiteralPath $Request.winswExecutable -Algorithm SHA256).Hash -ine $Request.winswSha256) {
            throw 'WinSW digest differs.'
        }
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

function Invoke-EitmadInstallationWorker {
    param([string]$Backend, [string]$PipeName, [int]$BrokerId)
    & $script:HostAdapter { Assert-Administrator }
    Initialize-InstallationChannel
    $pipe = [EitmadInstallationChannel]::Connect($PipeName, $BrokerId)
    try { return [EitmadInstallationChannel]::RunWorker($pipe, $Backend) }
    finally { $pipe.Dispose() }
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
