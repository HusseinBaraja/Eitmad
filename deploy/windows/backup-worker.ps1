#Requires -Version 5.1
# Runs only inside a persistent, authenticated administrative PSSession.
param([string]$Action, $Node, [string]$Target, [pscredential]$StorageCredential, [pscredential]$PostgresCredential)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
if ($Action -eq 'Capture' -and $Node.kind -eq 'server') {
    # WinRM -FilePath does not transfer sibling modules. Use the participant's installed tools.
    Import-Module (Join-Path (Split-Path $Node.hostModule -Parent) 'backup-tools.psm1') -Force -DisableNameChecking
}

function Assert-LocalTree([string]$Path) {
    if ($Path -notmatch '^[A-Za-z]:\\' -or $Path -match '[,\r\n"]') { throw 'Invalid local backup input.' }
    for ($part = $Path; $part; $part = Split-Path $part -Parent) {
        if ((Get-Item -LiteralPath $part -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) {
            throw 'Backup inputs cannot use reparse points.'
        }
    }
    if (@(Get-ChildItem -LiteralPath $Path -Force -Recurse | Where-Object {
        $_.Attributes -band [IO.FileAttributes]::ReparsePoint
    }).Count) { throw 'Backup inputs cannot contain external links or tablespaces.' }
}

function Invoke-BackupTool([string]$File, [string[]]$Arguments) {
    # Never forward native diagnostics: PostgreSQL and Windows can include private paths.
    & $File @Arguments *> $null
    if ($LASTEXITCODE -ne 0) { throw 'Backup tool failed. Inspect the protected installation on this computer.' }
}

function Get-Boot { (Get-CimInstance Win32_OperatingSystem).LastBootUpTime.ToUniversalTime().ToString('o') }

if ($Action -eq 'Prepare') {
    if (Get-Variable EitmadBackup -Scope Global -ErrorAction SilentlyContinue) { throw 'Backup already prepared.' }
    $principal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
    if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) { throw 'Backup requires an administrator.' }
    $global:EitmadBackup = @{ Locks = New-Object 'Collections.Generic.List[IDisposable]'; Boot = Get-Boot
        Server = $null; WasRunning = $false; Mapping = $null }
    foreach ($volume in $Node.volumes) {
        if ($volume -notmatch '^[A-Za-z]:$' -or (Get-Volume -DriveLetter $volume[0]).FileSystem -ne 'NTFS') {
            throw 'Select complete local NTFS volumes.'
        }
    }
    # BMR must include every profile, not just the SQLite runtime. Credential Manager uses DPAPI.
    $profiles = [Environment]::ExpandEnvironmentVariables((Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\ProfileList').ProfilesDirectory)
    foreach ($path in @($Node.requiredPaths) + @($Node.runtimes) + @($profiles)) {
        if ($path.Substring(0, 2) -notin $Node.volumes) { throw 'A required path or Windows profile volume is excluded.' }
        if ($path -ne $profiles) { Assert-LocalTree $path }
    }
    foreach ($runtime in $Node.runtimes) {
        # FileShare.None conflicts with Rust's open and fs2 lock. Never kill an active client.
        $lockPath = Join-Path $runtime 'engine.authority.lock'
        $global:EitmadBackup.Locks.Add([IO.File]::Open($lockPath, 'Open', 'ReadWrite', 'None'))
    }
    if ($Node.kind -eq 'server') {
        $module = Import-Module $Node.hostModule -Force -PassThru
        $settings = & $module { param($Root, $Name)
            Assert-InstalledProtection $Root $Name
            Read-HostManifest $Root $Name
        } $Node.hostRoot $Node.serviceName
        if ($Node.hostRoot.Substring(0, 2) -notin $Node.volumes -or $Node.hostRoot -notin $Node.requiredPaths) {
            throw 'The complete host directory must be a required input.'
        }
        Assert-LocalTree $Node.hostRoot
        $global:EitmadBackup.Locks.Add([IO.File]::Open((Join-Path $Node.hostRoot 'host.lock'), 'OpenOrCreate', 'ReadWrite', 'None'))
        $global:EitmadBackup.Settings = $settings
        $global:EitmadBackup.HostModule = $module
        $global:EitmadBackup.Server = & $module { param($s) Get-OwnedService $s 'Server' } $settings
        if ($global:EitmadBackup.Server.Status -ne 'Running') { throw 'The server must be healthy before backup.' }
    }
    # Create a separate authenticated SMB session; no double-hop delegation or password arguments.
    if (-not @(Get-SmbMapping | Where-Object RemotePath -eq $Node.share).Count) {
        [void](New-SmbMapping -RemotePath $Node.share -UserName $StorageCredential.UserName `
            -Password $StorageCredential.GetNetworkCredential().Password -Persistent $false -RequirePrivacy $true)
        $global:EitmadBackup.Mapping = $true
    }
    $shareParts = $Node.share.TrimStart('\').Split('\')
    if (-not @(Get-SmbConnection | Where-Object {
        $_.ServerName -ieq $shareParts[0] -and $_.ShareName -ieq $shareParts[1] -and $_.Encrypted
    }).Count) { throw 'An encrypted SMB connection is required.' }
    [pscustomobject]@{ computer = $env:COMPUTERNAME; machineId = (Get-CimInstance Win32_ComputerSystemProduct).UUID; boot = $global:EitmadBackup.Boot }
} elseif ($Action -eq 'Freeze') {
    if ($Node.kind -eq 'server') {
        $global:EitmadBackup.WasRunning = $true
        $global:EitmadBackup.Server.Stop()
        $global:EitmadBackup.Server.WaitForStatus([ServiceProcess.ServiceControllerStatus]::Stopped, [TimeSpan]::FromSeconds(60))
    }
} elseif ($Action -eq 'Capture') {
    if ((Get-Boot) -cne $global:EitmadBackup.Boot) { throw 'Computer rebooted during the recovery point.' }
    if ($Node.kind -eq 'server') {
        $global:EitmadBackup.Server.Refresh()
        if ($global:EitmadBackup.Server.Status -ne 'Stopped') { throw 'Server write barrier was lost.' }
        $bin = Join-Path $Node.hostRoot 'postgres/bin'
        Backup-Postgres $bin $global:EitmadBackup.Settings.databasePort $PostgresCredential (Join-Path $Target 'cluster')
    }
    # A unique target prevents wbadmin from overwriting an earlier network generation.
    $image = Join-Path $Target 'image'
    [void][IO.Directory]::CreateDirectory($image)
    Invoke-BackupTool "$env:SystemRoot\System32\wbadmin.exe" @('start', 'backup', ('-backupTarget:' + $image),
        ('-include:' + ($Node.volumes -join ',')), '-allCritical', '-vssCopy', '-quiet')
    Invoke-BackupTool "$env:SystemRoot\System32\wbadmin.exe" @('get', 'versions', ('-backupTarget:' + $image),
        ('-machine:' + $env:COMPUTERNAME))
    [pscustomobject]@{ imageCatalogVerified = $true }
} elseif ($Action -eq 'Check') {
    if ((Get-Boot) -cne $global:EitmadBackup.Boot) { throw 'Computer rebooted during backup.' }
    if ($Node.kind -eq 'server') {
        $global:EitmadBackup.Server.Refresh()
        if ($global:EitmadBackup.Server.Status -ne 'Stopped') { throw 'Server resumed before the cohort completed.' }
    }
    if ($global:EitmadBackup.Locks.Count -ne (@($Node.runtimes).Count + [int]($Node.kind -eq 'server'))) {
        throw 'A write barrier was lost.'
    }
} elseif ($Action -eq 'Release') {
    if (Get-Variable EitmadBackup -Scope Global -ErrorAction SilentlyContinue) {
        try {
            if ($global:EitmadBackup.WasRunning) {
                $global:EitmadBackup.Server.Start()
                $global:EitmadBackup.Server.WaitForStatus([ServiceProcess.ServiceControllerStatus]::Running, [TimeSpan]::FromSeconds(60))
                & $global:EitmadBackup.HostModule { param($s) Wait-Ready $s } $global:EitmadBackup.Settings
            }
        } finally {
            foreach ($lock in $global:EitmadBackup.Locks) { $lock.Dispose() }
            if ($global:EitmadBackup.Server) { $global:EitmadBackup.Server.Dispose() }
            if ($global:EitmadBackup.Mapping) { Remove-SmbMapping -RemotePath $Node.share -Force -UpdateProfile:$false }
            Remove-Variable EitmadBackup -Scope Global
        }
    }
} else { throw 'Unknown backup worker action.' }
