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
    $script:services = @{}
    foreach ($kind in 'Server', 'Postgres') {
        $item = [pscustomobject]@{Kind=$kind; Status='Stopped'}
        $item | Add-Member ScriptMethod Start { $script:events.Add('start:'+$this.Kind); $this.Status='Running' }
        $item | Add-Member ScriptMethod Stop { $script:events.Add('stop:'+$this.Kind); $this.Status='Stopped' }
        $item | Add-Member ScriptMethod WaitForStatus { param($Status, $Timeout) }
        $script:services[$kind] = $item
    }
    function Get-OwnedService { param($Settings, $Kind) return $script:services[$Kind] }
    function Get-NetTCPConnection { param($State, $LocalPort, $ErrorAction) }
    function Protect-Directory { param($Path, $Readers) [void][IO.Directory]::CreateDirectory($Path) }
    function Invoke-Native { param($File, $Arguments) $script:events.Add(($Arguments -join '|')) }
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
    Set-HostRelease $settings $one (Join-Path $Evidence '1.0.0') Repair '' $false
    Assert ([IO.File]::ReadAllText((Join-Path $root 'app/eitmad-server.exe')) -ceq 'release one') 'Repair did not restore original binary.'
    Assert (($script:events -join ',').Contains('rust:migrate,start:Server,ready')) 'Repair bypassed Rust migration or readiness.'
    Stop-Host $settings
    [IO.File]::Delete((Join-Path $root 'app/eitmad-server.exe'))
    Set-HostRelease $settings $one (Join-Path $Evidence '1.0.0') Repair '' $false
    Assert ([IO.File]::ReadAllText((Join-Path $root 'app/eitmad-server.exe')) -ceq 'release one') 'Repair did not restore a missing binary.'
    Stop-Host $settings
    $two = New-Release '1.1.0' 'release two'
    Reject { Set-HostRelease $settings $two (Join-Path $Evidence '1.1.0') Upgrade '' $false } 'requires BackupGeneration'
    Assert ([IO.File]::ReadAllText((Join-Path $root 'app/eitmad-server.exe')) -ceq 'release one') 'Missing backup changed installed bytes.'
    $generation = Join-Path $Evidence 'generation'
    [void][IO.Directory]::CreateDirectory($generation)
    [pscustomobject]@{nodes=@([pscustomobject]@{kind='server'; computer=$env:COMPUTERNAME; requiredPaths=@($root)})} |
        ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $generation 'complete.json')
    # Backup format/content verification has its own suite. This test covers the
    # release gate and failure recovery, without running wbadmin or machine services.
    'param($Generation,$PostgresBin)' | Set-Content -LiteralPath (Join-Path $Evidence '1.1.0/restore-backup.ps1')
    $script:failMigration = $true
    $script:events.Clear()
    Reject { Set-HostRelease $settings $two (Join-Path $Evidence '1.1.0') Upgrade $generation $true } 'Release failed'
    Assert (Test-Path -LiteralPath (Join-Path $root 'release-pending.json')) 'Failed upgrade lost recovery marker.'
    Assert ($script:services.Server.Status -eq 'Stopped' -and $script:services.Postgres.Status -eq 'Stopped') 'Failed migration left a writer running.'
    Assert (-not ($script:events -join ',').Contains('delayed-auto')) 'Failed upgrade enabled automatic startup.'
    Assert ([IO.File]::ReadAllText((Join-Path $root 'app/eitmad-server.exe')) -ceq 'release two') 'Failure silently rolled back a possibly migrated database.'
    $eventCount = $script:events.Count
    Reject { Invoke-EitmadHost Start -StateDirectory $root -Name EitmadReleaseProof } 'interrupted release is pending'
    Assert ($script:events.Count -eq $eventCount) 'A pending release started a service.'
    Reject { Set-HostRelease $settings $one (Join-Path $Evidence '1.0.0') Repair '' $false } 'exact interrupted release'
    [IO.File]::WriteAllText((Join-Path $root 'app/release.json'), 'interrupted inventory write')
    $script:failMigration = $false
    Set-HostRelease $settings $two (Join-Path $Evidence '1.1.0') Repair '' $false
    Assert (-not (Test-Path -LiteralPath (Join-Path $root 'release-pending.json'))) 'Successful recovery retained the pending gate.'
    Assert ((Get-Content -LiteralPath (Join-Path $root 'app/release.json') -Raw | ConvertFrom-Json).version -ceq '1.1.0') 'Recovery inventory is incorrect.'
    Assert ([IO.File]::ReadAllText((Join-Path $root 'data/retained')) -ceq 'committed and pending work') 'Release changed durable work.'
    Assert ([IO.File]::ReadAllText((Join-Path $root 'private/retained')) -ceq 'synthetic credential dependency') 'Release replaced credential dependencies.'
} $evidence
Write-Host 'PASS: real file replacement; mocked services; repair, backup gate, failed migration, exact-release recovery, and retained work/credentials.'
Write-Host "Synthetic evidence: $evidence"
