#Requires -Version 5.1
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
Import-Module (Join-Path $PSScriptRoot 'package.psm1') -Force
Import-Module (Join-Path $PSScriptRoot 'host.psm1') -Force
$module = Get-Module host
$evidence = Join-Path ([IO.Path]::GetTempPath()) ('eitmad-release-proof-' + [Guid]::NewGuid().ToString('N'))
[void][IO.Directory]::CreateDirectory($evidence)
& $module {
    param($Evidence)
    function Assert($Condition, $Message) { if (-not $Condition) { throw $Message } }
    function Reject([scriptblock]$Operation, [string]$Message) {
        $failed = $false
        try { & $Operation } catch { $failed = $true; if ($_.Exception.Message -notlike "*$Message*") { throw } }
        Assert $failed "Expected failure: $Message"
    }
    $root = Join-Path $Evidence 'host'
    foreach ($folder in 'app', 'data', 'private', 'postgres') { [void][IO.Directory]::CreateDirectory((Join-Path $root $folder)) }
    $settings = [pscustomobject]@{version=1; name='EitmadReleaseProof'; root=$root; databasePort=55439; serverPort=58443}
    $settings | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $root 'host.json')
    [IO.File]::WriteAllText((Join-Path $root 'data/retained'), 'committed and pending work')
    [IO.File]::WriteAllText((Join-Path $root 'private/retained'), 'synthetic credential dependency')
    $script:events = New-Object 'Collections.Generic.List[string]'
    $script:startupModes = @{ EitmadReleaseProofPostgres = 'auto'; EitmadReleaseProofServer = 'delayed-auto' }
    $script:failServerAuto = $false
    $script:failMarkerRemoval = $false
    $script:failDemand = ''
    $script:failStop = ''
    $script:finalizationFailure = $null
    $script:finalizationFailed = $false
    $script:services = @{}
    foreach ($kind in 'Server', 'Postgres') {
        $item = [pscustomobject]@{Kind=$kind; Status='Stopped'}
        $item | Add-Member ScriptMethod Start { $script:events.Add('start:'+$this.Kind); $this.Status='Running' }
        $item | Add-Member ScriptMethod Stop {
            $script:events.Add('stop:'+$this.Kind)
            if ($script:failStop -eq $this.Kind) { throw ('synthetic stop failure: ' + $this.Kind) }
            $this.Status='Stopped'
        }
        $item | Add-Member ScriptMethod WaitForStatus { param($Status, $Timeout) }
        $script:services[$kind] = $item
    }
    function Get-OwnedService { param($Settings, $Kind) return $script:services[$Kind] }
    function Get-NetTCPConnection { param($State, $LocalPort, $ErrorAction) }
    function Protect-Directory { param($Path, $Readers) [void][IO.Directory]::CreateDirectory($Path) }
    function Invoke-Native {
        param($File, $Arguments)
        $script:events.Add(($Arguments -join '|'))
        if ($Arguments[0] -eq 'config') {
            if ($Arguments[3] -eq 'delayed-auto' -and $script:failServerAuto) {
                $script:finalizationFailed = $true
                throw $script:finalizationFailure
            }
            if ($script:finalizationFailed -and $Arguments[3] -eq 'demand' -and $Arguments[1] -eq ('EitmadReleaseProof' + $script:failDemand)) {
                throw ('synthetic demand-start failure: ' + $script:failDemand)
            }
            $script:startupModes[$Arguments[1]] = $Arguments[3]
        }
    }
    function Remove-Item {
        [CmdletBinding()]
        param($LiteralPath)
        if ($LiteralPath -eq (Join-Path $root 'release-pending.json')) {
            $script:events.Add('remove:pending')
            if ($script:failMarkerRemoval) {
                $script:finalizationFailed = $true
                throw $script:finalizationFailure
            }
        }
        Microsoft.PowerShell.Management\Remove-Item -LiteralPath $LiteralPath -ErrorAction Stop
    }
    function Test-RustConfiguration {
        param($Root, $Executable, $Command='check-config')
        $script:events.Add('rust:'+$Command)
        if ($Command -eq 'migrate' -and $script:failMigration) { throw 'synthetic migration failure' }
    }
    function Wait-Ready { param($Settings) $script:events.Add('ready') }
    function Assert-Administrator {}
    function Assert-ProtectedParents { param($Root) }
    function Assert-InstalledProtection { param($Root, $Name) }
    $script:failMigration = $false
    function New-Release($Version, $Bytes) {
        $directory = Join-Path $Evidence $Version
        [void][IO.Directory]::CreateDirectory($directory)
        [IO.File]::WriteAllText((Join-Path $directory 'eitmad-server.exe'), $Bytes)
        $release = [pscustomobject]@{version=$Version; windowsVersion=($Version+'.0');
            migrations=[pscustomobject]@{one='immutable'};
            files=[pscustomobject]@{'eitmad-server.exe'=(Get-FileHash (Join-Path $directory 'eitmad-server.exe') -Algorithm SHA256).Hash}}
        $release | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $directory 'release.json') -Encoding UTF8
        return $release
    }
    $one = New-Release '1.0.0' 'release one'
    Copy-Item -LiteralPath (Join-Path $Evidence '1.0.0/release.json') -Destination (Join-Path $root 'app/release.json')
    [IO.File]::WriteAllText((Join-Path $root 'app/eitmad-server.exe'), 'damaged binary')
    Set-HostRelease $settings $one (Join-Path $Evidence '1.0.0') Repair
    Assert ([IO.File]::ReadAllText((Join-Path $root 'app/eitmad-server.exe')) -ceq 'release one') 'Repair did not restore original binary.'
    Assert (($script:events -join ',').Contains('rust:migrate,start:Server,ready')) 'Repair bypassed Rust migration or readiness.'
    Stop-Host $settings
    [IO.File]::Delete((Join-Path $root 'app/eitmad-server.exe'))
    Set-HostRelease $settings $one (Join-Path $Evidence '1.0.0') Repair
    Assert ([IO.File]::ReadAllText((Join-Path $root 'app/eitmad-server.exe')) -ceq 'release one') 'Repair did not restore a missing binary.'
    Stop-Host $settings
    $two = New-Release '1.1.0' 'release two'
    $script:failMigration = $true
    $script:events.Clear()
    Reject { Set-HostRelease $settings $two (Join-Path $Evidence '1.1.0') Upgrade } 'Release failed'
    Assert (Test-Path -LiteralPath (Join-Path $root 'release-pending.json')) 'Failed upgrade lost recovery marker.'
    Assert ($script:services.Server.Status -eq 'Stopped' -and $script:services.Postgres.Status -eq 'Stopped') 'Failed migration left a writer running.'
    Assert ($script:startupModes.EitmadReleaseProofServer -eq 'demand' -and $script:startupModes.EitmadReleaseProofPostgres -eq 'demand') 'Failed migration left automatic startup enabled.'
    Assert (-not ($script:events -join ',').Contains('delayed-auto')) 'Failed upgrade enabled automatic startup.'
    Assert ([IO.File]::ReadAllText((Join-Path $root 'app/eitmad-server.exe')) -ceq 'release two') 'Failure silently rolled back a possibly migrated database.'
    $eventCount = $script:events.Count
    Reject { Invoke-EitmadHost Start -StateDirectory $root -Name EitmadReleaseProof } 'interrupted release is pending'
    Assert ($script:events.Count -eq $eventCount) 'A pending release started a service.'
    Reject { Set-HostRelease $settings $one (Join-Path $Evidence '1.0.0') Repair } 'exact interrupted release'
    [IO.File]::WriteAllText((Join-Path $root 'app/release.json'), 'interrupted inventory write')
    $script:failMigration = $false
    Invoke-EitmadHost Repair -StateDirectory $root -Name EitmadReleaseProof `
        -ReleaseDirectory (Join-Path $Evidence '1.1.0') -Release $two
    Assert (-not (Test-Path -LiteralPath (Join-Path $root 'release-pending.json'))) 'Successful recovery retained the pending gate.'
    Assert ((Get-Content -LiteralPath (Join-Path $root 'app/release.json') -Raw | ConvertFrom-Json).version -ceq '1.1.0') 'Recovery inventory is incorrect.'
    $pendingFile = Join-Path $root 'release-pending.json'
    foreach ($scenario in @(
        @{name='Pending-marker removal failure'; serverAuto=$false; demand=''; stop=''},
        @{name='Server automatic-start configuration failure'; serverAuto=$true; demand=''; stop=''},
        @{name='Server cleanup failures'; serverAuto=$false; demand='Server'; stop='Server'},
        @{name='Postgres cleanup failures'; serverAuto=$false; demand='Postgres'; stop='Postgres'}
    )) {
        Stop-Host $settings
        $script:events.Clear()
        $script:failServerAuto = $scenario.serverAuto
        $script:failMarkerRemoval = -not $scenario.serverAuto
        $script:failDemand = $scenario.demand
        $script:failStop = $scenario.stop
        $script:finalizationFailure = [IO.IOException]::new('synthetic finalization failure: ' + $scenario.name)
        $script:finalizationFailed = $false
        $failure = $null
        try { Set-HostRelease $settings $two (Join-Path $Evidence '1.1.0') Repair } catch { $failure = $_ }
        Assert ($null -ne $failure) ('Finalization did not fail: ' + $scenario.name)
        Assert ([object]::ReferenceEquals($failure.Exception.InnerException, $script:finalizationFailure)) 'Cleanup lost the original finalization exception.'
        Assert ($failure.Exception.Message.Contains($script:finalizationFailure.Message)) 'Release error omitted the original failure.'
        Assert (Test-Path -LiteralPath $pendingFile) 'Finalization failure lost the pending marker.'
        $pendingBytes = [IO.File]::ReadAllText($pendingFile)
        $marker = $pendingBytes | ConvertFrom-Json
        Assert ($marker.version -ceq $two.version -and $marker.binarySha256 -ceq $two.files.'eitmad-server.exe' -and
            $marker.previousRelease.version -ceq $two.version) 'Finalization failure changed pending recovery details.'
        Assert ((Get-Content -LiteralPath (Join-Path $root 'app/release.json') -Raw | ConvertFrom-Json).version -ceq $two.version) 'Finalization failure lost the target inventory.'
        $events = $script:events -join ','
        Assert ($events.Contains('config|EitmadReleaseProofPostgres|start=|auto,config|EitmadReleaseProofServer|start=|delayed-auto')) 'Failure did not reach automatic-start finalization.'
        Assert ($script:events.Contains('remove:pending') -eq $script:failMarkerRemoval) 'Marker removal occurred at the wrong finalization stage.'
        Assert ($events.EndsWith('config|EitmadReleaseProofServer|start=|demand,config|EitmadReleaseProofPostgres|start=|demand,stop:Server,stop:Postgres')) 'A cleanup failure skipped or reordered the remaining cleanup attempts.'
        foreach ($kind in 'Server', 'Postgres') {
            $expectedMode = if ($scenario.demand -eq $kind) { if ($kind -eq 'Server') { 'delayed-auto' } else { 'auto' } } else { 'demand' }
            $expectedState = if ($scenario.stop -eq $kind) { 'Running' } else { 'Stopped' }
            Assert ($script:startupModes['EitmadReleaseProof'+$kind] -eq $expectedMode) ("Incorrect $kind startup mode after finalization failure.")
            Assert ($script:services[$kind].Status -eq $expectedState) ("Incorrect $kind state after finalization failure.")
        }
        if ($scenario.demand) {
            Assert ($failure.Exception.Message.Contains("Set $($scenario.demand) to demand-start: synthetic demand-start failure: $($scenario.demand)")) 'Demand-start cleanup failure was not reported.'
            Assert ($failure.Exception.Message.Contains("Stop $($scenario.stop):") -and
                $failure.Exception.Message.Contains("synthetic stop failure: $($scenario.stop)")) 'Stop cleanup failure was not reported.'
            Assert ($failure.Exception.Message.Contains('state is not confirmed') -and
                -not $failure.Exception.Message.Contains('Services are stopped')) 'Failed cleanup claimed a safe service state.'
        } else {
            Assert ($failure.Exception.Message.Contains('Services are stopped with demand-start.')) 'Successful cleanup did not report its result.'
        }
        $eventCount = $script:events.Count
        Reject { Set-HostRelease $settings $one (Join-Path $Evidence '1.0.0') Repair } 'exact interrupted release'
        Assert ($script:events.Count -eq $eventCount -and [IO.File]::ReadAllText($pendingFile) -ceq $pendingBytes) 'Wrong-release repair changed pending recovery state.'
        $script:failServerAuto = $false; $script:failMarkerRemoval = $false; $script:failDemand = ''; $script:failStop = ''
        Stop-Host $settings
        Set-HostRelease $settings $two (Join-Path $Evidence '1.1.0') Repair
        Assert (-not (Test-Path -LiteralPath $pendingFile)) 'Successful finalization retained the pending marker.'
        Assert ($script:startupModes.EitmadReleaseProofPostgres -eq 'auto' -and
            $script:startupModes.EitmadReleaseProofServer -eq 'delayed-auto') 'Successful finalization changed intended automatic startup modes.'
        Assert ($script:services.Postgres.Status -eq 'Running' -and $script:services.Server.Status -eq 'Running') 'Successful finalization stopped a service.'
    }
    Assert ((Get-Content -LiteralPath (Join-Path $root 'releases/1.0.0.0/release.json') -Raw | ConvertFrom-Json).version -ceq '1.0.0') 'Release changed the prior inventory.'
    Assert ([IO.File]::ReadAllText((Join-Path $root 'releases/1.0.0.0/eitmad-server.exe')) -ceq 'release one') 'Release changed the prior binary.'
    Assert ([IO.File]::ReadAllText((Join-Path $root 'app/eitmad-server.exe')) -ceq 'release two') 'Finalization changed the target binary.'
    Assert ([IO.File]::ReadAllText((Join-Path $root 'data/retained')) -ceq 'committed and pending work') 'Release changed durable work.'
    Assert ([IO.File]::ReadAllText((Join-Path $root 'private/retained')) -ceq 'synthetic credential dependency') 'Release replaced credential dependencies.'
} $evidence
Write-Host 'PASS: real file replacement; mocked services; repair, failed migration, exact-release recovery, and retained work/credentials.'
Write-Host 'PASS: finalization failures restore demand-start and stop both services; cleanup failures retain the original error, report unsafe state, and allow remaining cleanup; success retains automatic startup.'
Write-Host "Synthetic evidence: $evidence"
