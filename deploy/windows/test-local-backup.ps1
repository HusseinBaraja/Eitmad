#Requires -Version 5.1
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
Import-Module (Join-Path $PSScriptRoot 'local-backup.psm1') -Force -DisableNameChecking
$module = Get-Module local-backup
$root = Join-Path $env:TEMP ('eitmad-local-backup-' + [Guid]::NewGuid().ToString('N'))
[void][IO.Directory]::CreateDirectory($root)
& $module {
    param($Root)
    function Assert($Value, [string]$Message) { if (-not $Value) { throw $Message } }
    function Reject([scriptblock]$Operation) {
        $failed = $false; try { & $Operation } catch { $failed = $true }
        Assert $failed 'Expected rejection.'
    }
    $principal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
    if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        Reject { Invoke-LocalBackupAction 'Initialize' (Join-Path $Root 'unauthorized') '' @{} }
        Assert (-not (Test-Path -LiteralPath (Join-Path $Root 'unauthorized'))) 'Unauthorized call wrote state.'
        foreach ($action in 'Guard', 'ManagementBackend', 'Capture', 'Schedule', 'Validate') {
            # An Owner-shaped request must never substitute for the Windows token.
            Reject { Invoke-LocalBackupAction $action $Root '' @{owner=$true; backend='synthetic'} }
        }
    }
    # Writable local NTFS storage works without a BitLocker provider or encrypted volume.
    $script:BackupFileSystem = 'NTFS'; $script:BackupDriveLetter = 'Z'
    function Get-Volume { param($FilePath)
        [pscustomobject]@{FileSystem=$script:BackupFileSystem;DriveLetter=$script:BackupDriveLetter;SizeRemaining=1024} }
    function Get-Partition { param($DriveLetter) [pscustomobject]@{DiskNumber=7} }
    function Get-BitLockerVolume { throw 'Disk encryption is optional; the provider must not be required.' }
    $plainDisk = Get-BackupDisk $Root
    Assert ($plainDisk.disk -eq 7 -and $plainDisk.free -eq 1024) 'Plain NTFS storage was refused.'
    $script:BackupFileSystem = 'exFAT'; Reject { Get-BackupDisk $Root }
    $script:BackupFileSystem = 'NTFS'; $script:BackupDriveLetter = $null; Reject { Get-BackupDisk $Root }
    # Synthetic observations only. No tasks, volumes, services, or live directories are changed.
    function Get-BackupDisk { param($Path) [pscustomobject]@{ disk = 1; free = 100 } }
    Reject { Assert-BackupCapacity $Root 51 50 }
    Assert-BackupCapacity $Root 50 50
    Reject { Get-LocalBackupPath 'C:\PROGRA~1\Backups' }
    $unicodeAlias = [pscustomobject]@{hostRoot=('C:\'+[char]0xC9+'itmad'); recoveryDirectory='C:\Recovery';
        localDestination=('C:\'+[char]0xE9+'itmad\Backups');externalDestination=$null}
    Reject { Assert-BackupPathSeparation $unicodeAlias 'C:\State' }
    $config = [pscustomobject]@{localDestination=$Root; externalDestination=$Root;
        deploymentId='11111111-1111-4111-8111-111111111111'; hostRoot='C:\Synthetic'; recoveryDirectory='C:\Recovery'}
    function Assert-LocalBackupControl { param($Path) }
    function Get-Partition { param($DriveLetter) [pscustomobject]@{DiskNumber=1} }
    function Get-Disk { param($Number) [pscustomobject]@{BusType='SATA'} }
    Reject { Get-LocalBackupRoot $config $true }
    function Get-BackupDisk { param($Path) [pscustomobject]@{disk=2;free=100} }
    function Get-Disk { param($Number) [pscustomobject]@{BusType='File Backed Virtual'} }
    Reject { Get-LocalBackupRoot $config $true }
    # Exercise real inventory/path/deletion checks with a sentinel outside each candidate.
    function Generation([string]$Number, [string]$Deployment = '11111111-1111-4111-8111-111111111111') {
        $id = $Number.PadLeft(20,'0') + '-' + [Guid]::NewGuid().ToString('N')
        $path = Join-Path $Root $id; [void][IO.Directory]::CreateDirectory($path)
        Write-LocalBackupJson (Join-Path $path 'complete.json') @{version=1;kind='serverLocal';deploymentId=$Deployment;id=$id;
            files=@(@{path='cluster\backup_manifest';sha256=('A'*64);bytes=1})}
        return $path
    }
    $good = Generation '1'
    $held = Generation '2'; [IO.File]::WriteAllText((Join-Path $held 'hold'), '')
    $foreign = Generation '3' '22222222-2222-4222-8222-222222222222'
    $partial = Join-Path $Root ('4'.PadLeft(20,'0') + '-' + [Guid]::NewGuid().ToString('N'))
    [void][IO.Directory]::CreateDirectory($partial)
    $unknown = Generation '5'; [IO.File]::WriteAllText((Join-Path $unknown 'complete.json'), '{')
    $ids = @(Get-LocalBackupInventory $Root $config.deploymentId)
    Assert ($ids.Count -eq 1 -and $ids[0] -eq (Split-Path $good -Leaf)) 'Inventory included protected or unrecognized backups.'
    Reject { Remove-LocalBackupGeneration $Root $config.deploymentId (Split-Path $held -Leaf) }
    Reject { Get-LocalGeneration $Root '..\outside' }
    $outside = Join-Path $Root 'outside'; [void][IO.Directory]::CreateDirectory($outside)
    [IO.File]::WriteAllText((Join-Path $outside 'sentinel'), 'preserve')
    [void](New-Item -ItemType Junction -Path (Join-Path $good 'link') -Value $outside)
    Reject { Remove-LocalBackupGeneration $Root $config.deploymentId (Split-Path $good -Leaf) }
    Assert (Test-Path -LiteralPath (Join-Path $outside 'sentinel')) 'Deletion followed a junction.'
    $expired = Generation '6'
    Remove-LocalBackupGeneration $Root $config.deploymentId (Split-Path $expired -Leaf)
    Assert (-not (Test-Path -LiteralPath $expired)) 'Recognized expired generation was retained.'
    foreach ($path in $held,$foreign,$partial,$unknown,$outside) { Assert (Test-Path -LiteralPath $path) 'Retention removed preserved evidence.' }
    # Scheduling is observed through mocks; no task is installed on this machine.
    function New-ScheduleFixture {
        [pscustomobject]@{
            TaskName=('EitmadServerBackup-' + $config.deploymentId); TaskPath='\'; State='Ready'
            Actions=@([pscustomobject]@{Execute='C:\Synthetic\eitmad-server.exe';Arguments=('windows-backup tick "' + $Root + '"')})
            Principal=[pscustomobject]@{UserId='SYSTEM'}
            Triggers=@([pscustomobject]@{CimClass=@{CimClassName='MSFT_TaskTimeTrigger'}; Enabled=$true
                StartBoundary='2026-10-21T12:00:00'; EndBoundary=''
                Repetition=[pscustomobject]@{Interval='PT1H';Duration='';StopAtDurationEnd=$false}})
            Settings=[pscustomobject]@{StartWhenAvailable=$true;MultipleInstances=2;ExecutionTimeLimit='PT3H'}
        }
    }
    function ScheduleFailed([scriptblock]$Operation) {
        $message = ''; try { & $Operation } catch { $message = $_.Exception.Message }
        Assert ($message -ceq 'ScheduleFailed') ('Expected ScheduleFailed, got: ' + $message)
    }
    $script:ScheduledTask = $null; $script:Registrations = 0; $script:Repairs = 0; $script:ApplyScheduleRepair = $true
    function Get-ScheduledTask { param($TaskName, $TaskPath, $ErrorAction)
        Assert ($TaskName -ceq ('EitmadServerBackup-' + $config.deploymentId) -and $TaskPath -ceq '\') 'Lookup escaped the owned task path.'
        return $script:ScheduledTask }
    function New-ScheduledTaskAction { param($Execute,$Argument) [pscustomobject]@{Execute=$Execute;Arguments=$Argument} }
    function New-ScheduledTaskTrigger { param([switch]$Once,$At,$RepetitionInterval)
        Assert ($Once -and $At -gt (Get-Date) -and $RepetitionInterval.TotalHours -eq 1) 'Missing bounded catch-up tick.'
        return (New-ScheduleFixture).Triggers[0] }
    function New-ScheduledTaskSettingsSet { param([switch]$StartWhenAvailable,$MultipleInstances,$ExecutionTimeLimit)
        Assert ($StartWhenAvailable -and $MultipleInstances -eq 'IgnoreNew' -and $ExecutionTimeLimit.TotalHours -eq 3) 'Unsafe scheduler settings.'
        return (New-ScheduleFixture).Settings }
    function Register-ScheduledTask { param($TaskName,$TaskPath,$Action,$Trigger,$Settings,$User,$RunLevel)
        Assert ($TaskName -ceq ('EitmadServerBackup-' + $config.deploymentId) -and $TaskPath -ceq '\' -and
            $User -eq 'SYSTEM' -and $RunLevel -eq 'Highest' -and $Action.Execute -ceq 'C:\Synthetic\eitmad-server.exe' -and
            $Action.Arguments -ceq ('windows-backup tick "' + $Root + '"')) 'Task did not invoke Rust policy.'
        $script:Registrations++; $script:ScheduledTask = New-ScheduleFixture
        $script:ScheduledTask.Actions = @($Action); $script:ScheduledTask.Triggers = @($Trigger); $script:ScheduledTask.Settings = $Settings }
    function Set-ScheduledTask { param($TaskName,$TaskPath,$Trigger,$Settings)
        Assert ($TaskName -ceq $script:ScheduledTask.TaskName -and $TaskPath -ceq $script:ScheduledTask.TaskPath) 'Repair changed a different task.'
        $script:Repairs++
        if ($script:ApplyScheduleRepair) { $script:ScheduledTask.Triggers = @($Trigger); $script:ScheduledTask.Settings = $Settings } }
    ScheduleFailed { Read-LocalBackupSchedule $config $Root 'C:\Synthetic\eitmad-server.exe' }
    Register-LocalBackupSchedule $config $Root 'C:\Synthetic\eitmad-server.exe'
    Read-LocalBackupSchedule $config $Root 'C:\Synthetic\eitmad-server.exe'
    Register-LocalBackupSchedule $config $Root 'C:\Synthetic\eitmad-server.exe'
    Assert ($script:Registrations -eq 1 -and $script:Repairs -eq 0) 'Healthy registration was not idempotent.'
    # Accept equivalent XML durations and the enum's displayed value as well as its CIM integer.
    $script:ScheduledTask.Triggers[0].Repetition.Interval = 'PT60M'
    $script:ScheduledTask.Settings.ExecutionTimeLimit = 'PT180M'; $script:ScheduledTask.Settings.MultipleInstances = 'IgnoreNew'
    Read-LocalBackupSchedule $config $Root 'C:\Synthetic\eitmad-server.exe'
    $invalidSchedules = @(
        @{name='missing trigger'; change={param($t) $t.Triggers=@()}},
        @{name='disabled trigger'; change={param($t) $t.Triggers[0].Enabled=$false}},
        @{name='incorrect cadence'; change={param($t) $t.Triggers[0].Repetition.Interval='PT2H'}},
        @{name='malformed cadence'; change={param($t) $t.Triggers[0].Repetition.Interval='invalid'}},
        @{name='finite repetition'; change={param($t) $t.Triggers[0].Repetition.Duration='P1D'}},
        @{name='expiring trigger'; change={param($t) $t.Triggers[0].EndBoundary='2026-10-22T12:00:00'}},
        @{name='incorrect trigger kind'; change={param($t) $t.Triggers[0].CimClass.CimClassName='MSFT_TaskLogonTrigger'}},
        @{name='extra trigger'; change={param($t) $t.Triggers += $t.Triggers[0]}},
        @{name='missing catch-up'; change={param($t) $t.Settings.StartWhenAvailable=$false}},
        @{name='parallel instances'; change={param($t) $t.Settings.MultipleInstances=0}},
        @{name='missing settings'; change={param($t) $t.Settings=$null}},
        @{name='incorrect execution limit'; change={param($t) $t.Settings.ExecutionTimeLimit='PT2H'}},
        @{name='malformed execution limit'; change={param($t) $t.Settings.ExecutionTimeLimit='invalid'}},
        @{name='unlimited execution'; change={param($t) $t.Settings.ExecutionTimeLimit='PT0S'}}
    )
    foreach ($case in $invalidSchedules) {
        $script:ScheduledTask = New-ScheduleFixture; & $case.change $script:ScheduledTask
        $repairs = $script:Repairs
        ScheduleFailed { Read-LocalBackupSchedule $config $Root 'C:\Synthetic\eitmad-server.exe' }
        Assert ($script:Repairs -eq $repairs -and $script:Registrations -eq 1) ('Status repaired ' + $case.name)
        $action = $script:ScheduledTask.Actions; $principal = $script:ScheduledTask.Principal
        Register-LocalBackupSchedule $config $Root 'C:\Synthetic\eitmad-server.exe'
        Read-LocalBackupSchedule $config $Root 'C:\Synthetic\eitmad-server.exe'
        Assert ($script:Repairs -eq ($repairs + 1) -and $script:Registrations -eq 1 -and
            [object]::ReferenceEquals($action,$script:ScheduledTask.Actions) -and
            [object]::ReferenceEquals($principal,$script:ScheduledTask.Principal)) ('Unsafe repair: ' + $case.name)
    }
    foreach ($change in @(
        {param($t) $t.Actions[0].Execute='foreign.exe'},
        {param($t) $t.Actions[0].Arguments='foreign arguments'},
        {param($t) $t.Actions += $t.Actions[0]},
        {param($t) $t.Principal.UserId='foreign-user'},
        {param($t) $t.State='Disabled'}
    )) {
        $script:ScheduledTask = New-ScheduleFixture; & $change $script:ScheduledTask
        $script:ScheduledTask.Triggers = @(); $repairs = $script:Repairs
        ScheduleFailed { Read-LocalBackupSchedule $config $Root 'C:\Synthetic\eitmad-server.exe' }
        ScheduleFailed { Register-LocalBackupSchedule $config $Root 'C:\Synthetic\eitmad-server.exe' }
        Assert ($script:Repairs -eq $repairs -and $script:Registrations -eq 1) 'Refused task was changed.'
    }
    $script:ScheduledTask = New-ScheduleFixture; $script:ScheduledTask.Triggers = @()
    $script:ApplyScheduleRepair = $false
    ScheduleFailed { Register-LocalBackupSchedule $config $Root 'C:\Synthetic\eitmad-server.exe' }
    # Refuse writing a restore stage inside its own source, before creating any directory.
    & $script:HostTools { function script:Assert-Administrator {} }
    function Test-LocalBackupGeneration { param($Generation,$Deployment,$PostgresBin) }
    $managed = Join-Path $Root ('server-v1-' + $config.deploymentId)
    [void][IO.Directory]::CreateDirectory($managed)
    $source = Join-Path $managed ('7'.PadLeft(20,'0') + '-' + [Guid]::NewGuid().ToString('N'))
    [void][IO.Directory]::CreateDirectory($source)
    Reject { Restore-LocalServerBackup $source 'C:\Synthetic' (Join-Path $source 'stage') }
    Assert (-not (Test-Path -LiteralPath (Join-Path $source 'stage'))) 'Restore wrote into its source backup.'
    # A full disk during external payload copy reports capacity failure and retains the source.
    [IO.File]::WriteAllText((Join-Path $source 'payload'), 'synthetic preserved backup')
    $script:CopySourceRoot = $managed; $script:CopyTargetRoot = Join-Path $Root 'copy-target'
    [void][IO.Directory]::CreateDirectory($script:CopyTargetRoot)
    $config | Add-Member NoteProperty reserveBytes 1073741824
    function Get-LocalBackupRoot { param($Configuration,$External,$Create)
        if ($External) { return $script:CopyTargetRoot }; return $script:CopySourceRoot }
    function Get-TreeBytes { param($Path) return 1 }
    $script:SpaceChecks = 0
    function Get-BackupDisk { param($Path)
        $script:SpaceChecks++
        [pscustomobject]@{disk=1;free=$(if ($script:SpaceChecks -eq 1) {1073741825} else {0})} }
    function New-LocalBackupDirectory { param($Path) [void][IO.Directory]::CreateDirectory($Path) }
    function Copy-Item { param($LiteralPath,$Destination,[switch]$Recurse) throw 'synthetic disk full' }
    try { Copy-LocalServerBackup $config (Split-Path $source -Leaf); throw 'Expected copy failure.' }
    catch { Assert ($_.Exception.Message -ceq 'InsufficientSpace') 'Mid-copy capacity failure lost its typed status.' }
    Assert ((Get-Content -LiteralPath (Join-Path $source 'payload') -Raw) -ceq 'synthetic preserved backup') 'Failed copy changed its source.'
    Write-Host 'PASS: capacity reserve and mid-copy disk failure, same/virtual disk rejection, held/foreign/partial preservation, junction refusal, scheduler policy, unauthorized writes, and restore-source protection.'
} $root
Write-Host ('Synthetic evidence: ' + $root)
