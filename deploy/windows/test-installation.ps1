#Requires -Version 5.1
[CmdletBinding()]
param()
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
Import-Module (Join-Path $PSScriptRoot 'installation.psm1') -Force
$adapter = Get-Module installation
$evidence = Join-Path ([IO.Path]::GetTempPath()) ('eitmad-install-adapter-' + [Guid]::NewGuid().ToString('N'))
[void][IO.Directory]::CreateDirectory($evidence)

& $adapter {
    param($Root)
    function Assert($Condition, [string]$Message) { if (-not $Condition) { throw $Message } }
    function Reject([scriptblock]$Operation) {
        $caught = $false
        try { & $Operation } catch { $caught = $true }
        Assert $caught 'Expected safe refusal.'
    }
    Assert (Test-InstallationProofRequest '{"version":1,"sequence":9,"stage":"enrollDesktop","status":"running"}') 'Fresh-proof input was not requested.'
    Assert (-not (Test-InstallationProofRequest '{"version":1,"sequence":8,"stage":"verifyHttps","status":"running"}')) 'Host progress requested private input.'
    Assert (-not (Test-InstallationProofRequest '{"version":1,"sequence":10,"stage":"enrollDesktop","status":"failed"}')) 'Failed enrollment requested a proof.'
    $script:identity = [pscustomobject]@{Sid = 'S-1-5-21-111-222-333-1001'; Elevated = $false}
    function Get-InstallationIdentity { return $script:identity }
    $script:installed = $false
    function Get-AppxPackage { param($Name) if ($script:installed) { return [pscustomobject]@{Name = $Name} } }
    $trace = Join-Path $Root 'trace.jsonl'
    $desktopScript = @'
param($Action, $ExpectedReleaseSha256, $Publisher)
@{action=$Action; digest=$ExpectedReleaseSha256; publisher=$Publisher} | ConvertTo-Json -Compress |
    Add-Content -LiteralPath (Join-Path $PSScriptRoot 'trace.jsonl')
Write-Host 'Synthetic native output must not enter the progress stream.'
'@
    [IO.File]::WriteAllText((Join-Path $Root 'install-client.ps1'), $desktopScript)
    $client = [pscustomobject]@{version = 1; intendedUserSid = $script:identity.Sid;
        desktop = [pscustomobject]@{directory = $Root; releaseSha256 = ('a' * 64); publisher = 'CN=Synthetic'}}
    $output = @(Invoke-EitmadInstallationAction -Action ClientPreflight -Backend 'synthetic' -Request $client)
    Assert ($output.Count -eq 0) 'Native verification output escaped.'
    $script:identity = [pscustomobject]@{Sid = 'S-1-5-21-999-888-777-500'; Elevated = $true}
    Reject { Invoke-EitmadInstallationAction -Action Desktop -Backend 'synthetic' -Request $client }
    $script:identity = [pscustomobject]@{Sid = $client.intendedUserSid; Elevated = $true}
    Reject { Invoke-EitmadInstallationAction -Action Desktop -Backend 'synthetic' -Request $client }
    $script:identity.Elevated = $false
    $script:installed = $true
    Reject { Invoke-EitmadInstallationAction -Action Desktop -Backend 'synthetic' -Request $client }
    $script:installed = $false
    Invoke-EitmadInstallationAction -Action Desktop -Backend 'synthetic' -Request $client
    $calls = @(Get-Content -LiteralPath $trace | ForEach-Object { $_ | ConvertFrom-Json })
    Assert (($calls.action -join ',') -ceq 'Verify,Verify,Install') 'Desktop path or refusal order changed.'
    Assert (@($calls | Where-Object { $_.digest -cne ('a' * 64) -or $_.publisher -cne 'CN=Synthetic' }).Count -eq 0) 'Package identity was lost.'

    # Host actions use the existing service adapter. All OS effects are synthetic.
    & $script:HostAdapter { function script:Assert-Administrator {} }
    $hostScript = @'
param($Action, $StateDirectory, $Name, $ExpectedReleaseSha256, $Publisher, $LanDirectory,
    $ClusterDirectory, $PostgresBin, $WinSwExecutable, $WinSwSha256, [switch]$PrivateLan)
@{action=$Action; privateLan=[bool]$PrivateLan; state=$StateDirectory; name=$Name;
    cluster=$ClusterDirectory; digest=$ExpectedReleaseSha256} | ConvertTo-Json -Compress |
    Add-Content -LiteralPath (Join-Path $PSScriptRoot 'host-trace.jsonl')
Write-Host 'Synthetic private native output.'
'@
    [IO.File]::WriteAllText((Join-Path $Root 'install-server.ps1'), $hostScript)
    $request = [pscustomobject]@{server = $client.desktop; serviceStateDirectory = (Join-Path $Root 'host'); serviceName = 'Synthetic';
        preparation = [pscustomobject]@{stateDirectory = (Join-Path $Root 'prepared'); postgresBin = (Join-Path $Root 'bin')};
        winswExecutable = (Join-Path $Root 'winsw.exe'); winswSha256 = ('b' * 64)}
    foreach ($action in 'Install', 'Firewall', 'Start') {
        $output = @(Invoke-EitmadInstallationAction -Action $action -Backend 'synthetic' -Request $request)
        Assert ($output.Count -eq 0) 'Host output escaped into typed progress.'
    }
    $calls = @(Get-Content -LiteralPath (Join-Path $Root 'host-trace.jsonl') | ForEach-Object { $_ | ConvertFrom-Json })
    Assert (($calls.action -join ',') -ceq 'Install,Firewall,Start') 'Host adapter actions changed.'
    Assert ($calls[1].privateLan -and -not $calls[0].privateLan -and -not $calls[2].privateLan) 'Private-network LocalSubnet policy was lost.'
    Assert ($calls[0].cluster -ceq (Join-Path $request.preparation.stateDirectory 'cluster')) 'An unrelated database was imported.'
    Assert ($calls[0].digest -ceq ('a' * 64)) 'Server package authentication was lost.'

    # Runtime installers are selected only on the elevated Server path. Substitute
    # registry/process boundaries; no real runtime or registry state is changed.
    $bundle = Join-Path $Root 'offline'
    [void][IO.Directory]::CreateDirectory((Join-Path $bundle 'server/application'))
    [IO.File]::WriteAllText((Join-Path $bundle 'offline.cat'), 'synthetic catalog marker')
    $backend = Join-Path $bundle 'server/application/eitmad-server.exe'
    $script:haveRuntimes = $true
    $script:installerExit = 0
    $script:installerCalls = @()
    function Get-ItemProperty {
        param($LiteralPath, $ErrorAction)
        if ($script:haveRuntimes) { return [pscustomobject]@{Installed=1; Version='v14.44.35211.0'; Release=528040} }
        return $null
    }
    function Start-Process {
        param($FilePath, $ArgumentList, [switch]$PassThru, $WindowStyle)
        $script:installerCalls += $FilePath
        $child = [pscustomobject]@{ExitCode=$script:installerExit}
        $child | Add-Member ScriptMethod WaitForExit { param($milliseconds) return $true }
        $child | Add-Member ScriptMethod Dispose {}
        return $child
    }
    Install-OfflineServerPrerequisites $backend
    Assert ($script:installerCalls.Count -eq 0) 'Existing runtimes were unnecessarily installed.'
    $script:haveRuntimes = $false
    Install-OfflineServerPrerequisites $backend
    Assert ($script:installerCalls.Count -eq 2) 'Both offline Server runtime installers are required.'
    Assert ($script:installerCalls[0] -ceq (Join-Path $bundle 'server/vc-redist/vc_redist.x64.exe')) 'C++ runtime came from outside the protected bundle.'
    $script:installerExit = 3010
    $script:installerCalls = @()
    Reject { Install-OfflineServerPrerequisites $backend }
    Assert ($script:installerCalls.Count -eq 1) 'Setup continued after a required restart.'
} $evidence

# Exercise real local pipes and peer PID checks, without UAC or installation.
Add-Type -Path (Join-Path $PSScriptRoot 'installation-channel.cs')
$backend = Join-Path $evidence 'synthetic-worker.exe'
Add-Type -OutputAssembly $backend -OutputType ConsoleApplication -TypeDefinition @'
using System;
public static class SyntheticInstallationWorker {
    public static int Main() {
        if (Console.ReadLine() != "synthetic-installation") return 1;
        Console.WriteLine("{\"version\":1,\"sequence\":1,\"stage\":\"verifyHttps\",\"status\":\"running\"}");
        Console.WriteLine("{\"version\":1,\"sequence\":2,\"stage\":\"enrollDesktop\",\"status\":\"running\"}");
        Console.Out.Flush();
        if (Console.ReadLine() != "synthetic-fresh-proof") return 2;
        Console.WriteLine("{\"version\":1,\"sequence\":3,\"stage\":\"complete\",\"status\":\"succeeded\"}");
        return 0;
    }
}
'@
$clientScript = @'
param($ChannelSource, $PipeName, [int]$BrokerId, $Mode, $Backend)
$ErrorActionPreference = 'Stop'
Add-Type -Path $ChannelSource
try {
    $pipe = [EitmadInstallationChannel]::Connect($PipeName, $BrokerId)
    try {
        if ($Mode -eq 'refresh') {
            Import-Module (Join-Path (Split-Path $ChannelSource) 'installation.psm1') -Force
            $code = & (Get-Module installation) {
                param($Pipe, $Backend)
                [EitmadInstallationChannel]::RunWorker($Pipe, $Backend,
                    [Func[string,bool]]{ param($line) return (Test-InstallationProofRequest $line) })
            } $pipe $Backend
            if ($code -ne 0) { throw 'Synthetic worker failed.' }
        }
        elseif ($Mode -eq 'oversize') { $bytes = [BitConverter]::GetBytes(65537); $pipe.Write($bytes,0,4); $pipe.Flush() }
        else { [EitmadInstallationChannel]::Send($pipe, [EitmadInstallationChannel]::Receive($pipe,65536)) }
    } finally { $pipe.Dispose() }
    if ($Mode -eq 'wrong-server') { exit 1 }
    exit 0
} catch { if ($Mode -eq 'wrong-server') { exit 0 }; exit 1 }
'@
$childFile = Join-Path $evidence 'pipe-client.ps1'
[IO.File]::WriteAllText($childFile, $clientScript)
foreach ($mode in 'echo', 'wrong-client', 'wrong-server', 'oversize', 'refresh') {
    $name = 'eitmad-install-test-' + [Guid]::NewGuid().ToString('N')
    $pipe = [EitmadInstallationChannel]::Create($name)
    $child = $null
    try {
        $broker = if ($mode -eq 'wrong-server') { $PID + 1 } else { $PID }
        $arguments = @('-NoProfile', '-NonInteractive', '-File', $childFile, '-ChannelSource', (Join-Path $PSScriptRoot 'installation-channel.cs'),
            '-PipeName', $name, '-BrokerId', [string]$broker, '-Mode', $mode, '-Backend', $backend) | ForEach-Object { '"' + $_ + '"' }
        $child = Start-Process -FilePath "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe" `
            -ArgumentList ($arguments -join ' ') -WindowStyle Hidden -PassThru
        if ($mode -eq 'wrong-client') {
            $rejected = $false
            try { [EitmadInstallationChannel]::Accept($pipe, ($child.Id + 1)) } catch { $rejected = $true }
            if (-not $rejected) { throw 'Unexpected elevation client was accepted.' }
            $pipe.Dispose()
        } else {
            [EitmadInstallationChannel]::Accept($pipe, $child.Id)
            if ($mode -eq 'echo') {
                $private = '{"synthetic":"' + [Guid]::NewGuid().ToString('N') + '","workshop":"ورشة اختبار"}'
                [EitmadInstallationChannel]::Send($pipe, $private)
                if ([EitmadInstallationChannel]::Receive($pipe, 65536) -cne $private) { throw 'Private UTF-8 pipe input changed.' }
                $private = $null
            } elseif ($mode -eq 'refresh') {
                [EitmadInstallationChannel]::Send($pipe, 'synthetic-installation')
                $ready = [EitmadInstallationChannel]::Receive($pipe, 32768) | ConvertFrom-Json
                if ($ready.stage -cne 'verifyHttps') { throw 'Host readiness was skipped.' }
                $request = [EitmadInstallationChannel]::Receive($pipe, 32768)
                $requiresInput = & $adapter { param($line) Test-InstallationProofRequest $line } $request
                if (-not $requiresInput) { throw 'Fresh-proof request was lost.' }
                [EitmadInstallationChannel]::Send($pipe, 'synthetic-fresh-proof')
                $complete = [EitmadInstallationChannel]::Receive($pipe, 32768) | ConvertFrom-Json
                if ($complete.stage -cne 'complete' -or $complete.status -cne 'succeeded') { throw 'Worker did not receive fresh proof.' }
                if ([EitmadInstallationChannel]::Receive($pipe, 16) -cne '' -or
                    [EitmadInstallationChannel]::Receive($pipe, 16) -cne 'success') { throw 'Worker completion was lost.' }
            } elseif ($mode -eq 'oversize') {
                $rejected = $false
                try { [void][EitmadInstallationChannel]::Receive($pipe, 65536) } catch { $rejected = $true }
                if (-not $rejected) { throw 'Oversized pipe input was accepted.' }
            }
        }
        if (-not $child.WaitForExit(30000)) { throw 'Synthetic pipe client did not stop.' }
        if ($mode -ne 'wrong-client' -and $child.ExitCode -ne 0) { throw 'Synthetic pipe client failed.' }
    } finally { $pipe.Dispose(); if ($child) { $child.Dispose() } }
}
Write-Host 'PASS: Client and Server adapter paths, wrong-user/elevation/existing-state refusals, private UTF-8 channel, peer PID checks, frame bound, and fresh-proof exchange. No installation, service, or firewall change.'
