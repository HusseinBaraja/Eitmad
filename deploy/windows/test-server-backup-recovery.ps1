#Requires -Version 5.1
# Called only by the isolated Rust two-client recovery drill. Credentials stay in private environment/files.
param([Parameter(Mandatory)][string]$Root, [Parameter(Mandatory)][string]$PostgresBin, [Parameter(Mandatory)][int]$DatabasePort)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
Import-Module (Join-Path $PSScriptRoot 'local-backup.psm1') -Force -DisableNameChecking
$module = Get-Module local-backup
& $module {
    param($Root, $Bin, $Port)
    function Assert($Value, [string]$Message) { if (-not $Value) { throw $Message } }
    function Reject([scriptblock]$Operation) {
        $failed = $false; try { & $Operation } catch { $failed=$true }
        Assert $failed 'Expected recovery rejection.'
    }
    $script:FixtureRoot = [IO.Path]::GetFullPath($Root)
    # This developer process is not elevated, and checkout ancestors grant sandbox access.
    # Simulate privilege/ancestor observations only; restrict every fixture write to this new root.
    & $script:HostTools { function script:Assert-Administrator {} }
    function Assert-LocalBackupControl { param($Path)
        $full = Get-LocalBackupPath $Path
        if ($full -ine $script:FixtureRoot -and -not $full.StartsWith($script:FixtureRoot + '\', [StringComparison]::OrdinalIgnoreCase)) {
            throw 'Fixture escaped its isolated root.'
        }
        $trusted = @([Security.Principal.WindowsIdentity]::GetCurrent().User.Value, 'S-1-5-18')
        foreach ($rule in (Get-Acl -LiteralPath $full).GetAccessRules($true,$true,[Security.Principal.SecurityIdentifier])) {
            if ($rule.AccessControlType -eq 'Allow' -and $rule.IdentityReference.Value -notin $trusted) { throw 'Fixture permits other readers.' }
        }
    }
    function New-LocalBackupDirectory { param($Path)
        $full = Get-LocalBackupPath $Path
        if (-not $full.StartsWith($script:FixtureRoot + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'Unsafe fixture target.' }
        if (Test-Path -LiteralPath $full) { Assert-LocalBackupControl $full; return }
        Assert-LocalBackupControl (Split-Path $full -Parent)
        [void][IO.Directory]::CreateDirectory($full)
        $acl = New-Object Security.AccessControl.DirectorySecurity
        $sid = [Security.Principal.WindowsIdentity]::GetCurrent().User
        $acl.SetOwner($sid); $acl.SetAccessRuleProtection($true,$false)
        foreach ($identity in $sid,(New-Object Security.Principal.SecurityIdentifier('S-1-5-18'))) {
            $acl.AddAccessRule((New-Object Security.AccessControl.FileSystemAccessRule($identity,'FullControl','ContainerInherit,ObjectInherit','None','Allow')))
        }
        Set-Acl -LiteralPath $full -AclObject $acl
    }
    $hostRoot = Join-Path $Root 'synthetic-host'
    $recovery = Join-Path $Root 'certificates'
    $local = Join-Path $Root 'l'
    $external = Join-Path $Root 'e'
    foreach ($path in $hostRoot, $local, $external) { New-LocalBackupDirectory $path }
    foreach ($relative in 'private','app','postgres','manifests','data') { New-LocalBackupDirectory (Join-Path $hostRoot $relative) }
    # The database and certificate/key bytes are real synthetic application inputs.
    $database = [Uri]$env:EITMAD_DIRECT_TEST_DATABASE_URL
    [IO.File]::WriteAllText((Join-Path $hostRoot 'private/database-url'), $env:EITMAD_DIRECT_TEST_DATABASE_URL)
    $token = [Convert]::ToBase64String([byte[]](1..32 | ForEach-Object {9})).TrimEnd('=').Replace('+','-').Replace('/','_')
    [IO.File]::WriteAllText((Join-Path $hostRoot 'private/token-key'), $token)
    Copy-Item -LiteralPath $env:EITMAD_DIRECT_TEST_CERTIFICATE -Destination (Join-Path $hostRoot 'private/server-cert.pem')
    Copy-Item -LiteralPath $env:EITMAD_DIRECT_TEST_PRIVATE_KEY -Destination (Join-Path $hostRoot 'private/server-key.pem')
    Copy-Item -LiteralPath (Join-Path $recovery 'trusted-ca.pem') -Destination (Join-Path $hostRoot 'private/trusted-ca.pem')
    [IO.File]::WriteAllText((Join-Path $recovery 'postgres-admin-password'), $env:EITMAD_RECOVERY_ADMIN_PASSWORD)
    [IO.File]::WriteAllText((Join-Path $recovery 'application-password'), [Uri]::UnescapeDataString($database.UserInfo.Split(':')[1]))
    [IO.File]::WriteAllText((Join-Path $recovery 'server.json'), '{"version":1,"fixture":true}')
    Write-LocalBackupJson (Join-Path $hostRoot 'host.json') @{version=1;name='Synthetic';root=$hostRoot;databasePort=$Port}
    foreach ($kind in 'app','postgres') { [IO.File]::WriteAllText((Join-Path $hostRoot ($kind+'/host-service.xml')), '<service />') }
    # Use real PG executables through a fixture bin path; no installation or service is involved.
    $script:FixtureBin = $Bin
    $script:FixturePort = $Port
    $script:FixtureHost = $hostRoot
    $script:FixtureExternal = $external
    # Mock installed-service/disk observations too. Capture, copy, hashing, and staging are real.
    function Get-LocalBackupSources { param($Configuration) @{databasePort=$script:FixturePort} }
    function Get-BackupDisk { param($Path)
        [pscustomobject]@{disk=$(if ($Path.StartsWith($script:FixtureExternal)) {2} else {1});free=1099511627776} }
    function Get-Partition { param($DriveLetter) [pscustomobject]@{DiskNumber=1} }
    function Get-Disk { param($Number) [pscustomobject]@{BusType='SATA'} }
    function Backup-Postgres { param($Bin,$Port,$Credential,$Target)
        & (Get-Module backup-tools) { param($b,$p,$c,$t) Backup-Postgres $b $p $c $t } $script:FixtureBin $Port $Credential $Target }
    function Invoke-BackupTool { param($File,$Arguments)
        & (Get-Module backup-tools) { param($b,$a) Invoke-BackupTool (Join-Path $b 'pg_verifybackup.exe') $a } $script:FixtureBin $Arguments }
    $config = [pscustomobject]@{version=1;deploymentId='11111111-1111-4111-8111-111111111111';hostRoot=$hostRoot;
        serviceName='Synthetic';recoveryDirectory=$recovery;localDestination=$local;externalDestination=$external;reserveBytes=1073741824}
    $id = '00000000001792600000-' + [Guid]::NewGuid().ToString('N')
    Invoke-LocalServerCapture $config $id
    Copy-LocalServerBackup $config $id
    $externalRoot = Get-LocalBackupRoot $config $true
    $generation = Get-LocalGeneration $externalRoot $id
    # Restore from the external copy, not the source or local copy.
    $stage = Join-Path $Root 'staged-recovery'
    Restore-LocalServerBackup $generation $Bin $stage
    Reject { Restore-LocalServerBackup $generation $Bin $stage }
    $original = Get-Content -LiteralPath (Join-Path $hostRoot 'private/database-url') -Raw
    Assert ((Get-Content -LiteralPath (Join-Path $stage 'host/private/database-url') -Raw) -ceq $original) 'Application credential changed.'
    # CA renewal must work from staged key + staged password, with unchanged trust.
    $fingerprint = (Get-FileHash -LiteralPath (Join-Path $stage 'recovery/trusted-ca.pem')).Hash
    $password = Get-Content -LiteralPath (Join-Path $stage 'recovery/certificate-password') -Raw
    $password | & $env:EITMAD_RECOVERY_CERTIFICATE_TOOL renew (Join-Path $stage 'recovery') *> $null
    if ($LASTEXITCODE -ne 0) { throw 'Restored CA renewal failed.' }
    Assert ((Get-FileHash -LiteralPath (Join-Path $stage 'recovery/trusted-ca.pem')).Hash -ceq $fingerprint) 'CA trust changed.'
    # Tampering must reject recovery without changing the already-staged installation.
    [IO.File]::AppendAllText((Join-Path $generation 'host/private/token-key'), 'tampered')
    Reject { Restore-LocalServerBackup $generation $Bin (Join-Path $Root 'rejected-stage') }
    Assert (-not (Test-Path -LiteralPath (Join-Path $Root 'rejected-stage'))) 'Tampered backup created a stage.'
    Write-Host 'PASS: real capture, copied payload, isolated staging, credential continuity, CA renewal, existing-stage and tamper refusal.'
} $Root $PostgresBin $DatabasePort
