#Requires -Version 7.4
[CmdletBinding(DefaultParameterSetName = 'Sign')]
param(
    [Parameter(Mandatory, ParameterSetName = 'Sign')][string]$Path,
    [Parameter(Mandatory, ParameterSetName = 'Export')][string]$ExportCertificate
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
Import-Module (Join-Path $PSScriptRoot 'signing-certificate.psm1') -Force
$certificate = Get-EitmadSigningCertificate
if ($PSCmdlet.ParameterSetName -eq 'Export') {
    $bytes = $certificate.Export([Security.Cryptography.X509Certificates.X509ContentType]::Cert)
    $file = [IO.File]::Open($ExportCertificate, 'CreateNew', 'Write', 'None')
    try { $file.Write($bytes, 0, $bytes.Length) } finally { $file.Dispose() }
    return
}
$signtool = Get-Command signtool.exe -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source -First 1
if (-not $signtool) {
    $kits = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
    $signtool = Get-ChildItem -Path (Join-Path $kits '10.*\x64\signtool.exe') |
        Sort-Object { [version]$_.Directory.Parent.Name } -Descending |
        Select-Object -ExpandProperty FullName -First 1
}
if (-not $signtool -or (Get-AuthenticodeSignature -LiteralPath $signtool).Status -ne 'Valid') {
    throw 'Install the authentic Windows SDK x64 SignTool on the build computer.'
}
& $signtool sign /fd SHA256 /sha1 $certificate.Thumbprint /s My $Path
if ($LASTEXITCODE -ne 0) { throw 'Local self-signed signing failed.' }
