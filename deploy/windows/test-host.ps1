#Requires -Version 5.1
[CmdletBinding()]
param()
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
Import-Module (Join-Path $PSScriptRoot 'host.psm1') -Force
$module = Get-Module host
$evidence = Join-Path ([IO.Path]::GetTempPath()) ('eitmad-service-check-' + [Guid]::NewGuid().ToString('N'))
[void][IO.Directory]::CreateDirectory($evidence)

& $module {
    param($Evidence)
    function Assert($Condition, [string]$Message) { if (-not $Condition) { throw $Message } }
    function Reject([scriptblock]$Operation, [string]$Expected) {
        $caught = $false
        try { & $Operation } catch {
            $caught = $true
            if ($_.Exception.Message -notlike "*$Expected*") { throw }
        }
        Assert $caught "Expected rejection: $Expected"
    }
    # Exercise real Windows argv handling, including the quotes SCM needs around executable paths.
    $argv = Join-Path $Evidence 'argv.json'
    $python = (Get-Command python.exe).Source
    Invoke-Native $python @('-c', 'import json,sys; open(sys.argv[1],"w").write(json.dumps(sys.argv[2:]))',
        $argv, '"C:\Program Files\Eitmad\host.exe"', 'C:\trailing\', 'a"b')
    $actual = Get-Content -LiteralPath $argv -Raw | ConvertFrom-Json
    Assert ($actual[0] -ceq '"C:\Program Files\Eitmad\host.exe"' -and $actual[1] -ceq 'C:\trailing\' -and $actual[2] -ceq 'a"b') 'Windows argv quoting changed.'

    $root = Join-Path $Evidence 'host'
    [void][IO.Directory]::CreateDirectory($root)
    $settings = [pscustomobject]@{version = 1; name = 'EitmadTest'; root = $root
        endpoint = 'https://localhost:58443/'; serverPort = 58443; databasePort = 55439; address = '127.0.0.1'}
    $settings | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $root 'host.json')
    [IO.File]::WriteAllText((Join-Path $root 'retained-data'), 'synthetic durable sentinel')
    $xml = New-WrapperXml $settings @{'EITMAD_SERVER_DATABASE_URL_FILE' = (Join-Path $root 'private/database-url')}
    Assert ($xml.service.depend -ceq 'EitmadTestPostgres') 'Database dependency missing.'
    Assert ($xml.service.onfailure[-1].action -ceq 'none') 'Recovery would restart forever.'
    Assert ($xml.service.executable -eq (Join-Path $root 'app/eitmad-server.exe')) 'Wrapper launched the wrong application.'
    Assert ($xml.service.env.value -eq (Join-Path $root 'private/database-url')) 'Secret file reference changed.'
    [void][IO.Directory]::CreateDirectory((Join-Path $root 'postgres'))
    $postgresXml = New-WrapperXml $settings @{} 'Postgres'
    $postgresXml.Save((Join-Path $root 'postgres/host-service.xml'))
    Assert ($postgresXml.service.executable -eq (Join-Path $root 'postgres/bin/postgres.exe')) 'Postmaster is not directly supervised.'

    $script:events = New-Object 'Collections.Generic.List[string]'
    $script:services = @{}
    foreach ($kind in 'Postgres', 'Server') {
        $service = [pscustomobject]@{Kind = $kind; Status = 'Stopped'}
        $service | Add-Member ScriptMethod Start { $script:events.Add('start:' + $this.Kind); $this.Status = 'Running' }
        $service | Add-Member ScriptMethod Stop { $script:events.Add('stop:' + $this.Kind); $this.Status = 'Stopped' }
        $service | Add-Member ScriptMethod WaitForStatus { param($Status, $Timeout) }
        $script:services['EitmadTest' + $kind] = $service
    }
    function Assert-Administrator {}
    function Assert-ProtectedParents { param($Root) }
    function Assert-InstalledProtection { param($Root, $Name) }
    function Get-Service { param($Name, $ErrorAction) return $script:services[$Name] }
    function Get-ItemProperty {
        param($LiteralPath)
        $id = Split-Path $LiteralPath -Leaf
        if ($script:foreign) { return [pscustomobject]@{ImagePath = '"C:\other.exe"'; ObjectName = 'LocalSystem'} }
        $image = if ($id.EndsWith('Server')) { '"' + (Join-Path $script:root 'app/host-service.exe') + '"' }
            else { '"' + (Join-Path $script:root 'postgres/host-service.exe') + '"' }
        return [pscustomobject]@{ImagePath = $image; ObjectName = "NT SERVICE\$id"}
    }
    function Get-NetTCPConnection { param($State, $LocalPort, $ErrorAction) if ($script:conflict) { [pscustomobject]@{LocalPort = $LocalPort} } }
    $script:firewall = $null
    $script:firewallChanges = New-Object 'Collections.Generic.List[string]'
    function Get-NetFirewallRule { param($PolicyStore, $Name, $ErrorAction) return $script:firewall }
    function Get-NetFirewallApplicationFilter {
        param([Parameter(ValueFromPipeline)]$InputObject)
        process { [pscustomobject]@{Program = $InputObject.Program} }
    }
    function New-NetFirewallRule {
        param($PolicyStore, $Name, $Enabled, $Direction, $Action, $Profile, $Protocol, $LocalAddress,
            $LocalPort, $RemoteAddress, $Program, $EdgeTraversalPolicy, $DisplayName, $Group)
        $script:firewallChanges.Add('create')
        $script:firewall = [pscustomobject]$PSBoundParameters
    }
    function Set-NetFirewallRule {
        param($PolicyStore, $Name, $Enabled, $Direction, $Action, $Profile, $Protocol, $LocalAddress,
            $LocalPort, $RemoteAddress, $Program, $EdgeTraversalPolicy)
        $script:firewallChanges.Add('update')
        foreach ($key in $PSBoundParameters.Keys) { $script:firewall.$key = $PSBoundParameters[$key] }
    }
    function Remove-NetFirewallRule {
        param([Parameter(ValueFromPipeline)]$InputObject)
        process { $script:firewallChanges.Add('remove'); $script:firewall = $null }
    }
    function Test-Endpoint { param($Endpoint, $Route) $script:events.Add('probe:' + $Route); return $script:ready }
    function Invoke-Native {
        param($File, $Arguments)
        $script:events.Add(($Arguments -join '|'))
        if ($Arguments[0] -eq 'delete') { $script:services.Remove($Arguments[1]) }
    }
    $script:root = $root; $script:foreign = $false; $script:conflict = $false; $script:ready = $true
    Invoke-EitmadHost Start -StateDirectory $root -Name EitmadTest
    Assert (($script:events -join ',') -ceq 'start:Postgres,start:Server,probe:readyz') 'Startup order or readiness gate changed.'
    $status = Invoke-EitmadHost Status -StateDirectory $root -Name EitmadTest
    Assert ($status.Ready -and $status.Live) 'Ready status was not reported.'
    $script:ready = $false
    $status = Invoke-EitmadHost Status -StateDirectory $root -Name EitmadTest
    Assert (-not $status.Ready) 'A running process incorrectly implied readiness.'
    $script:events.Clear()
    Invoke-EitmadHost Stop -StateDirectory $root -Name EitmadTest
    Assert (($script:events -join ',') -ceq 'stop:Server,stop:Postgres') 'Shutdown order changed.'
    $script:events.Clear(); $script:conflict = $true
    Reject { Invoke-EitmadHost Start -StateDirectory $root -Name EitmadTest } 'already in use'
    Assert ($script:events.Count -eq 0) 'Port conflict started or stopped another process.'
    $script:conflict = $false; $script:foreign = $true
    Reject { Invoke-EitmadHost Uninstall -StateDirectory $root -Name EitmadTest } 'another executable or identity'
    Assert ($script:services.Count -eq 2) 'Foreign service was removed.'
    $script:foreign = $false
    Invoke-EitmadHost Uninstall -StateDirectory $root -Name EitmadTest
    Assert ($script:services.Count -eq 0) 'Uninstall left service registrations.'
    Assert ([IO.File]::ReadAllText((Join-Path $root 'retained-data')) -ceq 'synthetic durable sentinel') 'Uninstall changed durable data.'
    Assert (Test-Path -LiteralPath (Join-Path $root 'host.json')) 'Uninstall removed the reinstall manifest.'
    function Test-RustConfiguration { param($Root) $script:events.Add('rust:check-config') }
    function Protect-Directory { param($Path, $Readers) $script:events.Add('acl:' + $Path) }
    function Ensure-EventSource { param($Name) }
    $script:events.Clear()
    Install-Host $settings
    $installed = $script:events -join "`n"
    Assert ($installed.Contains('obj=|NT SERVICE\EitmadTestServer')) 'Server account is not isolated.'
    Assert ($installed.Contains('obj=|NT SERVICE\EitmadTestPostgres')) 'PostgreSQL account is not isolated.'
    Assert ($installed.Contains('actions=|restart/10000/restart/30000//0')) 'SCM recovery is not bounded.'
    Assert ($script:events[0] -ceq 'rust:check-config' -and $script:events[-1] -ceq 'config|EitmadTestServer|start=|delayed-auto') 'Automatic startup was enabled before configuration and protection.'
    Reject { Initialize-Host $root 'EitmadTest' 'source' 'source' 'source' 'source' ('A' * 64) } 'new state directory'
    Reject { Get-SafePath '\\server\share' } 'local directory'
    Reject { Get-SafePath 'C:\bad%path' } 'local directory'
    $settings.address = '192.168.50.10'
    $settings | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $root 'host.json')
    foreach ($invalid in 'Any', '192.168.50.0/24', '192.168.50.1-192.168.50.20', '8.8.8.8', '::1', '127.0.0.1', '10.1', '010.1.2.3') {
        Reject { Set-ClientFirewall $settings $invalid } 'exact private IPv4'
    }
    Assert ($script:firewallChanges.Count -eq 0) 'Invalid client input changed firewall policy.'
    Invoke-EitmadHost Firewall -StateDirectory $root -Name EitmadTest -ClientAddress '192.168.50.21' -WhatIf
    Assert ($script:firewallChanges.Count -eq 0) 'Firewall preview changed policy.'
    Invoke-EitmadHost Firewall -StateDirectory $root -Name EitmadTest -ClientAddress '192.168.50.21'
    Assert ($script:firewall.RemoteAddress -ceq '192.168.50.21' -and $script:firewall.Profile -ceq 'Private' -and
        $script:firewall.Program -ceq (Join-Path $root 'app/eitmad-server.exe') -and
        $script:firewall.LocalPort -eq 58443 -and $script:firewall.LocalAddress -ceq '192.168.50.10' -and
        $script:firewall.Protocol -ceq 'TCP' -and $script:firewall.Direction -ceq 'Inbound' -and
        $script:firewall.EdgeTraversalPolicy -ceq 'Block') 'Client rule escaped the installed HTTPS scope.'
    Invoke-EitmadHost Firewall -StateDirectory $root -Name EitmadTest -ClientAddress '192.168.50.22'
    Assert (($script:firewallChanges -join ',') -ceq 'create,update' -and
        $script:firewall.RemoteAddress -ceq '192.168.50.22') 'Changing the client retained access for the previous address.'
    $script:answers = New-Object 'Collections.Generic.Queue[string]'
    function Read-Host { param($Prompt) return $script:answers.Dequeue() }
    $script:answers.Enqueue('')
    Set-ClientFirewall $settings
    Assert ($script:firewallChanges.Count -eq 2) 'Cancelling the prompt changed the rule.'
    $script:answers.Enqueue('192.168.50.0/24'); $script:answers.Enqueue(' 192.168.50.23 ')
    Set-ClientFirewall $settings
    Assert ($script:firewall.RemoteAddress -ceq '192.168.50.23') 'The prompt did not recover from invalid input.'
    $script:firewall.Group = 'Another installation'
    Reject { Set-ClientFirewall $settings '192.168.50.24' } 'another installation'
    Assert ($script:firewallChanges.Count -eq 3) 'A foreign firewall rule was changed.'
    $script:firewall.Group = 'Eitmad host: ' + $root
    $script:firewall.Program = 'C:\other.exe'
    Reject { Set-ClientFirewall $settings '192.168.50.24' } 'another installation'
    $script:firewall.Program = Join-Path $root 'app/eitmad-server.exe'
    Invoke-EitmadHost Firewall -StateDirectory $root -Name EitmadTest -PrivateLan -WhatIf
    Assert ($script:firewallChanges.Count -eq 3) 'Private LAN preview changed policy.'
    Reject { Invoke-EitmadHost Firewall -StateDirectory $root -Name EitmadTest -PrivateLan -ClientAddress '192.168.50.24' } 'not both'
    Reject { Invoke-EitmadHost Start -StateDirectory $root -Name EitmadTest -PrivateLan } 'only for the Firewall'
    Assert ($script:firewallChanges.Count -eq 3) 'Invalid Private LAN options changed policy.'
    $settings.address = '0.0.0.0'
    Reject { Set-ClientFirewall $settings -PrivateLan } 'private IPv4 listener'
    $settings.address = '192.168.50.10'
    Invoke-EitmadHost Firewall -StateDirectory $root -Name EitmadTest -PrivateLan
    Assert ($script:firewallChanges.Count -eq 4 -and $script:firewallChanges[-1] -ceq 'update') 'Private LAN left a separate client rule.'
    Assert ($script:firewall.RemoteAddress -ceq 'LocalSubnet' -and $script:firewall.Profile -ceq 'Private' -and
        $script:firewall.Program -ceq (Join-Path $root 'app/eitmad-server.exe') -and
        $script:firewall.LocalPort -eq 58443 -and $script:firewall.LocalAddress -ceq '192.168.50.10' -and
        $script:firewall.Protocol -ceq 'TCP' -and $script:firewall.Direction -ceq 'Inbound' -and
        $script:firewall.Enabled -ceq 'True' -and $script:firewall.Action -ceq 'Allow' -and
        $script:firewall.EdgeTraversalPolicy -ceq 'Block') 'Private LAN escaped the installed HTTPS scope.'
    $script:firewall.Group = 'Another installation'
    Reject { Set-ClientFirewall $settings -PrivateLan } 'another installation'
    Assert ($script:firewallChanges.Count -eq 4) 'Private LAN changed a foreign rule.'
    $script:firewall.Group = 'Eitmad host: ' + $root
    Invoke-EitmadHost Firewall -StateDirectory $root -Name EitmadTest -ClientAddress '192.168.50.24'
    Assert ($script:firewall.RemoteAddress -ceq '192.168.50.24') 'An exact client rule retained LocalSubnet access.'
    Invoke-EitmadHost Firewall -StateDirectory $root -Name EitmadTest -PrivateLan
    Invoke-EitmadHost Uninstall -StateDirectory $root -Name EitmadTest
    Assert (-not $script:firewall -and $script:firewallChanges[-1] -ceq 'remove') 'Uninstall retained the client allow rule.'
    Assert (Test-Path -LiteralPath (Join-Path $root 'retained-data')) 'Firewall configuration changed durable data.'
    Write-Host 'PASS: real argv quoting; mocked SCM startup, readiness, shutdown, port conflicts, ownership, bounded recovery, and preserved uninstall data.'
    Write-Host 'PASS: exact client firewall scope, replacement, prompt retry/cancel, preview, ownership, and removal on uninstall.'
    Write-Host 'PASS: Private-only LocalSubnet HTTPS scope, preview, conflicting option rejection, ownership, and replacement.'
} $evidence
Write-Host "Synthetic evidence: $evidence"
