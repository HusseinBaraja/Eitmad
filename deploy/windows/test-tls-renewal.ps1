#Requires -Version 5.1
param([string]$CertificateExecutable = (Join-Path $PSScriptRoot '../../tools/lan-certificate/bin/Release/net8.0-windows/Eitmad.LanCertificate.exe'),
    [switch]$ProtectedConfiguration)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
if ($ProtectedConfiguration) {
    $principal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
    if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        throw 'Protected configuration verification requires an elevated Windows token. No test directories were created.'
    }
}
Import-Module (Join-Path $PSScriptRoot 'tls-renewal.psm1') -Force -DisableNameChecking
$module = Get-Module tls-renewal
$root = Join-Path $env:TEMP ('eitmad-tls-adapter-' + [Guid]::NewGuid().ToString('N'))
[void][IO.Directory]::CreateDirectory($root)
& $module {
    param($Root, $Tool, $ProtectedConfiguration)
    function Assert($Value, [string]$Message) { if (-not $Value) { throw $Message } }
    function Reject([scriptblock]$Operation) {
        $failed = $false; try { & $Operation } catch { $failed = $true }
        Assert $failed 'Expected refusal.'
    }
    $OutputEncoding = New-Object Text.UTF8Encoding($false, $true)
    $unlockPassword = 'isolated adapter CA password ' + [char]0x0633
    # No services, scheduled tasks, trust stores, or firewall rules are changed.
    Reject { Invoke-TlsMechanic Guard $Root @{} }
    foreach ($child in 'renewal/tool','app','private') { [void][IO.Directory]::CreateDirectory((Join-Path $Root $child)) }
    foreach ($file in Get-ChildItem -LiteralPath (Split-Path $Tool -Parent) -File) {
        Copy-Item -LiteralPath $file.FullName -Destination (Join-Path $Root 'renewal/tool')
    }
    $authority = Join-Path $Root 'renewal/authority'
    $unlockPassword | & $Tool initialize $authority workshop.test | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Isolated certificate initialization failed.' }
    [void](Invoke-TlsTool $Root retain @{password=$unlockPassword})
    $old = Get-Content -LiteralPath (Join-Path $authority 'active-tls.json') -Raw | ConvertFrom-Json
    $next = Invoke-TlsTool $Root stage @{}
    Copy-Item -LiteralPath (Join-Path $authority 'trusted-ca.pem') -Destination (Join-Path $Root 'private/trusted-ca.pem')
    Copy-Item -LiteralPath (Join-Path $authority $old.Certificate) -Destination (Join-Path $Root 'private/server-cert.pem')
    Copy-Item -LiteralPath (Join-Path $authority $old.PrivateKey) -Destination (Join-Path $Root 'private/server-key.pem')
    $bundle = Join-Path $Root ('private/' + (Split-Path $next.Certificate -Parent))
    [void][IO.Directory]::CreateDirectory($bundle)
    foreach ($path in $next.Certificate,$next.PrivateKey) {
        Copy-Item -LiteralPath (Join-Path $authority $path) -Destination (Join-Path $Root ('private/' + $path))
    }
    $oldPair = Read-TlsPair $Root 'private/server-cert.pem' 'private/server-key.pem'
    $newPair = Read-TlsPair $Root ('private/' + $next.Certificate) ('private/' + $next.PrivateKey)
    $settings = [pscustomobject]@{name='IsolatedTls';root=$Root;endpoint='https://workshop.test:8443/';serverPort=8443;databasePort=55432;address='127.0.0.2'}
    $xml = & $script:HostTools { param($s) New-WrapperXml $s @{
        EITMAD_SERVER_LISTEN='0.0.0.0:8443'
        EITMAD_SERVER_TLS_CERTIFICATE=(Join-Path $s.root 'private/server-cert.pem')
        EITMAD_SERVER_TLS_PRIVATE_KEY=(Join-Path $s.root 'private/server-key.pem')
    } } $settings
    $manifest = Join-Path $Root 'app/host-service.xml'
    $xml.Save($manifest)
    Assert ((Get-TlsProbeAddress $Root $settings) -ceq '127.0.0.1') 'Wildcard readiness did not select loopback.'
    $listener = $xml.service.env | Where-Object name -CEQ 'EITMAD_SERVER_LISTEN'
    $listener.SetAttribute('value', '127.0.0.1:8443'); $xml.Save($manifest)
    Assert ((Get-TlsProbeAddress $Root $settings) -ceq '127.0.0.1') 'Explicit local listener was not retained.'
    $listener.SetAttribute('value', '192.0.2.1:8443'); $xml.Save($manifest)
    Reject { Get-TlsProbeAddress $Root $settings }
    $listener.SetAttribute('value', '0.0.0.0:8444'); $xml.Save($manifest)
    Reject { Get-TlsProbeAddress $Root $settings }
    $listener.SetAttribute('value', '0.0.0.0:8443'); $xml.Save($manifest)
    Set-TlsPair $Root $newPair
    $xml = [xml](Get-Content -LiteralPath $manifest -Raw)
    Assert (($xml.service.env | Where-Object name -EQ 'EITMAD_SERVER_TLS_CERTIFICATE').value -eq (Join-Path $Root $newPair.certificate)) 'New certificate was not activated.'
    Assert (($xml.service.env | Where-Object name -EQ 'EITMAD_SERVER_TLS_PRIVATE_KEY').value -eq (Join-Path $Root $newPair.privateKey)) 'Matching key was not activated.'
    $before = [IO.File]::ReadAllText($manifest)
    $bad = @{} + $newPair; $bad.privateKey = $oldPair.privateKey
    Reject { Set-TlsPair $Root $bad }
    Assert ($before -ceq [IO.File]::ReadAllText($manifest)) 'Mismatched key changed active references.'
    Set-TlsPair $Root $oldPair
    Assert (Test-Path -LiteralPath (Join-Path $Root $newPair.privateKey)) 'Rollback removed the staged pair.'
    Reject { Get-TlsPairPath $Root 'private/../outside/server-key.pem' }

    if ($ProtectedConfiguration) {
        # Use real protected directories and DPAPI, but no service or task registrations.
        $installed = Join-Path $Root 'protected-host'
        & $script:HostTools { param($p) Protect-Directory $p; Protect-Directory (Join-Path $p 'private') } $installed
        Copy-Item -LiteralPath (Join-Path $authority 'trusted-ca.pem') -Destination (Join-Path $installed 'private/trusted-ca.pem')
        $protectedSettings = [pscustomobject]@{root=$installed;name='IsolatedTls';endpoint=$settings.endpoint;serverPort=8443}
        $configuration = [pscustomobject]@{lanDirectory=$authority;certificateExecutable=$Tool;password=$unlockPassword}
        Initialize-TlsControl $installed $protectedSettings $configuration
        Assert-TlsControl (Join-Path $installed 'renewal')
        Assert-TlsControl (Join-Path $installed 'app/deploy/windows')
        $retained = Invoke-TlsTool $installed stage @{}
        Assert ($retained.Version -eq 1) 'Protected retained credential did not unlock the same CA.'
        [IO.File]::WriteAllBytes((Join-Path $installed 'renewal/authority/unlock.dpapi'), @(1,2,3))
        Initialize-TlsControl $installed $protectedSettings $configuration
        [void](Invoke-TlsTool $installed stage @{})
        Assert ((Get-FileHash -LiteralPath (Join-Path $installed 'renewal/authority/ca-key.encrypted.pem')).Hash -ceq
            (Get-FileHash -LiteralPath (Join-Path $authority 'ca-key.encrypted.pem')).Hash) 'Credential repair replaced the CA.'
    }

    $script:task = $null; $script:registrations = 0
    function Get-ScheduledTask { param($TaskName,$ErrorAction) return $script:task }
    function New-ScheduledTaskAction { param($Execute,$Argument) [pscustomobject]@{Execute=$Execute;Arguments=$Argument} }
    function New-ScheduledTaskTrigger { param([switch]$Once,$At,$RepetitionInterval)
        [pscustomobject]@{Repetition=[pscustomobject]@{Interval=[Xml.XmlConvert]::ToString($RepetitionInterval)}} }
    function New-ScheduledTaskSettingsSet { param([switch]$StartWhenAvailable,$MultipleInstances,$ExecutionTimeLimit)
        [pscustomobject]@{StartWhenAvailable=[bool]$StartWhenAvailable;MultipleInstances=$MultipleInstances} }
    function Register-ScheduledTask { param($TaskName,$Action,$Trigger,$Settings,$User,$RunLevel,[switch]$Force)
        $script:registrations++
        $script:task = [pscustomobject]@{Actions=@($Action);Triggers=@($Trigger);Settings=$Settings
            Principal=[pscustomobject]@{UserId=$User};State='Ready'} }
    Register-TlsSchedule $Root $settings 3600
    Read-TlsSchedule $Root $settings 3600
    Assert ($script:task.Actions[0].Arguments -ceq ('windows-tls tick "' + $Root + '"')) 'Task contains unexpected input.'
    Assert ($script:task.Principal.UserId -ceq 'SYSTEM') 'Task requires an interactive credential.'
    $script:task.State = 'Disabled'
    Reject { Read-TlsSchedule $Root $settings 3600 }
    Register-TlsSchedule $Root $settings 3600
    $script:task.Actions[0].Execute = 'foreign.exe'
    Reject { Register-TlsSchedule $Root $settings 3600 }
    Assert ($script:registrations -eq 2) 'A foreign task was replaced.'

    & $script:HostTools {
        param($s)
        $script:tlsEvents = New-Object 'Collections.Generic.List[string]'
        function script:Get-OwnedService { param($Settings,$Kind)
            if ($Kind -ne 'Server') { throw 'Renewal touched PostgreSQL.' }
            $service = [pscustomobject]@{Status='Running'}
            $service | Add-Member ScriptMethod Stop { $script:tlsEvents.Add('stop') }
            $service | Add-Member ScriptMethod WaitForStatus { param($Status,$Timeout) $script:tlsEvents.Add('wait') }
            return $service
        }
        function script:Start-HostService { param($Settings,$Kind)
            if ($Kind -ne 'Server') { throw 'Renewal started PostgreSQL.' }
            $script:tlsEvents.Add('start')
        }
    } $settings
    Restart-TlsHost $settings
    $events = & $script:HostTools { return ($script:tlsEvents -join ',') }
    Assert ($events -ceq 'stop,wait,start') 'The host was not stopped before restart.'
} $root ([IO.Path]::GetFullPath($CertificateExecutable)) $ProtectedConfiguration
Write-Host 'TLS adapter checks passed: isolated signing, atomic pair references, rollback, task catch-up settings, and task ownership.'
