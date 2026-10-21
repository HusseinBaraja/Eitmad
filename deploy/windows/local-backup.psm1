Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
Import-Module (Join-Path $PSScriptRoot 'host.psm1') -Force
Import-Module (Join-Path $PSScriptRoot 'backup.psm1') -Force
Import-Module (Join-Path $PSScriptRoot 'backup-tools.psm1') -Force -DisableNameChecking
$script:HostTools = Get-Module host
$script:BackupTools = Get-Module backup

function Assert-LocalBackupControl([string]$Path) {
    & $script:BackupTools { param($p) Assert-BackupControl $p } $Path
}

function Get-LocalBackupPath([string]$Path) {
    if ($Path -notmatch '^[A-Za-z]:\\' -or $Path -match '[<>|~\x00-\x1f]' -or
        @($Path.Substring(3).Split('\') | Where-Object { $_ -eq '' -or $_ -match '[. ]$' -or
            $_ -match '^(?i:CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])(?:\.|$)' }).Count) { throw 'DestinationUnavailable' }
    & $script:HostTools { param($p) Get-SafePath $p } $Path
}

function Assert-BackupPathSeparation($Configuration, [string]$StateDirectory) {
    $paths = @($StateDirectory,$Configuration.hostRoot,$Configuration.recoveryDirectory,
        $Configuration.localDestination,$Configuration.externalDestination) | Where-Object { $_ }
    $paths = @($paths | ForEach-Object { Get-LocalBackupPath $_ })
    for ($i=0; $i -lt $paths.Count; $i++) {
        for ($j=$i+1; $j -lt $paths.Count; $j++) {
            # Use Windows ordinal comparison for non-ASCII case aliases as well.
            if ($paths[$i].Equals($paths[$j],[StringComparison]::OrdinalIgnoreCase) -or
                $paths[$i].StartsWith($paths[$j]+'\',[StringComparison]::OrdinalIgnoreCase) -or
                $paths[$j].StartsWith($paths[$i]+'\',[StringComparison]::OrdinalIgnoreCase)) { throw 'DestinationUnavailable' }
        }
    }
}

function Assert-LocalBackupTree([string]$Path) {
    & $script:HostTools { param($p) Assert-PlainTree $p } $Path
}

function New-LocalBackupDirectory([string]$Path) {
    [void](Get-LocalBackupPath $Path)
    if (Test-Path -LiteralPath $Path) { Assert-LocalBackupControl $Path }
    else {
        Assert-LocalBackupControl (Split-Path $Path -Parent)
        & $script:HostTools { param($p) Protect-Directory $p } $Path
    }
}

function Get-BackupDisk([string]$Path) {
    $volume = Get-Volume -FilePath $Path
    if ($volume.FileSystem -ne 'NTFS' -or -not $volume.DriveLetter) { throw 'DestinationUnavailable' }
    $bitlocker = Get-BitLockerVolume -MountPoint ($volume.DriveLetter + ':')
    if ($bitlocker.ProtectionStatus -ne 'On' -or $bitlocker.VolumeStatus -ne 'FullyEncrypted' -or
        $bitlocker.LockStatus -ne 'Unlocked') { throw 'DestinationUnavailable' }
    $partition = Get-Partition -DriveLetter $volume.DriveLetter
    [pscustomobject]@{ disk = $partition.DiskNumber; free = [uint64]$volume.SizeRemaining }
}

function Get-LocalBackupRoot($Configuration, [bool]$External = $false, [bool]$Create = $false) {
    $destination = if ($External) { $Configuration.externalDestination } else { $Configuration.localDestination }
    if (-not $destination) { throw 'DestinationUnavailable' }
    $path = Get-LocalBackupPath $destination
    if (-not (Test-Path -LiteralPath $path -PathType Container)) { throw 'DestinationUnavailable' }
    Assert-LocalBackupControl $path
    $disk = Get-BackupDisk $path
    if ($External) {
        # Drive letters alone do not prove independence (partitions can share a physical disk).
        $sourceDisk = (Get-Partition -DriveLetter $Configuration.hostRoot[0]).DiskNumber
        $localDisk = (Get-Partition -DriveLetter $Configuration.localDestination[0]).DiskNumber
        $recoveryDisk = (Get-Partition -DriveLetter $Configuration.recoveryDirectory[0]).DiskNumber
        if ($disk.disk -in @($sourceDisk, $localDisk, $recoveryDisk)) { throw 'DestinationUnavailable' }
        foreach ($number in $disk.disk, $sourceDisk, $localDisk, $recoveryDisk) {
            # Different VHD/Storage Spaces numbers can still have the same physical backing.
            # Accept only directly attached drive types whose boundary Windows can identify.
            if ((Get-Disk -Number $number).BusType -notin 'ATA','ATAPI','SATA','SAS','NVMe','USB','SD','MMC','1394') {
                throw 'DestinationUnavailable'
            }
        }
    }
    $root = Join-Path $path ('server-v1-' + $Configuration.deploymentId)
    # PostgreSQL's Windows file tools are not consistently long-path aware.
    # Reserve room for staging, the generation ID, and database relation/WAL filenames.
    if ($root.Length -gt 120) { throw 'DestinationUnavailable' }
    if ($Create) { New-LocalBackupDirectory $root }
    elseif (-not (Test-Path -LiteralPath $root -PathType Container)) { throw 'DestinationUnavailable' }
    else { Assert-LocalBackupControl $root }
    return $root
}

function Assert-BackupCapacity([string]$Root, [uint64]$Required, [uint64]$Reserve) {
    $disk = Get-BackupDisk $Root
    if ($disk.free -lt $Reserve -or $Required -gt ($disk.free - $Reserve)) { throw 'InsufficientSpace' }
}

function Get-TreeBytes([string]$Path) {
    [uint64]$total = 0
    foreach ($file in Get-ChildItem -LiteralPath $Path -File -Recurse -Force) { $total += [uint64]$file.Length }
    return $total
}

function Get-LocalGeneration([string]$Root, [string]$Id) {
    if ($Id -notmatch '^\d{20}-[a-f0-9]{32}$') { throw 'CaptureFailed' }
    $path = Get-LocalBackupPath (Join-Path $Root $Id)
    if ((Split-Path $path -Parent) -ine $Root) { throw 'CaptureFailed' }
    return $path
}

function Write-LocalBackupJson([string]$Path, $Value) {
    & $script:BackupTools { param($p, $v) Write-BackupJson $p $v } $Path $Value
}

function Get-LocalBackupManifest([string]$Generation, [string]$Deployment) {
    Assert-LocalBackupTree $Generation
    $marker = Get-Content -LiteralPath (Join-Path $Generation 'complete.json') -Raw | ConvertFrom-Json
    if ($marker.version -ne 1 -or $marker.kind -cne 'serverLocal' -or $marker.deploymentId -cne $Deployment -or
        $marker.id -cne (Split-Path $Generation -Leaf) -or @($marker.files).Count -eq 0 -or
        'cluster\backup_manifest' -notin $marker.files.path) { throw 'CaptureFailed' }
    return $marker
}

function Write-LocalBackupManifest([string]$Generation, $Configuration, [string]$Id) {
    $files = @(Get-ChildItem -LiteralPath $Generation -File -Force -Recurse | Where-Object Name -ne 'incomplete.json' | ForEach-Object {
        [pscustomobject]@{ path = $_.FullName.Substring($Generation.Length + 1); bytes = $_.Length;
            sha256 = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash }
    })
    Write-LocalBackupJson (Join-Path $Generation 'complete.json') @{
        version = 1; kind = 'serverLocal'; deploymentId = $Configuration.deploymentId; id = $Id; files = $files
    }
}

function Test-LocalBackupGeneration([string]$Generation, [string]$Deployment, [string]$PostgresBin) {
    $marker = Get-LocalBackupManifest $Generation $Deployment
    $seen = @{}
    foreach ($entry in $marker.files) {
        if ($entry.path -match '(^[\\/]|:|(^|[\\/])\.\.?([\\/]|$))' -or $seen.ContainsKey($entry.path)) { throw 'CaptureFailed' }
        $file = Get-LocalBackupPath (Join-Path $Generation $entry.path)
        if (-not $file.StartsWith($Generation + '\', [StringComparison]::OrdinalIgnoreCase) -or
            $entry.sha256 -notmatch '^[A-Fa-f0-9]{64}$' -or
            (Get-Item -LiteralPath $file -Force).Length -ne $entry.bytes -or
            (Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash -cne $entry.sha256) { throw 'CaptureFailed' }
        $seen[$entry.path] = $true
    }
    foreach ($required in 'backup-configuration.json', 'cluster\backup_manifest', 'host\host.json', 'host\private\database-url',
        'host\private\token-key', 'host\private\server-key.pem', 'host\private\server-cert.pem', 'host\private\trusted-ca.pem',
        'host\app\host-service.xml', 'host\postgres\host-service.xml',
        'recovery\ca-key.encrypted.pem', 'recovery\trusted-ca.pem', 'recovery\certificate-password',
        'recovery\postgres-admin-password', 'recovery\application-password', 'recovery\identity.json',
        'recovery\server.json', 'recovery\active-tls.json') {
        if (-not $seen.ContainsKey($required)) { throw 'CaptureFailed' }
    }
    # Reject added files too. A completion marker must describe the full payload.
    foreach ($file in Get-ChildItem -LiteralPath $Generation -File -Recurse -Force) {
        $relative = $file.FullName.Substring($Generation.Length + 1)
        if ($relative -notin 'complete.json', 'incomplete.json', 'hold' -and -not $seen.ContainsKey($relative)) { throw 'CaptureFailed' }
    }
    Invoke-BackupTool (Join-Path $PostgresBin 'pg_verifybackup.exe') @('--exit-on-error', (Join-Path $Generation 'cluster'))
}

function Get-LocalBackupSources($Configuration) {
    $hostRoot = Get-LocalBackupPath $Configuration.hostRoot
    $recovery = Get-LocalBackupPath $Configuration.recoveryDirectory
    & $script:HostTools { param($r, $n) Assert-InstalledProtection $r $n } $hostRoot $Configuration.serviceName
    Assert-LocalBackupTree (Join-Path $hostRoot 'data')
    foreach ($relative in 'postgresql.conf','postgresql.auto.conf','pg_hba.conf','pg_ident.conf') {
        $file = Join-Path $hostRoot ('data/' + $relative)
        if (Test-Path -LiteralPath $file) {
            if (Get-Content -LiteralPath $file | Where-Object { $_ -match '^\s*(include(_if_exists|_dir)?|data_directory|hba_file|ident_file|ssl_key_file|ssl_cert_file|ssl_ca_file|ssl_crl_file|ssl_dh_params_file)\s*=?\s*[^#\s]' }) {
                throw 'CaptureFailed'
            }
        }
    }
    Assert-LocalBackupControl (Split-Path $recovery -Parent)
    $owner = (Get-Acl -LiteralPath $recovery).GetOwner([Security.Principal.SecurityIdentifier]).Value
    # The CA tool deliberately owns its directory as the creating administrator's SID.
    # SYSTEM scheduled runs must accept that identity without accepting arbitrary profile owners.
    $administrators = @(Get-LocalGroupMember -SID 'S-1-5-32-544' | ForEach-Object { $_.SID.Value })
    if ($owner -notin $administrators) { throw 'CaptureFailed' }
    $tls = Join-Path $recovery 'tls'
    Assert-LocalBackupTree $tls
    foreach ($item in @((Get-Item -LiteralPath $recovery -Force)) + @(Get-ChildItem -LiteralPath $recovery -Force | Where-Object Name -ne 'cluster') +
        @(Get-ChildItem -LiteralPath $tls -Force -Recurse)) {
        if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'CaptureFailed' }
        foreach ($rule in (Get-Acl -LiteralPath $item.FullName).GetAccessRules($true,$true,[Security.Principal.SecurityIdentifier])) {
            if ($rule.AccessControlType -eq 'Allow' -and $rule.IdentityReference.Value -notin $owner,'S-1-5-18') { throw 'CaptureFailed' }
        }
    }
    foreach ($relative in 'ca-key.encrypted.pem', 'trusted-ca.pem', 'identity.json', 'certificate-password',
        'postgres-admin-password', 'application-password', 'server.json', 'active-tls.json') {
        if (-not (Test-Path -LiteralPath (Join-Path $recovery $relative) -PathType Leaf)) { throw 'CaptureFailed' }
    }
    & $script:HostTools { param($r, $n) Read-HostManifest $r $n } $hostRoot $Configuration.serviceName
}

function Invoke-LocalServerCapture($Configuration, [string]$Id) {
    $settings = Get-LocalBackupSources $Configuration
    $root = Get-LocalBackupRoot $Configuration $false $true
    $target = Get-LocalGeneration $root $Id
    if (Test-Path -LiteralPath $target) { throw 'CaptureFailed' }
    $published = $target
    $pending = Join-Path $root ('.p-' + [Guid]::NewGuid().ToString('N').Substring(0,8))
    if (Test-Path -LiteralPath $pending) { throw 'CaptureFailed' }
    # Leave room for streamed WAL and growth; do not delete backups to make space.
    $bytes = (Get-TreeBytes (Join-Path $Configuration.hostRoot 'data')) +
        (Get-TreeBytes (Join-Path $Configuration.hostRoot 'private')) + (Get-TreeBytes $Configuration.recoveryDirectory)
    Assert-BackupCapacity $root ($bytes * 2) $Configuration.reserveBytes
    $hostLock = $null; $caLock = $null
    try {
        $hostLock = [IO.File]::Open((Join-Path $Configuration.hostRoot 'host.lock'), 'OpenOrCreate', 'ReadWrite', 'None')
        $caLock = [IO.File]::Open((Join-Path $Configuration.recoveryDirectory 'certificate.lock'), 'OpenOrCreate', 'ReadWrite', 'None')
        New-LocalBackupDirectory $pending
        $target = Join-Path $pending $Id
        New-LocalBackupDirectory $target
        Write-LocalBackupJson (Join-Path $target 'incomplete.json') @{ version = 1; id = $Id }
        Write-LocalBackupJson (Join-Path $target 'backup-configuration.json') $Configuration
        $hostTarget = Join-Path $target 'host'
        New-LocalBackupDirectory $hostTarget
        foreach ($relative in 'host.json', 'private', 'manifests') {
            $source = Join-Path $Configuration.hostRoot $relative
            Assert-LocalBackupTree $source
            Copy-Item -LiteralPath $source -Destination $hostTarget -Recurse
        }
        foreach ($kind in 'app', 'postgres') {
            $folder = Join-Path $hostTarget $kind; New-LocalBackupDirectory $folder
            Copy-Item -LiteralPath (Join-Path $Configuration.hostRoot ($kind + '/host-service.xml')) -Destination $folder
        }
        $recoveryTarget = Join-Path $target 'recovery'; New-LocalBackupDirectory $recoveryTarget
        foreach ($relative in 'ca-key.encrypted.pem', 'trusted-ca.pem', 'identity.json', 'certificate-password',
            'postgres-admin-password', 'application-password', 'server.json', 'active-tls.json', 'tls') {
            Copy-Item -LiteralPath (Join-Path $Configuration.recoveryDirectory $relative) -Destination $recoveryTarget -Recurse
        }
        $password = Get-Content -LiteralPath (Join-Path $Configuration.recoveryDirectory 'postgres-admin-password') -Raw
        $credential = New-Object pscredential('postgres', (ConvertTo-SecureString $password -AsPlainText -Force))
        $password = $null
        $bin = Join-Path $Configuration.hostRoot 'postgres/bin'
        Backup-Postgres $bin $settings.databasePort $credential (Join-Path $target 'cluster')
        $credential = $null
        Assert-BackupCapacity $root 0 $Configuration.reserveBytes
        Write-LocalBackupManifest $target $Configuration $Id
        Test-LocalBackupGeneration $target $Configuration.deploymentId $bin
        [void](Get-LocalBackupPath $published)
        Move-Item -LiteralPath $target -Destination $published
        [IO.Directory]::Delete($pending, $false)
    } catch {
        # A verification failure after publication must never leave a usable completion marker.
        if (Test-Path -LiteralPath (Join-Path $target 'complete.json')) {
            Move-Item -LiteralPath (Join-Path $target 'complete.json') -Destination (Join-Path $target 'failed-verification.json')
        }
        Assert-BackupCapacity $root 0 $Configuration.reserveBytes
        throw
    } finally {
        if ($caLock) { $caLock.Dispose() }; if ($hostLock) { $hostLock.Dispose() }
    }
}

function Copy-LocalServerBackup($Configuration, [string]$Id) {
    $source = Get-LocalGeneration (Get-LocalBackupRoot $Configuration) $Id
    $bin = Join-Path $Configuration.hostRoot 'postgres/bin'
    Test-LocalBackupGeneration $source $Configuration.deploymentId $bin
    $root = Get-LocalBackupRoot $Configuration $true $true
    $target = Get-LocalGeneration $root $Id
    if (Test-Path -LiteralPath $target) {
        # A completed prior copy can be accepted after interrupted status publication.
        Test-LocalBackupGeneration $target $Configuration.deploymentId $bin
        return
    }
    Assert-BackupCapacity $root (Get-TreeBytes $source) $Configuration.reserveBytes
    $published = $target
    $pending = Join-Path $root ('.p-' + [Guid]::NewGuid().ToString('N').Substring(0,8))
    if (Test-Path -LiteralPath $pending) { throw 'CaptureFailed' }
    $target = Join-Path $pending $Id
    $temporary = Join-Path $target 'complete.json'
    try {
        New-LocalBackupDirectory $pending
        New-LocalBackupDirectory $target
        foreach ($file in Get-ChildItem -LiteralPath $source -Force | Where-Object Name -notin 'complete.json', 'hold') {
            Copy-Item -LiteralPath $file.FullName -Destination $target -Recurse
        }
        # Verify the copy before publishing its completion marker.
        Copy-Item -LiteralPath (Join-Path $source 'complete.json') -Destination $temporary
        Test-LocalBackupGeneration $target $Configuration.deploymentId $bin
        Assert-BackupCapacity $root 0 $Configuration.reserveBytes
        [void](Get-LocalBackupPath $published)
        Move-Item -LiteralPath $target -Destination $published
        [IO.Directory]::Delete($pending, $false)
    } catch {
        if (Test-Path -LiteralPath $temporary) { Move-Item -LiteralPath $temporary -Destination (Join-Path $target 'failed-verification.json') }
        Assert-BackupCapacity $root 0 $Configuration.reserveBytes
        throw
    }
}

function Get-LocalBackupInventory([string]$Root, [string]$Deployment) {
    $ids = @()
    foreach ($item in Get-ChildItem -LiteralPath $Root -Directory -Force) {
        if ($item.Name -notmatch '^\d{20}-[a-f0-9]{32}$' -or (Test-Path -LiteralPath (Join-Path $item.FullName 'hold'))) { continue }
        try { [void](Get-LocalBackupManifest $item.FullName $Deployment); $ids += $item.Name } catch { continue }
        if ($ids.Count -gt 1024) { throw 'RetentionFailed' }
    }
    return $ids
}

function Remove-LocalBackupGeneration([string]$Root, [string]$Deployment, [string]$Id) {
    $path = Get-LocalGeneration $Root $Id
    [void](Get-LocalBackupManifest $path $Deployment)
    if (Test-Path -LiteralPath (Join-Path $path 'hold')) { throw 'RetentionFailed' }
    # Exact direct child and all descendants were checked for reparse points above.
    Remove-Item -LiteralPath $path -Recurse -Force
}

function Register-LocalBackupSchedule($Configuration, [string]$StateDirectory, [string]$Backend) {
    Assert-LocalBackupControl $Backend
    foreach ($name in 'local-backup.ps1', 'local-backup.psm1', 'backup-tools.psm1', 'backup.psm1', 'host.psm1') {
        Assert-LocalBackupControl (Join-Path $PSScriptRoot $name)
    }
    $name = 'EitmadServerBackup-' + $Configuration.deploymentId
    $arguments = 'windows-backup tick "' + $StateDirectory + '"'
    $existing = Get-ScheduledTask -TaskName $name -ErrorAction SilentlyContinue
    if ($existing) {
        if ($existing.Actions.Execute -cne $Backend -or $existing.Actions.Arguments -cne $arguments -or
            $existing.Principal.UserId -notin 'SYSTEM', 'S-1-5-18' -or $existing.State -eq 'Disabled') { throw 'ScheduleFailed' }
        return
    }
    $action = New-ScheduledTaskAction -Execute $Backend -Argument $arguments
    $trigger = New-ScheduledTaskTrigger -Once -At (Get-Date).AddMinutes(1) -RepetitionInterval (New-TimeSpan -Hours 1)
    $settings = New-ScheduledTaskSettingsSet -StartWhenAvailable -MultipleInstances IgnoreNew -ExecutionTimeLimit (New-TimeSpan -Hours 3)
    Register-ScheduledTask -TaskName $name -Action $action -Trigger $trigger -Settings $settings -User SYSTEM -RunLevel Highest | Out-Null
}

function Read-LocalBackupSchedule($Configuration, [string]$StateDirectory, [string]$Backend) {
    $task = Get-ScheduledTask -TaskName ('EitmadServerBackup-' + $Configuration.deploymentId) -ErrorAction SilentlyContinue
    if (-not $task -or @($task.Actions).Count -ne 1 -or $task.Actions.Execute -cne $Backend -or
        $task.Actions.Arguments -cne ('windows-backup tick "' + $StateDirectory + '"') -or
        $task.Principal.UserId -notin 'SYSTEM','S-1-5-18' -or $task.State -eq 'Disabled') { throw 'ScheduleFailed' }
}

function Restore-LocalServerBackup([string]$Generation, [string]$PostgresBin, [string]$StageDirectory) {
    & $script:HostTools { Assert-Administrator }
    $source = Get-LocalBackupPath $Generation
    $id = Split-Path $source -Leaf
    if ($id -notmatch '^\d{20}-[a-f0-9]{32}$') { throw 'CaptureFailed' }
    $parent = Split-Path (Split-Path $source -Parent) -Leaf
    if ($parent -notmatch '^server-v1-([a-f0-9-]{36})$') { throw 'CaptureFailed' }
    $deployment = $Matches[1]
    Assert-LocalBackupControl $source
    Test-LocalBackupGeneration $source $deployment $PostgresBin
    $stage = Get-LocalBackupPath $StageDirectory
    if ($stage.StartsWith($source + '\', [StringComparison]::OrdinalIgnoreCase) -or
        $source.StartsWith($stage + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'CaptureFailed' }
    if (Test-Path -LiteralPath $stage) { throw 'CaptureFailed' }
    $stageParent = Split-Path $stage -Parent
    Assert-LocalBackupControl $stageParent
    Assert-BackupCapacity $stageParent (Get-TreeBytes $source) 1073741824
    New-LocalBackupDirectory $stage
    foreach ($relative in 'cluster', 'host', 'recovery', 'backup-configuration.json') {
        Copy-Item -LiteralPath (Join-Path $source $relative) -Destination $stage -Recurse
    }
    # Verify staged bytes independently without activating services or altering source paths.
    foreach ($entry in (Get-LocalBackupManifest $source $deployment).files) {
        $file = Join-Path $stage $entry.path
        if ((Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash -cne $entry.sha256) { throw 'CaptureFailed' }
    }
    Invoke-BackupTool (Join-Path $PostgresBin 'pg_verifybackup.exe') @('--exit-on-error', (Join-Path $stage 'cluster'))
    # The existing CA renewal tool requires the current administrator's private directory.
    $caRoot = Join-Path $stage 'recovery'
    $sid = [Security.Principal.WindowsIdentity]::GetCurrent().User
    $acl = New-Object Security.AccessControl.DirectorySecurity
    $acl.SetOwner($sid); $acl.SetAccessRuleProtection($true,$false)
    foreach ($identity in $sid, (New-Object Security.Principal.SecurityIdentifier('S-1-5-18'))) {
        $acl.AddAccessRule((New-Object Security.AccessControl.FileSystemAccessRule($identity,'FullControl','ContainerInherit,ObjectInherit','None','Allow')))
    }
    Set-Acl -LiteralPath $caRoot -AclObject $acl
    foreach ($item in Get-ChildItem -LiteralPath $caRoot -Force -Recurse) {
        $acl = if ($item.PSIsContainer) { New-Object Security.AccessControl.DirectorySecurity } else { New-Object Security.AccessControl.FileSecurity }
        $acl.SetOwner($sid); $acl.SetAccessRuleProtection($false,$false)
        Set-Acl -LiteralPath $item.FullName -AclObject $acl
    }
}

function Invoke-LocalBackupAction([string]$Action, [string]$StateDirectory, [string]$Backend, $Request) {
    if ($Action -eq 'SetupDestination') {
        & $script:HostTools { Assert-Administrator }
        Assert-BackupPathSeparation $Request.configuration $StateDirectory
        New-LocalBackupDirectory $Request.configuration.localDestination
        return @()
    }
    & $script:HostTools { Assert-Administrator }
    $state = Get-LocalBackupPath $StateDirectory
    if ($Action -eq 'Initialize') { New-LocalBackupDirectory $state }
    Assert-LocalBackupControl $state
    if ($Action -in 'Initialize', 'Guard') { return @() }
    if ($Action -eq 'ManagementBackend') {
        Assert-LocalBackupControl $Request.backend
        $root = Split-Path (Split-Path (Split-Path $Request.backend -Parent) -Parent) -Parent
        Assert-LocalBackupControl $root
        $verifier = Join-Path $PSScriptRoot 'verify-offline.ps1'
        @{root=$root; anchor=$Backend; publisher=''} | ConvertTo-Json -Compress |
            & "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -NonInteractive -File $verifier *> $null
        if ($LASTEXITCODE -ne 0) { throw 'CaptureFailed' }
        return @()
    }
    if ($Action -eq 'Publish') {
        if ($Request.temporary -notmatch '^[a-f0-9-]{36}\.tmp$' -or $Request.event -notmatch '^[a-zA-Z]{1,32}$') { throw 'CaptureFailed' }
        $temporary = Join-Path $state $Request.temporary
        Assert-LocalBackupControl $temporary
        $target = Join-Path $state 'state.json'
        # Persist the sanitized operational audit before publication; a failed append blocks mutation.
        [IO.File]::AppendAllText((Join-Path $state 'audit.jsonl'), ((@{version=1; event=$Request.event; at=$Request.at;
            actor=[Security.Principal.WindowsIdentity]::GetCurrent().User.Value; status=$Request.status} | ConvertTo-Json -Depth 8 -Compress) + "`n"))
        if (Test-Path -LiteralPath $target) { [IO.File]::Replace($temporary, $target, [NullString]::Value) }
        else { [IO.File]::Move($temporary, $target) }
        return @()
    }
    $config = $Request.configuration
    switch ($Action) {
        Validate {
            Assert-BackupPathSeparation $config $state
            [void](Get-LocalBackupSources $config)
            [void](Get-LocalBackupRoot $config $false $true)
            if ($config.externalDestination) { [void](Get-LocalBackupRoot $config $true $true) }
        }
        Capture { Invoke-LocalServerCapture $config $Request.id }
        Copy { Copy-LocalServerBackup $config $Request.id }
        Probe {
            $generation = Get-LocalGeneration (Get-LocalBackupRoot $config $Request.external) $Request.id
            Test-LocalBackupGeneration $generation $config.deploymentId (Join-Path $config.hostRoot 'postgres/bin')
        }
        List { return @(Get-LocalBackupInventory (Get-LocalBackupRoot $config $Request.external) $config.deploymentId) }
        Delete { Remove-LocalBackupGeneration (Get-LocalBackupRoot $config $Request.external) $config.deploymentId $Request.id }
        Schedule { Register-LocalBackupSchedule $config $state $Backend }
        ReadSchedule { Read-LocalBackupSchedule $config $state $Backend }
        default { throw 'CaptureFailed' }
    }
    return @()
}

Export-ModuleMember -Function Invoke-LocalBackupAction, Test-LocalBackupGeneration, Restore-LocalServerBackup
