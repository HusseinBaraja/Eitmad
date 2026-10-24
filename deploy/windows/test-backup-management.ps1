#Requires -Version 5.1
# Real Rust commands and authenticated local pipe; isolated, explicitly synthetic OS observations.
param([Parameter(Mandatory)][string]$Backend)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$root = Join-Path $env:TEMP ('eitmad-management-' + [Guid]::NewGuid().ToString('N'))
[void][IO.Directory]::CreateDirectory($root)
$application = Join-Path $root 'server/application'
[void][IO.Directory]::CreateDirectory((Join-Path $application 'deploy/windows'))
$worker = Join-Path $application 'eitmad-server.exe'
Copy-Item -LiteralPath $Backend -Destination $worker
$state = Join-Path $root 'EitmadBackupState'
[void][IO.Directory]::CreateDirectory($state)
$native = @'
param($Action, $StateDirectory, $Backend)
$ErrorActionPreference='Stop'
$root = [IO.Path]::GetFullPath($env:ProgramData)
if ([IO.Path]::GetFullPath($StateDirectory) -cne (Join-Path $root 'EitmadBackupState')) { exit 1 }
$request = [Console]::In.ReadToEnd() | ConvertFrom-Json
$outcome='succeeded'; $ids=@()
if (Test-Path -LiteralPath (Join-Path $root 'deny-admin')) { $outcome='captureFailed' }
else {
    switch ($Action) {
        Guard {}
        Initialize {}
        ManagementBackend { if ($request.backend -cne $Backend) { exit 1 } }
        Publish {
            if ($request.temporary -notmatch '^[a-f0-9-]{36}\.tmp$') { exit 1 }
            $temporary=Join-Path $StateDirectory $request.temporary
            [IO.File]::AppendAllText((Join-Path $StateDirectory 'events'), $request.event+"`n")
            [IO.File]::Replace($temporary,(Join-Path $StateDirectory 'state.json'),[NullString]::Value)
        }
        Validate { if ($request.configuration.localDestination -like '*Rejected') { $outcome='destinationUnavailable' } }
        ReadSchedule { if (-not (Test-Path -LiteralPath (Join-Path $root 'schedule'))) { $outcome='scheduleFailed' } }
        Schedule { [IO.File]::WriteAllText((Join-Path $root 'schedule'),'synthetic schedule') }
        Capture {
            if (Test-Path -LiteralPath (Join-Path $root 'full')) { $outcome='insufficientSpace' }
            else { [IO.File]::WriteAllText((Join-Path $root $request.id),'synthetic preserved generation') }
        }
        Copy { $outcome='destinationUnavailable' }
        List {}
        Probe {}
        default { exit 1 }
    }
}
[Console]::Out.WriteLine((@{outcome=$outcome; generations=$ids} | ConvertTo-Json -Compress))
'@
[IO.File]::WriteAllText((Join-Path $application 'deploy/windows/local-backup.ps1'), $native)
$configuration = @{version=1;deploymentId='11111111-1111-4111-8111-111111111111';hostRoot='C:\SyntheticHost';
    recoveryDirectory='C:\SyntheticRecovery';serviceName='Synthetic';localDestination='C:\SyntheticLocal';externalDestination='E:\SyntheticExternal';
    intervalHours=24;retentionCount=7;reserveBytes=1073741824}
$saved = @{backend=$worker;configuration=$configuration;generation=$null;status=@{version=1;deploymentId=$configuration.deploymentId;
    lastAttemptAt=$null;lastLocalSuccessAt=$null;lastExternalSuccessAt=$null;localOutcome='neverRun';externalOutcome='neverRun';
    nextDueAt=0;overdue=$true;externalConfigured=$true;diskLossProtected=$false;scheduleOutcome='neverRun'}}
[IO.File]::WriteAllText((Join-Path $state 'state.json'), ($saved | ConvertTo-Json -Depth 8))
Add-Type -Path (Join-Path $PSScriptRoot 'installation-channel.cs')
$clientScript = @'
param($Channel, $Pipe, [int]$Broker, $Backend, $Root)
$ErrorActionPreference='Stop'
$env:ProgramData=$Root
Add-Type -Path $Channel
$connection=[EitmadInstallationChannel]::Connect($Pipe,$Broker)
try {
    $code=[EitmadInstallationChannel]::RunWorker($connection,$Backend,'windows-backup-management-worker',[Func[string,bool]]{param($line) return $false})
    exit $code
} finally { $connection.Dispose() }
'@
$client = Join-Path $root 'client.ps1'
[IO.File]::WriteAllText($client, $clientScript)
function Assert($Value, [string]$Message) { if (-not $Value) { throw $Message } }
function Request([string]$Action, $Destinations=$null, [switch]$Denied) {
    $name='eitmad-management-test-'+[Guid]::NewGuid().ToString('N')
    $pipe=[EitmadInstallationChannel]::Create($name)
    $child=$null
    try {
        $arguments=@('-NoProfile','-NonInteractive','-File',$client,'-Channel',(Join-Path $PSScriptRoot 'installation-channel.cs'),
            '-Pipe',$name,'-Broker',[string]$PID,'-Backend',$worker,'-Root',$root) | ForEach-Object { '"'+$_+'"' }
        $child=Start-Process -FilePath "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe" -ArgumentList ($arguments -join ' ') -WindowStyle Hidden -PassThru
        [EitmadInstallationChannel]::Accept($pipe,$child.Id)
        [EitmadInstallationChannel]::Send($pipe, (@{version=1;action=$Action;destinations=$Destinations} | ConvertTo-Json -Compress))
        $line=[EitmadInstallationChannel]::Receive($pipe,32768)
        if ($Denied) { Assert ($line -ceq '') 'Denied operation exposed state.' }
        else { Assert ($line -ne '') 'Management response missing.'; Assert ([EitmadInstallationChannel]::Receive($pipe,16) -ceq '') 'Unexpected extra response.' }
        $result=[EitmadInstallationChannel]::Receive($pipe,16)
        Assert ($result -ceq $(if ($Denied) {'failed'} else {'success'})) 'Incorrect worker result.'
        Assert ($child.WaitForExit(10000)) 'Worker did not exit.'
        if (-not $Denied) { return ($line | ConvertFrom-Json) }
    } finally { $pipe.Dispose(); if ($child) { $child.Dispose() } }
}
$initial=Request 'inspect'
Assert ($initial.status.localOutcome -ceq 'neverRun' -and $initial.status.scheduleOutcome -ceq 'scheduleFailed') 'Initial failure states lost.'
$captured=Request 'capture'
Assert ($captured.status.lastLocalSuccessAt -gt 0 -and $captured.status.localOutcome -ceq 'succeeded') 'Local success missing.'
Assert ($captured.actionOutcome -ceq 'destinationUnavailable' -and -not $captured.status.diskLossProtected) 'External failure hidden.'
$generations=@(Get-ChildItem -LiteralPath $root -File | Where-Object Name -Match '^\d{20}-[a-f0-9]{32}$')
Assert ($generations.Count -eq 1) 'Expected one synthetic generation.'
[IO.File]::WriteAllText((Join-Path $root 'full'),'synthetic capacity failure')
$failed=Request 'capture'
Assert ($failed.actionOutcome -ceq 'insufficientSpace' -and $failed.status.lastLocalSuccessAt -eq $captured.status.lastLocalSuccessAt) 'Failed capture erased last success.'
$rejected=Request 'saveDestinations' @{local='D:\Rejected';external=$null}
Assert ($rejected.actionOutcome -ceq 'destinationUnavailable' -and $rejected.destinations.local -ceq $configuration.localDestination) 'Rejected destination was saved.'
$changed=Request 'saveDestinations' @{local='D:\SyntheticNew';external=$null}
Assert ($changed.status.localOutcome -ceq 'neverRun' -and $null -eq $changed.status.lastLocalSuccessAt -and -not $changed.status.diskLossProtected) 'Changed destination retained old evidence.'
Assert ([IO.File]::ReadAllText($generations[0].FullName) -ceq 'synthetic preserved generation') 'Destination change damaged an old generation.'
$scheduled=Request 'repairSchedule'
Assert ($scheduled.status.scheduleOutcome -ceq 'succeeded') 'Schedule repair result missing.'
$hash=(Get-FileHash -LiteralPath (Join-Path $state 'state.json')).Hash
[IO.File]::WriteAllText((Join-Path $root 'deny-admin'),'synthetic Windows denial')
Request 'capture' -Denied
Assert ((Get-FileHash -LiteralPath (Join-Path $state 'state.json')).Hash -ceq $hash) 'Denied request changed state.'
Assert ((Get-Content -LiteralPath (Join-Path $state 'events')) -contains 'configured') 'Rust configuration audit boundary was skipped.'
Write-Host 'PASS: real Rust management and private pipe, separate copy failure, capacity failure, rejected/changed destinations, preserved generations, schedule repair, and denied state access.'
Write-Host 'Windows privilege, release trust, encrypted disks, capture bytes, and scheduler observations were simulated; no UAC, live data, or tasks changed.'
Write-Host ('Synthetic evidence: '+$root)
