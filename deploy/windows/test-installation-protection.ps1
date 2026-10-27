#Requires -Version 5.1
# Elevated, isolated Windows ACL proof. Installs no service, task, app, or firewall rule.
[CmdletBinding()]
param([Parameter(Mandatory)][string]$SourceBundle, [Parameter(Mandatory)][string]$EvidenceDirectory)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$adapter = Import-Module (Join-Path $PSScriptRoot 'installation.psm1') -Force -PassThru
& $adapter { & $script:HostAdapter { Assert-Administrator } }
$evidence = [IO.Path]::GetFullPath($EvidenceDirectory)
if (Test-Path -LiteralPath $evidence) { throw 'Use a new evidence directory.' }
[void][IO.Directory]::CreateDirectory($evidence)
$prefix = Join-Path $env:ProgramData ('EitmadSetupProof-' + [Guid]::NewGuid().ToString('N').Substring(0,12))
$stage = $prefix + '-Payload'
$result = [ordered]@{complete=$false; payload=$stage; servicesChanged=$false; tasksChanged=$false}
try {
    $anchor = Join-Path $SourceBundle 'server/application/eitmad-server.exe'
    $powershell = "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe"
    @{root=$SourceBundle;anchor=$anchor;publisher=''} | ConvertTo-Json -Compress |
        & $powershell -NoProfile -NonInteractive -File (Join-Path $PSScriptRoot 'verify-offline.ps1')
    if ($LASTEXITCODE -ne 0) { throw 'Source bundle authentication failed.' }
    & $adapter { param($source,$target) New-ProtectedInstallationPayload $source $target } $SourceBundle $stage
    @{root=$stage;anchor=$anchor;publisher=''} | ConvertTo-Json -Compress |
        & $powershell -NoProfile -NonInteractive -File (Join-Path $PSScriptRoot 'verify-offline.ps1')
    if ($LASTEXITCODE -ne 0) { throw 'Protected bundle authentication failed.' }
    $postgres = Join-Path $stage 'server/postgresql'
    $user = [Security.Principal.WindowsIdentity]::GetCurrent().User
    # Reproduce the former Admin/SYSTEM-only payload without running a database.
    $acl = Get-Acl -LiteralPath $postgres
    foreach ($rule in @($acl.GetAccessRules($true,$false,[Security.Principal.SecurityIdentifier]))) {
        if ($rule.IdentityReference -eq $user) { [void]$acl.RemoveAccessRuleSpecific($rule) }
    }
    Set-Acl -LiteralPath $postgres -AclObject $acl
    function Probe-Initdb {
        $start = New-Object Diagnostics.ProcessStartInfo
        $start.FileName = Join-Path $postgres 'bin/initdb.exe'
        $start.Arguments = '--show -D "' + $prefix + '-Cluster"'
        $start.UseShellExecute=$false; $start.CreateNoWindow=$true
        $start.RedirectStandardOutput=$true; $start.RedirectStandardError=$true
        $child = [Diagnostics.Process]::Start($start)
        try {
            $stdout=$child.StandardOutput.ReadToEndAsync(); $stderr=$child.StandardError.ReadToEndAsync()
            if (-not $child.WaitForExit(30000)) { throw 'PostgreSQL tool probe timed out.' }
            [void]$stdout.GetAwaiter().GetResult(); [void]$stderr.GetAwaiter().GetResult()
            return $child.ExitCode
        } finally { $child.Dispose() }
    }
    $result.beforeReadGrant = Probe-Initdb
    if ($result.beforeReadGrant -ne -1073741515) { throw 'The original restricted-token DLL failure was not reproduced.' }
    & $adapter { param($root) Grant-PostgresToolReadAccess $root } $stage
    $result.afterReadGrant = Probe-Initdb
    if ($result.afterReadGrant -ne 0) { throw 'PostgreSQL still cannot read its protected tools.' }
    foreach ($path in $stage,(Join-Path $stage 'server/application/eitmad-server.exe')) {
        $rules = (Get-Acl -LiteralPath $path).GetAccessRules($true,$true,[Security.Principal.SecurityIdentifier])
        if (@($rules | Where-Object { $_.AccessControlType -eq 'Allow' -and $_.IdentityReference.Value -notin 'S-1-5-18','S-1-5-32-544' }).Count) {
            throw 'PostgreSQL read access exposed installation controls.'
        }
    }
    $result.complete=$true
} catch { $result.error=$_.Exception.Message }
$result | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $evidence 'protection-result.json') -Encoding UTF8
if (-not $result.complete) { throw $result.error }
Write-Host 'PASS: restricted PostgreSQL token and protected installation controls. Synthetic payload retained.'
