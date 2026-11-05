#Requires -Version 5.1
# Isolated filesystem and mocked Windows registrations. Never uninstall this PC.
[CmdletBinding()]
param()
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
Import-Module (Join-Path $PSScriptRoot 'uninstallation.psm1') -Force
$module = Get-Module uninstallation
$evidence = Join-Path ([IO.Path]::GetTempPath()) ('eitmad-uninstall-proof-' + [Guid]::NewGuid().ToString('N'))
[void][IO.Directory]::CreateDirectory($evidence)
& $module {
    param($evidence)
    function Assert($Condition, [string]$Message) { if (-not $Condition) { throw $Message } }
    function Reject([scriptblock]$Operation, [string]$Message) {
        $caught = $false
        try { & $Operation } catch { $caught = $true; if ($_.Exception.Message -notlike ('*' + $Message + '*')) { throw } }
        Assert $caught ('Expected rejection: ' + $Message)
    }
    function Write-Fixture([string]$Path, [string]$Text) {
        [void][IO.Directory]::CreateDirectory((Split-Path $Path -Parent))
        [IO.File]::WriteAllText($Path, $Text)
    }
    # An identically named package from another publisher is not ours to remove.
    function Get-AppxPackage { param($Name) return [pscustomobject]@{Publisher='CN=Foreign'; PackageFullName='foreign-package'} }
    function Remove-AppxPackage { param($Package, $ErrorAction) throw 'Foreign package removal must not run' }
    Reject { Remove-DesktopRegistration 'CN=Eitmad' } 'publisher differs'
    function Assert-DesktopStopped {}
    $script:removedDesktop = 0
    function Remove-DesktopRegistration { $script:removedDesktop++ }
    $userRoot = Join-Path $evidence 'user'
    Write-Fixture (Join-Path $userRoot 'Eitmad/engine/authority.sqlite3') 'synthetic user records'
    Write-Fixture (Join-Path $userRoot 'Eitmad/engine/desktop-device.json') 'synthetic credential references'
    Write-Fixture (Join-Path $userRoot 'Eitmad/Installers/old-release/setup.exe') 'disposable cached program'
    Write-Fixture (Join-Path $userRoot 'EitmadSetupProgress/installation.json') 'completed setup marker'
    $journal = 'EitmadSetupUpdate-client-' + ('a' * 64) + '-' + ('b' * 64)
    Write-Fixture (Join-Path $userRoot ($journal + '/installation.json')) 'update marker'
    Write-Fixture (Join-Path $userRoot 'EitmadSetupUpdate-personal-notes/sentinel') 'unrelated file'
    Remove-UserInstallation $userRoot
    Assert ($script:removedDesktop -eq 1) 'Desktop package was not removed before resetting active paths.'
    Assert (-not (Test-Path -LiteralPath (Join-Path $userRoot 'Eitmad'))) 'Active desktop state blocks a fresh installation.'
    Assert (-not (Test-Path -LiteralPath (Join-Path $userRoot 'EitmadSetupProgress'))) 'Original setup journal remains active.'
    Assert (-not (Test-Path -LiteralPath (Join-Path $userRoot $journal))) 'Release setup journal remains active.'
    $retained = @(Get-ChildItem -LiteralPath (Join-Path $userRoot 'EitmadRetained') -Directory)[0].FullName
    Assert ([IO.File]::ReadAllText((Join-Path $retained 'Eitmad/engine/authority.sqlite3')) -ceq 'synthetic user records') 'User records changed.'
    Assert (Test-Path -LiteralPath (Join-Path $retained 'Eitmad/engine/desktop-device.json')) 'Credential references were lost.'
    Assert (-not (Test-Path -LiteralPath (Join-Path $retained 'Eitmad/Installers'))) 'Executable cache was retained.'
    Assert (Test-Path -LiteralPath (Join-Path $userRoot 'EitmadSetupUpdate-personal-notes/sentinel')) 'Unrelated directory changed.'
    # A second removal is safe and does not replace the first retained data set.
    Remove-UserInstallation $userRoot
    Assert ([IO.File]::ReadAllText((Join-Path $retained 'Eitmad/engine/authority.sqlite3')) -ceq 'synthetic user records') 'Retry replaced retained data.'

    $linkedRoot = Join-Path $evidence 'linked-user'
    $outside = Join-Path $evidence 'outside'
    Write-Fixture (Join-Path $outside 'sentinel') 'external data'
    [void][IO.Directory]::CreateDirectory((Join-Path $linkedRoot 'Eitmad'))
    $junction = Join-Path $linkedRoot 'Eitmad/engine'
    New-Item -ItemType Junction -Path $junction -Target $outside | Out-Null
    try {
        $before = $script:removedDesktop
        Reject { Remove-UserInstallation $linkedRoot } 'junctions'
        Assert ($script:removedDesktop -eq $before) 'Linked-path rejection occurred after package removal.'
        Assert ([IO.File]::ReadAllText((Join-Path $outside 'sentinel')) -ceq 'external data') 'Removal followed a junction.'
    } finally { [IO.Directory]::Delete($junction) }
    Reject { Assert-RemovalTree $outside $userRoot } 'outside'

    # Test the real registration preflight with SCM substituted inside its owner.
    & $script:HostAdapter {
        function script:Get-OwnedService { param($Settings, $Kind) if ($Kind -eq 'Postgres') { throw 'Foreign PostgreSQL service' }; return $null }
        function script:Stop-Host { param($Settings) throw 'Stop must not run before complete ownership validation' }
    }
    Reject { Remove-HostRegistrations (Join-Path $evidence 'missing-host') } 'Foreign PostgreSQL service'

    # Test machine file lifecycle separately. No SCM, registry, tasks, firewall,
    # administrator operations, or real ProgramData paths are touched.
    function Assert-RemovalOwner { param($Path) }
    function New-RetentionDirectory {
        param($Parent, $Machine)
        $path = Join-Path (Join-Path $Parent 'EitmadRetained') ([Guid]::NewGuid().ToString('N'))
        [void][IO.Directory]::CreateDirectory($path); return $path
    }
    & $script:HostAdapter { function script:Assert-Administrator {} }
    function Stop-PreparedDatabase { param($Prepared, $BundleDirectory) }
    $script:registrationFailure = $true
    function Remove-HostRegistrations { param($Root) if ($script:registrationFailure) { throw 'Service removal failed' } }
    $machine = Join-Path $evidence 'machine'
    Write-Fixture (Join-Path $machine 'EitmadHost/data/PG_VERSION') '17'
    Write-Fixture (Join-Path $machine 'EitmadHost/data/records') 'synthetic workshop records'
    Write-Fixture (Join-Path $machine 'EitmadHost/private/database-url') 'synthetic retained database reference'
    Write-Fixture (Join-Path $machine 'EitmadHost/app/eitmad-server.exe') 'synthetic program'
    Write-Fixture (Join-Path $machine 'EitmadHost/app/release.json') 'migration inventory'
    Write-Fixture (Join-Path $machine 'EitmadHost/postgres/bin/postgres.exe') 'synthetic runtime'
    Write-Fixture (Join-Path $machine 'EitmadHost/renewal/tool/tool.exe') 'synthetic tool'
    Write-Fixture (Join-Path $machine 'EitmadHost/renewal/authority/trusted-ca.pem') 'synthetic retained authority'
    Write-Fixture (Join-Path $machine 'EitmadPreparedWorkshop/cluster/records') 'synthetic prepared records'
    Write-Fixture (Join-Path $machine 'EitmadServerSetupProgress/installation.json') 'interrupted marker'
    Reject { Remove-MachineInstallation $evidence ('a' * 64) $machine } 'Service removal failed'
    Assert (Test-Path -LiteralPath (Join-Path $machine 'EitmadHost/data/records')) 'Service failure moved live data.'
    $script:registrationFailure = $false
    Write-Fixture (Join-Path $machine 'EitmadHost/data/postmaster.pid') '123'
    function Get-CimInstance { param($ClassName, $Filter) return [pscustomobject]@{ProcessId=123} }
    Reject { Remove-MachineInstallation $evidence ('a' * 64) $machine } 'live process'
    Assert (Test-Path -LiteralPath (Join-Path $machine 'EitmadHost/data/records')) 'A live cluster was moved.'
    function Get-CimInstance { param($ClassName, $Filter) return $null }
    Remove-MachineInstallation $evidence ('a' * 64) $machine
    foreach ($name in 'EitmadHost', 'EitmadPreparedWorkshop', 'EitmadServerSetupProgress') {
        Assert (-not (Test-Path -LiteralPath (Join-Path $machine $name))) ('Active reinstall blocker remains: ' + $name)
    }
    $kept = @(Get-ChildItem -LiteralPath (Join-Path $machine 'EitmadRetained') -Directory)[0].FullName
    Assert ([IO.File]::ReadAllText((Join-Path $kept 'EitmadHost/data/records')) -ceq 'synthetic workshop records') 'Workshop records changed.'
    Assert (Test-Path -LiteralPath (Join-Path $kept 'EitmadPreparedWorkshop/cluster/records')) 'Prepared database was lost.'
    Assert (Test-Path -LiteralPath (Join-Path $kept 'EitmadHost/private/database-url')) 'Data recovery references were lost.'
    Assert (Test-Path -LiteralPath (Join-Path $kept 'EitmadHost/renewal/authority/trusted-ca.pem')) 'TLS recovery authority was lost.'
    Assert ([IO.File]::ReadAllText((Join-Path $kept 'EitmadHost/release.json')) -ceq 'migration inventory') 'Migration inventory was lost.'
    foreach ($name in 'app', 'postgres', 'renewal/tool') {
        Assert (-not (Test-Path -LiteralPath (Join-Path $kept ('EitmadHost/' + $name)))) ('Installed executable tree remains: ' + $name)
    }
    Reject { Invoke-EitmadUninstall -Role client -Phase Machine } 'Client removal cannot'
} $evidence
Write-Host ('Uninstall safety proof passed. Synthetic evidence: ' + $evidence)
