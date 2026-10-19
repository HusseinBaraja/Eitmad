#Requires -Version 5.1
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
Import-Module (Join-Path $PSScriptRoot 'backup.psm1') -Force
$module = Get-Module backup
$evidence = Join-Path $env:TEMP ('eitmad-backup-' + [Guid]::NewGuid().ToString('N'))
[void][IO.Directory]::CreateDirectory($evidence)
& $module {
    param($Root)
    function Assert($Condition, [string]$Message) { if (-not $Condition) { throw $Message } }
    function Reject([scriptblock]$Operation) {
        $failed = $false
        try { & $Operation } catch { $failed = $true }
        Assert $failed 'Expected failure did not occur.'
    }
    $plan = [pscustomobject]@{ deploymentId = ('a' * 32); recoveryKitId = 'synthetic-kit'; retention = @{ daily = 2; weekly = 2 }
        nodes = @(@{ id = 'server' }, @{ id = 'client' }); statusDirectory = $Root; rpoHours = 24 }
    function Generation([DateTime]$Cut, [string]$Name = '', [string]$Deployment = ('a' * 32)) {
        if (-not $Name) { $Name = $Cut.ToString('yyyyMMddTHHmmssZ-') + [Guid]::NewGuid().ToString('N') }
        $path = Join-Path $Root $Name
        [void][IO.Directory]::CreateDirectory($path)
        Write-BackupJson (Join-Path $path 'complete.json') @{
            version = 1; deploymentId = $Deployment; id = $Name; status = 'complete'; cutUtc = $Cut.ToString('o')
            nodes = @(@{ id = 'server' }, @{ id = 'client' })
        }
        [IO.File]::WriteAllText((Join-Path $path 'sentinel'), 'synthetic backup')
        return $path
    }
    $old = Generation ([DateTime]'2026-08-01T02:00:00Z')
    $week = Generation ([DateTime]'2026-10-18T02:00:00Z')
    $yesterday = Generation ([DateTime]'2026-10-19T02:00:00Z')
    $latest = Generation ([DateTime]'2026-10-20T02:00:00Z')
    $held = Generation ([DateTime]'2026-07-01T02:00:00Z')
    [IO.File]::WriteAllText((Join-Path $held 'hold'), 'operator hold')
    $foreign = Generation ([DateTime]'2026-06-01T02:00:00Z') '' ('b' * 32)
    $unknown = Generation ([DateTime]'2026-05-01T02:00:00Z')
    [IO.File]::WriteAllText((Join-Path $unknown 'complete.json'), '{')
    $partial = Join-Path $Root ('20260401T020000Z-' + [Guid]::NewGuid().ToString('N'))
    [void][IO.Directory]::CreateDirectory($partial)
    $extra = Generation ([DateTime]'2026-10-20T01:00:00Z')
    $candidates = @(Get-BackupRetention $plan $Root)
    Assert ($candidates.Count -eq 2 -and $old -in $candidates.Path -and $extra -in $candidates.Path) 'Calendar retention selected the wrong generations.'
    Remove-ExpiredBackup $plan $Root
    foreach ($path in $week, $yesterday, $latest, $held, $foreign, $unknown, $partial) {
        Assert (Test-Path -LiteralPath $path) 'Retention removed protected, unknown, or needed history.'
    }
    Assert (-not (Test-Path -LiteralPath $old) -and -not (Test-Path -LiteralPath $extra)) 'Expired generations were not removed.'
    $unsafe = Generation ([DateTime]'2026-03-01T02:00:00Z')
    $outside = Join-Path $Root 'outside-generation'
    [void][IO.Directory]::CreateDirectory($outside)
    [IO.File]::WriteAllText((Join-Path $outside 'sentinel'), 'synthetic preserved data')
    [void](New-Item -ItemType Junction -Path (Join-Path $unsafe 'external') -Value $outside)
    Reject { Remove-ExpiredBackup $plan $Root }
    Assert (Test-Path -LiteralPath (Join-Path $outside 'sentinel')) 'Retention followed an external directory link.'
    Reject { Test-BackupFreshness $plan }
    Write-BackupJson (Join-Path $Root 'last-success.json') @{ status = 'complete'; deploymentId = $plan.deploymentId; cutUtc = [DateTime]::UtcNow.AddHours(-25).ToString('o') }
    Reject { Test-BackupFreshness $plan }
    Write-BackupJson (Join-Path $Root 'last-success.json') @{ status = 'complete'; deploymentId = $plan.deploymentId; cutUtc = [DateTime]::UtcNow.AddHours(-1).ToString('o') } -Replace
    Test-BackupFreshness $plan
    # A failed image must release all prepared barriers and cannot publish completion or prune.
    $script:phases = New-Object 'Collections.Generic.List[string]'
    $plan | Add-Member NoteProperty share $Root
    $plan | Add-Member NoteProperty storageCredentialFile 'storage'
    $plan | Add-Member NoteProperty postgresCredentialFile 'postgres'
    $plan.nodes = @([pscustomobject]@{ id = 'server'; kind = 'server'; volumes = @('C:'); requiredPaths = @('synthetic'); runtimes = @() },
        [pscustomobject]@{ id = 'client'; kind = 'client'; volumes = @('C:'); requiredPaths = @('synthetic'); runtimes = @('synthetic') })
    function Import-Clixml { param($LiteralPath) New-Object pscredential('synthetic', (ConvertTo-SecureString 'synthetic-pass' -AsPlainText -Force)) }
    function Test-BackupStorage { param($Plan, $Credential) [pscustomobject]@{ machineId = 'off-device' } }
    function Get-SmbMapping {}
    function New-SmbMapping { param($RemotePath, $UserName, $Password, $Persistent, $RequirePrivacy) [pscustomobject]@{ RemotePath = $RemotePath } }
    function Remove-SmbMapping { param($RemotePath, [switch]$Force, $UpdateProfile) $script:phases.Add('unmap') }
    function Open-BackupSession { param($Node) return $Node.id }
    function Get-CimInstance { param($ClassName) [pscustomobject]@{ UUID = 'server' } }
    function Remove-PSSession { param($Session) }
    function Invoke-Command {
        param($Session, $FilePath, $ArgumentList)
        $phase = $ArgumentList[0]; $script:phases.Add($Session + ':' + $phase)
        if ($phase -eq 'Prepare') { return [pscustomobject]@{ machineId = $Session; computer = $Session; boot = 'synthetic' } }
        if ($phase -eq 'Capture') {
            if ($script:failCapture) { throw 'synthetic storage failure' }
            if ($Session -eq 'server') {
                $cluster = Join-Path $ArgumentList[2] 'cluster'
                [void][IO.Directory]::CreateDirectory($cluster)
                [IO.File]::WriteAllText((Join-Path $cluster 'backup_manifest'), 'synthetic manifest')
            }
            return [pscustomobject]@{ imageCatalogVerified = $true }
        }
    }
    function Remove-ExpiredBackup { param($Plan, $Root) $script:phases.Add('retention') }
    function Write-EventLog { param($LogName, $Source, $EventId, $EntryType, $Message) }
    $script:failCapture = $true
    Reject { Invoke-BackupGeneration $plan }
    Assert (($script:phases -join ',') -ceq 'server:Prepare,client:Prepare,server:Freeze,client:Freeze,server:Capture,server:Release,client:Release,unmap') 'Failure left a barrier held or captured before all participants froze.'
    Assert (@(Get-ChildItem -LiteralPath (Join-Path $Root $plan.deploymentId) -Filter complete.json -Recurse).Count -eq 0) 'Failed backup was marked complete.'
    $script:phases.Clear(); $script:failCapture = $false
    Invoke-BackupGeneration $plan
    Assert (($script:phases -join ',') -ceq 'server:Prepare,client:Prepare,server:Freeze,client:Freeze,server:Capture,client:Capture,server:Check,client:Check,server:Release,client:Release,retention,unmap') 'Completion or retention occurred before validation and resume.'
    Test-BackupFreshness $plan
    Assert (@(Get-ChildItem -LiteralPath (Join-Path $Root $plan.deploymentId) -Filter complete.json -Recurse).Count -eq 1) 'Successful generation was not published once.'
    Write-Host 'PASS: retention calendars, holds, unknown and partial preservation, stale backup detection, successful cohort publication, and failure barrier cleanup.'
} $evidence
Write-Host "Synthetic evidence: $evidence"
