Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Assert-BackupControl([string]$Path) {
    $full = [IO.Path]::GetFullPath($Path)
    for ($part = $full; $part; $part = Split-Path $part -Parent) {
        $item = Get-Item -LiteralPath $part -Force
        if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Backup control paths cannot use reparse points.' }
        $acl = Get-Acl -LiteralPath $part
        $trusted = @('S-1-5-18', 'S-1-5-32-544', [Security.Principal.WindowsIdentity]::GetCurrent().User.Value,
            'S-1-5-80-956008885-3418522649-1831038044-1853292631-2271478464')
        if ($acl.GetOwner([Security.Principal.SecurityIdentifier]).Value -notin $trusted) { throw 'Untrusted backup control owner.' }
        foreach ($rule in $acl.GetAccessRules($true, $true, [Security.Principal.SecurityIdentifier])) {
            if ($rule.AccessControlType -ne 'Allow' -or $rule.IdentityReference.Value -in $trusted -or
                ($rule.PropagationFlags -band [Security.AccessControl.PropagationFlags]::InheritOnly)) { continue }
            $write = [Security.AccessControl.FileSystemRights]::Delete -bor
                [Security.AccessControl.FileSystemRights]::DeleteSubdirectoriesAndFiles -bor
                [Security.AccessControl.FileSystemRights]::ChangePermissions -bor [Security.AccessControl.FileSystemRights]::TakeOwnership
            if ($part -eq $full -or ($rule.FileSystemRights -band $write)) { throw 'Backup controls or their parents permit untrusted access.' }
        }
    }
}

function Read-BackupPlan([string]$Path) {
    Assert-BackupControl $Path
    $plan = Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
    if ($plan.version -ne 1 -or $plan.deploymentId -notmatch '^[a-f0-9]{32}$' -or
        $plan.recoveryKitId -notmatch '^[A-Za-z0-9-]{1,80}$' -or
        $plan.share -notmatch '^\\\\[^\\]+\\[^\\]+$' -or $plan.time -notmatch '^\d\d:\d\d$' -or
        $plan.retention.daily -lt 1 -or $plan.retention.weekly -lt 1 -or $plan.rpoHours -lt 24 -or $plan.rtoHours -le 0 -or
        $plan.intervalHours -lt 1 -or $plan.intervalHours + 3 -ge $plan.rpoHours) {
        throw 'Invalid backup plan or retention policy.'
    }
    [void][DateTime]::ParseExact($plan.time, 'HH:mm', [Globalization.CultureInfo]::InvariantCulture)
    if (@($plan.nodes | Where-Object kind -eq 'server').Count -ne 1 -or @($plan.nodes).Count -lt 2 -or
        @($plan.nodes | Group-Object id | Where-Object Count -gt 1).Count) { throw 'Specify one server and every independent client.' }
    foreach ($node in $plan.nodes) {
        if ($node.id -notmatch '^[a-z][a-z0-9-]{0,31}$' -or $node.kind -notin 'server', 'client', 'operator' -or
            -not @($node.volumes).Count -or -not @($node.requiredPaths).Count -or
            ($node.kind -eq 'client' -and -not @($node.runtimes).Count)) { throw 'Invalid backup participant.' }
        Assert-BackupControl $node.credentialFile
    }
    foreach ($path in $plan.storage.credentialFile, $plan.storageCredentialFile, $plan.postgresCredentialFile, $plan.statusDirectory) { Assert-BackupControl $path }
    return $plan
}

function Write-BackupJson([string]$Path, $Value, [switch]$Replace) {
    $temporary = $Path + '.' + [Guid]::NewGuid().ToString('N') + '.tmp'
    $json = $Value | ConvertTo-Json -Depth 12
    [IO.File]::WriteAllText($temporary, $json, (New-Object Text.UTF8Encoding($false)))
    # Only status is replaced. Generation completion uses a new path and File.Move.
    if ($Replace -and (Test-Path -LiteralPath $Path)) { [IO.File]::Replace($temporary, $Path, [NullString]::Value) }
    else { [IO.File]::Move($temporary, $Path) }
}

function Get-BackupRetention($Plan, [string]$Root) {
    $valid = @()
    foreach ($directory in Get-ChildItem -LiteralPath $Root -Directory -Force) {
        if ($directory.Name -notmatch '^\d{8}T\d{6}Z-[a-f0-9]{32}$' -or
            ($directory.Attributes -band [IO.FileAttributes]::ReparsePoint) -or
            (Test-Path -LiteralPath (Join-Path $directory.FullName 'hold'))) { continue }
        $marker = Join-Path $directory.FullName 'complete.json'
        if (-not (Test-Path -LiteralPath $marker -PathType Leaf) -or
            ((Get-Item -LiteralPath $marker -Force).Attributes -band [IO.FileAttributes]::ReparsePoint)) { continue }
        try {
            $generation = Get-Content -LiteralPath $marker -Raw | ConvertFrom-Json
            $cut = [DateTimeOffset]::ParseExact($generation.cutUtc, 'o', [Globalization.CultureInfo]::InvariantCulture)
            if ($generation.version -ne 1 -or $generation.deploymentId -cne $Plan.deploymentId -or
                $generation.id -cne $directory.Name -or $generation.status -cne 'complete' -or
                (($generation.nodes.id | Sort-Object) -join ',') -cne (($Plan.nodes.id | Sort-Object) -join ',')) { continue }
            $valid += [pscustomobject]@{ Path = $directory.FullName; Cut = $cut.UtcDateTime; Id = $directory.Name }
        } catch { continue } # Unknown/partial generations are never automatically deleted.
    }
    $ordered = @($valid | Sort-Object Cut -Descending)
    $keep = @{}
    # Retain the newest successful generation for each of the newest N UTC days and N ISO weeks.
    $days = @{}; $weeks = @{}
    foreach ($item in $ordered) {
        $day = $item.Cut.ToString('yyyy-MM-dd')
        $monday = $item.Cut.Date.AddDays(-(([int]$item.Cut.DayOfWeek + 6) % 7)).ToString('yyyy-MM-dd')
        if (-not $days.ContainsKey($day) -and $days.Count -lt $Plan.retention.daily) { $days[$day] = $true; $keep[$item.Id] = $true }
        if (-not $weeks.ContainsKey($monday) -and $weeks.Count -lt $Plan.retention.weekly) { $weeks[$monday] = $true; $keep[$item.Id] = $true }
    }
    return @($ordered | Where-Object { -not $keep.ContainsKey($_.Id) })
}

function Remove-ExpiredBackup($Plan, [string]$Root) {
    $resolvedRoot = (Get-Item -LiteralPath $Root -Force).FullName.TrimEnd('\')
    if ((Get-Item -LiteralPath $resolvedRoot -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) {
        throw 'Retention refused a redirected root.'
    }
    foreach ($candidate in Get-BackupRetention $Plan $resolvedRoot) {
        $path = (Get-Item -LiteralPath $candidate.Path -Force).FullName
        if ((Split-Path $path -Parent) -ine $resolvedRoot -or
            (Get-Item -LiteralPath $path -Force).Attributes -band [IO.FileAttributes]::ReparsePoint -or
            @(Get-ChildItem -LiteralPath $path -Force -Recurse | Where-Object { $_.Attributes -band [IO.FileAttributes]::ReparsePoint }).Count) {
            throw 'Retention refused an unsafe generation path.'
        }
        Remove-Item -LiteralPath $path -Recurse -Force
    }
}

function Open-BackupSession($Node) {
    # HTTPS certificate/name validation remains enabled; no TrustedHosts or CredSSP changes.
    New-PSSession -ComputerName $Node.computer -UseSSL -Credential (Import-Clixml -LiteralPath $Node.credentialFile) `
        -SessionOption (New-PSSessionOption -IdleTimeout 7200000 -OperationTimeout 7200000)
}

function Test-BackupStorage($Plan, $Credential) {
    $session = Open-BackupSession $Plan.storage
    try {
        Invoke-Command -Session $session -ScriptBlock {
            param($Share, $Name, $User)
            $smb = Get-SmbShare -Name $Name
            for ($part = $smb.Path; $part; $part = Split-Path $part -Parent) {
                if ((Get-Item -LiteralPath $part -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Redirected storage path.' }
            }
            if (-not $smb.EncryptData -or (Get-BitLockerVolume -MountPoint ([IO.Path]::GetPathRoot($smb.Path))).ProtectionStatus -ne 'On') {
                throw 'Backup share requires SMB encryption and BitLocker protection.'
            }
            if ($Share -ine ('\\' + $env:COMPUTERNAME + '\' + $Name)) { throw 'Use the storage computer name in the UNC share.' }
            $sid = (New-Object Security.Principal.NTAccount($User)).Translate([Security.Principal.SecurityIdentifier]).Value
            $trusted = @('S-1-5-18', 'S-1-5-32-544', $sid)
            foreach ($grant in Get-SmbShareAccess -Name $Name) {
                $accountSid = (New-Object Security.Principal.NTAccount($grant.AccountName)).Translate([Security.Principal.SecurityIdentifier]).Value
                if ($grant.AccessControlType -eq 'Allow' -and $accountSid -notin $trusted) { throw 'The backup share grants access to another principal.' }
            }
            $acl = Get-Acl -LiteralPath $smb.Path
            if ($acl.GetOwner([Security.Principal.SecurityIdentifier]).Value -notin $trusted) { throw 'Untrusted backup directory owner.' }
            foreach ($grant in $acl.GetAccessRules($true, $true, [Security.Principal.SecurityIdentifier])) {
                if ($grant.AccessControlType -eq 'Allow' -and $grant.IdentityReference.Value -notin $trusted) {
                    throw 'The backup directory grants access to another principal.'
                }
            }
            [pscustomobject]@{ machineId = (Get-CimInstance Win32_ComputerSystemProduct).UUID }
        } -ArgumentList $Plan.share, $Plan.storage.shareName, $Credential.UserName
    } finally { Remove-PSSession $session }
}

function Invoke-BackupGeneration($Plan) {
    $sessions = @{}; $prepared = @(); $released = @{}
    $mapping = $false
    $storage = Import-Clixml -LiteralPath $Plan.storageCredentialFile
    $postgres = Import-Clixml -LiteralPath $Plan.postgresCredentialFile
    $worker = Join-Path $PSScriptRoot 'backup-worker.ps1'
    $started = [DateTime]::UtcNow
    $id = $started.ToString('yyyyMMddTHHmmssZ-') + [Guid]::NewGuid().ToString('N')
    $root = Join-Path $Plan.share $Plan.deploymentId
    $destination = Join-Path $root $id
    $result = [ordered]@{ version = 1; deploymentId = $Plan.deploymentId; recoveryKitId = $Plan.recoveryKitId
        id = $id; status = 'failed'; startedUtc = $started.ToString('o') }
    try {
        $storageIdentity = Test-BackupStorage $Plan $storage
        # The coordinator also needs an SMB session. It owns exactly the mapping it creates.
        if (@(Get-SmbMapping | Where-Object RemotePath -eq $Plan.share).Count) { throw 'Backup logon already has a mapping to this share.' }
        [void](New-SmbMapping -RemotePath $Plan.share -UserName $storage.UserName `
            -Password $storage.GetNetworkCredential().Password -Persistent $false -RequirePrivacy $true)
        $mapping = $true
        [void][IO.Directory]::CreateDirectory($root)
        if ((Get-Item -LiteralPath $root -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Redirected generation root.' }
        if (Test-Path -LiteralPath $destination) { throw 'Generation destination exists.' }
        [void][IO.Directory]::CreateDirectory($destination)
        Write-BackupJson (Join-Path $destination 'incomplete.json') $result
        $identities = @()
        foreach ($node in $Plan.nodes) {
            $sessions[$node.id] = Open-BackupSession $node
            $node | Add-Member NoteProperty share $Plan.share -Force
            # Register before Prepare so partial preparation is also released.
            $prepared += $node
            $identity = Invoke-Command -Session $sessions[$node.id] -FilePath $worker -ArgumentList 'Prepare', $node, '', $storage
            if ($identity.machineId -eq $storageIdentity.machineId -or $identity.machineId -in @($identities | ForEach-Object machineId)) {
                throw 'Backup storage and participant computers must be independent machines.'
            }
            $identities += [pscustomobject]@{ id = $node.id; kind = $node.kind; machineId = $identity.machineId
                boot = $identity.boot; computer = $identity.computer; volumes = @($node.volumes)
                requiredPaths = @($node.requiredPaths); runtimes = @($node.runtimes) }
        }
        if ((Get-CimInstance Win32_ComputerSystemProduct).UUID -notin @($identities | ForEach-Object machineId)) {
            throw 'The coordinator must also be an imaged participant so its DPAPI dependencies survive.'
        }
        foreach ($node in $Plan.nodes) {
            Invoke-Command -Session $sessions[$node.id] -FilePath $worker -ArgumentList 'Freeze', $node
        }
        $result.cutUtc = [DateTime]::UtcNow.ToString('o')
        $result.nodes = $identities
        $result.retention = $Plan.retention
        foreach ($node in $Plan.nodes) {
            $target = Join-Path $destination $node.id
            [void][IO.Directory]::CreateDirectory($target)
            $databaseCredential = if ($node.kind -eq 'server') { $postgres } else { $null }
            $capture = Invoke-Command -Session $sessions[$node.id] -FilePath $worker -ArgumentList 'Capture', $node, $target, $null, $databaseCredential
            if (-not $capture.imageCatalogVerified) { throw 'Missing Windows image catalog.' }
        }
        foreach ($node in $Plan.nodes) {
            Invoke-Command -Session $sessions[$node.id] -FilePath $worker -ArgumentList 'Check', $node
        }
        foreach ($node in $Plan.nodes) {
            Invoke-Command -Session $sessions[$node.id] -FilePath $worker -ArgumentList 'Release', $node
            $released[$node.id] = $true
        }
        $server = $Plan.nodes | Where-Object kind -eq 'server'
        $manifest = Join-Path (Join-Path $destination $server.id) 'cluster/backup_manifest'
        $result.postgresManifestSha256 = (Get-FileHash -LiteralPath $manifest -Algorithm SHA256).Hash
        $result.status = 'complete'; $result.completedUtc = [DateTime]::UtcNow.ToString('o')
        $result.backupSeconds = ([DateTime]::UtcNow - $started).TotalSeconds
        Write-BackupJson (Join-Path $destination 'complete.json') $result
        Write-BackupJson (Join-Path $Plan.statusDirectory 'last-success.json') $result -Replace
        Remove-ExpiredBackup $Plan $root
        Write-EventLog -LogName Application -Source EitmadBackup -EventId 4100 -EntryType Information -Message 'LAN recovery generation completed and retention applied.'
    } finally {
        $resumeFailed = $false
        foreach ($node in $prepared) {
            if (-not $released.ContainsKey($node.id)) {
                try { Invoke-Command -Session $sessions[$node.id] -FilePath $worker -ArgumentList 'Release', $node }
                catch { $resumeFailed = $true }
            }
        }
        foreach ($session in $sessions.Values) { Remove-PSSession $session }
        if ($mapping) { Remove-SmbMapping -RemotePath $Plan.share -Force -UpdateProfile:$false }
        if ($resumeFailed) { throw 'Backup failed to resume a participant. Inspect services and close the retained remote session.' }
    }
}

function Test-BackupFreshness($Plan) {
    $file = Join-Path $Plan.statusDirectory 'last-success.json'
    if (-not (Test-Path -LiteralPath $file)) { throw 'No successful LAN backup is recorded.' }
    $status = Get-Content -LiteralPath $file -Raw | ConvertFrom-Json
    if ($status.status -cne 'complete' -or $status.deploymentId -cne $Plan.deploymentId -or
        ([DateTime]::UtcNow - [DateTime]::Parse($status.cutUtc).ToUniversalTime()).TotalHours -gt $Plan.rpoHours -or
        [DateTime]::Parse($status.cutUtc).ToUniversalTime() -gt [DateTime]::UtcNow.AddMinutes(2)) {
        throw 'LAN backup is stale; the data-loss target is exceeded.'
    }
}

function Test-BackupAvailability($Plan) {
    $credential = Import-Clixml -LiteralPath $Plan.storageCredentialFile
    $owned = $false
    try {
        [void](Test-BackupStorage $Plan $credential)
        if (-not @(Get-SmbMapping | Where-Object RemotePath -eq $Plan.share).Count) {
            [void](New-SmbMapping -RemotePath $Plan.share -UserName $credential.UserName `
                -Password $credential.GetNetworkCredential().Password -Persistent $false -RequirePrivacy $true)
            $owned = $true
        }
        $status = Get-Content -LiteralPath (Join-Path $Plan.statusDirectory 'last-success.json') -Raw | ConvertFrom-Json
        if ($status.id -notmatch '^\d{8}T\d{6}Z-[a-f0-9]{32}$') { throw 'Invalid successful generation reference.' }
        $root = Join-Path (Join-Path $Plan.share $Plan.deploymentId) $status.id
        $marker = Get-Content -LiteralPath (Join-Path $root 'complete.json') -Raw | ConvertFrom-Json
        if ($marker.status -cne 'complete' -or $marker.id -cne $status.id -or $marker.deploymentId -cne $Plan.deploymentId) {
            throw 'The successful off-device generation is unavailable.'
        }
        $server = $Plan.nodes | Where-Object kind -eq 'server'
        $hash = (Get-FileHash -LiteralPath (Join-Path (Join-Path $root $server.id) 'cluster/backup_manifest') -Algorithm SHA256).Hash
        if ($hash -cne $status.postgresManifestSha256) { throw 'The retained backup manifest changed.' }
    } finally {
        if ($owned) { Remove-SmbMapping -RemotePath $Plan.share -Force -UpdateProfile:$false }
    }
}

function Invoke-EitmadBackup {
    [CmdletBinding()]
    param([ValidateSet('Run', 'Status', 'Install')][string]$Action = 'Status', [Parameter(Mandatory)][string]$PlanPath)
    $plan = Read-BackupPlan $PlanPath
    foreach ($file in '', 'backup.psm1', 'backup-worker.ps1', 'backup-tools.psm1', 'run-backup.ps1', 'host.psm1') {
        Assert-BackupControl $(if ($file) { Join-Path $PSScriptRoot $file } else { $PSScriptRoot })
    }
    if ($Action -eq 'Install') {
        $credential = Get-Credential -UserName ([Security.Principal.WindowsIdentity]::GetCurrent().Name) -Message 'Windows backup task account'
        if ($credential.UserName -ine [Security.Principal.WindowsIdentity]::GetCurrent().Name) { throw 'Install as the task account that encrypted the credential files.' }
        if (-not [Diagnostics.EventLog]::SourceExists('EitmadBackup')) { New-EventLog -LogName Application -Source EitmadBackup }
        $entry = Join-Path $PSScriptRoot 'run-backup.ps1'
        foreach ($kind in 'Run', 'Status') {
            $name = 'EitmadBackup-' + $plan.deploymentId + '-' + $kind
            if (Get-ScheduledTask -TaskName $name -ErrorAction SilentlyContinue) { throw 'Backup task already exists. Inspect it before replacement.' }
            $arguments = '-NoProfile -NonInteractive -File "' + $entry + '" -Action ' + $kind + ' -PlanPath "' + [IO.Path]::GetFullPath($PlanPath) + '"'
            $actionValue = New-ScheduledTaskAction -Execute "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe" -Argument $arguments
            $next = (Get-Date).Date.Add([TimeSpan]::Parse($plan.time))
            if ($next -le (Get-Date)) { $next = $next.AddDays(1) }
            $trigger = if ($kind -eq 'Run') { New-ScheduledTaskTrigger -Once -At $next -RepetitionInterval (New-TimeSpan -Hours $plan.intervalHours) }
                else { New-ScheduledTaskTrigger -Once -At (Get-Date).AddMinutes(1) -RepetitionInterval (New-TimeSpan -Hours 1) }
            $settings = New-ScheduledTaskSettingsSet -StartWhenAvailable -MultipleInstances IgnoreNew -ExecutionTimeLimit (New-TimeSpan -Hours 3)
            Register-ScheduledTask -TaskName $name -Action $actionValue -Trigger $trigger -Settings $settings -RunLevel Highest `
                -User $credential.UserName -Password $credential.GetNetworkCredential().Password | Out-Null
        }
        return
    }
    $lock = [IO.File]::Open((Join-Path $plan.statusDirectory 'backup.lock'), 'OpenOrCreate', 'ReadWrite', 'None')
    try {
        try {
            if ($Action -eq 'Run') { Invoke-BackupGeneration $plan }
            Test-BackupFreshness $plan
            if ($Action -eq 'Status') { Test-BackupAvailability $plan }
            Write-Host 'LAN backup is within the configured data-loss target.'
        } catch {
            Write-BackupJson (Join-Path $plan.statusDirectory 'last-failure.json') @{ failedUtc = [DateTime]::UtcNow.ToString('o'); code = 'eitmad.backup.failed.v1' } -Replace
            Write-EventLog -LogName Application -Source EitmadBackup -EventId 4101 -EntryType Error -Message 'LAN backup failed or is stale. Inspect protected task status. Do not delete recovery data.'
            throw 'eitmad.backup.failed.v1'
        }
    } finally { $lock.Dispose() }
}

Export-ModuleMember -Function Invoke-EitmadBackup
