#Requires -Version 7.0
[CmdletBinding()]
param()
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
Import-Module (Join-Path $PSScriptRoot 'installation.psm1') -Force
& (Get-Module installation) {
    function Assert($Condition, [string]$Message) { if (-not $Condition) { throw $Message } }
    try { throw 'Protected offline copy failed. ورشة اختبار' } catch { $failure = $_ }
    $frame = New-InstallationWorkerFailure $failure
    $details = Read-InstallationWorkerFailure $frame
    $diagnostic = $details | ConvertFrom-Json
    Assert ($diagnostic.message -ceq $failure.Exception.Message) 'The original worker failure was lost.'
    Assert ($diagnostic.line -gt 0 -and $diagnostic.stack) 'Worker failure location was lost.'
    Assert ($null -eq (Read-InstallationWorkerFailure '{"version":1,"stage":"verifyHttps","status":"running"}')) 'Progress was treated as an adapter failure.'
    try { throw ('<"ورشة' * 10000) } catch { $large = New-InstallationWorkerFailure $_ }
    Assert ([Text.Encoding]::UTF8.GetByteCount($large) -le 32768) 'Worker failure exceeds the authenticated frame bound.'
    Assert ((Read-InstallationWorkerFailure $large).Length -le 4000) 'Worker diagnostic exceeds its text bound.'
    foreach ($invalid in '{"version":2,"kind":"adapterFailure","details":"failure"}',
        '{"version":1,"kind":"adapterFailure","details":17}') {
        $rejected = $false
        try { [void](Read-InstallationWorkerFailure $invalid) } catch { $rejected = $true }
        Assert $rejected 'Malformed adapter failure was accepted.'
    }
}

# Exercise the real preparation script's early failure, without tools or services.
$root = Join-Path ([IO.Path]::GetTempPath()) ('eitmad-preparation-diagnostic-' + [Guid]::NewGuid().ToString('N'))
[void][IO.Directory]::CreateDirectory($root)
$sentinel = Join-Path $root 'existing-state.txt'
[IO.File]::WriteAllText($sentinel, 'synthetic retained state')
$start = [Diagnostics.ProcessStartInfo]::new('pwsh.exe')
$start.UseShellExecute = $false
$start.CreateNoWindow = $true
$start.RedirectStandardOutput = $true
$start.RedirectStandardError = $true
$start.StandardErrorEncoding = [Text.Encoding]::UTF8
foreach ($argument in '-NoProfile', '-File', (Join-Path $PSScriptRoot 'prepare-workshop.ps1'),
    '-Action', 'Initialize', '-StateDirectory', $root, '-PostgresBin', $root,
    '-CertificateExecutable', (Join-Path $root 'unused.exe'), '-ServerName', 'synthetic', '-DatabasePort', '5433') {
    $start.ArgumentList.Add($argument)
}
$child = [Diagnostics.Process]::Start($start)
try {
    $errors = $child.StandardError.ReadToEndAsync()
    $output = $child.StandardOutput.ReadToEndAsync()
    if (-not $child.WaitForExit(10000)) { $child.Kill(); throw 'Preparation failure did not return.' }
    $text = $errors.GetAwaiter().GetResult()
    $diagnostic = $text | ConvertFrom-Json
    if ($child.ExitCode -eq 0 -or $diagnostic.message -cne 'Fresh setup requires a new directory. Existing state was not changed.' -or
        $diagnostic.line -le 0 -or -not $diagnostic.stack -or $output.GetAwaiter().GetResult()) {
        throw 'Preparation did not export the original bounded diagnostic.'
    }
    if ([IO.File]::ReadAllText($sentinel) -cne 'synthetic retained state') { throw 'Existing state changed.' }
} finally { $child.Dispose() }
Write-Host 'PASS: Worker failure framing, Arabic diagnostic, frame limits, malformed frames, and real preparation failure with retained state.'
