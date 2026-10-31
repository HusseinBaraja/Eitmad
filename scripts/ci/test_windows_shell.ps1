# Run complete, disjoint class groups in separate processes; each keeps its own WPF dispatcher.
[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [ValidateRange(1, 3)]
    [int]$Partition,
    [switch]$NoBuild,
    [string]$ResultsDirectory = 'artifacts/windows-shell-tests'
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# Balance the slow rendered classes using recorded test durations. New classes
# automatically run in partition 3, so adding a test never requires updating this list.
$first = @(
    'AccountSwitchRenderedTests', 'DialogHostRenderedTests', 'QuotationDraftRenderedTests',
    'CatalogImageRenderedTests', 'FurnitureRenderedTests', 'CompositionControlsRenderedTests',
    'SalesCatalogAuthorityRenderedTests', 'CustomerResolutionRenderedTests', 'ServerSelectionLeaseRenderedTests',
    'SharedControlsRenderedTests', 'DesktopEnrollmentRenderedTests', 'ProductsLoadingRenderedTests',
    'ReceptionQuotationsRenderedTests', 'WorkOrderAuthorityRenderedTests', 'ManagerCustomersRenderedTests',
    'ReceptionOrdersRenderedTests', 'QuotationsRenderedTests', 'HomeSearchRenderedTests'
)
$second = @(
    'CustomerDocumentsRenderedTests', 'OfflineSetupPayloadTests', 'CustomersRenderedTests',
    'OnboardingRenderedTests', 'CatalogImageThumbnailsTests', 'PricingRenderedTests',
    'EditorViewportRenderedTests', 'ProductsRenderedTests', 'QuotationEvaluationRenderedTests',
    'FurniturePublicationRenderedTests', 'QuotationLifecycleRenderedTests', 'MainWindowRenderedTests',
    'OrderAuthorityRenderedTests', 'SalesCatalogAuthorityTests', 'QuotationEvaluationTests',
    'PreviewHandoffRenderedTests', 'PairingRecoveryRenderedTests', 'SignInRenderedTests'
)
if (@($first | Where-Object { $_ -in $second }).Count -ne 0) {
    throw 'Windows shell test partitions overlap.'
}
$filter = switch ($Partition) {
    1 { ($first | ForEach-Object { "FullyQualifiedName~.$_." }) -join '|' }
    2 { ($second | ForEach-Object { "FullyQualifiedName~.$_." }) -join '|' }
    3 { (@($first + $second) | ForEach-Object { "FullyQualifiedName!~.$_." }) -join '&' }
}
$arguments = @('test', 'shells/windows/tests/Eitmad.WindowsShell.Tests.csproj',
    '--configuration', 'Release', '--nologo', '-m:1', '--filter', $filter,
    '--logger', "trx;LogFileName=partition-$Partition.trx", '--results-directory', $ResultsDirectory)
if ($NoBuild) { $arguments += '--no-build' }
& dotnet @arguments
exit $LASTEXITCODE
