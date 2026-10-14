using System.Reflection;
using Eitmad.WindowsShell.Tests.Rendered;
class AuditProgram {
[STAThread] static void Main(string[] args) {
if (args.Contains("Supplement")) { try { Supplement.Run(); } catch(Exception ex) { System.IO.File.WriteAllText("artifacts/ui-audit-2026-10-14-fullscreen-125/supplement-error.txt", ex.ToString()); Console.WriteLine(ex); } return; }
var tasks = new (string Type, string Method, object[] Args)[] {
("SignIn", "FormAndConnectionFailureStayWithinScrollableViewport", [1920,1080]),
("Home", "ScopedHomeRendersForBothRolesAndActionsHaveKeyboardNames", [1920,1080]),
("Home", "ReceptionDailyWorkIsVisibleWithoutScrolling", [1920,1080]),
("HomeSearch", "SubmittedHomeSearchRendersAndClearsForBothRoles", [1920,1080]),
("RawMaterials", "DurableListAndEditorsRenderAtBaselineSizes", [1920,1080]),
("Parts", "CreateWizardRendersAccessibleStepsAndMaterialPicker", [1920,1080]),
("Furniture", "CompleteEditorSavesReopensAndKeepsConflictAndRetryFields", [1920,1080]),
("Furniture", "PricingGroupsStartWithVariantIdentityAndContainLongNames", [1920,1080]),
("Furniture", "OptionGroupsRenderAdjustmentsAndKeepTheirOwnActions", [1920,1080]),
("Furniture", "InformationStartsWithIdentifyingFieldsAndKeepsImageReachable", [1920,1080,true]),
("Furniture", "ColorSelectorKeepsEveryAllowedVisualAndPriceDifference", [1920,1080]),
("Products", "DurableListAndSupplierEditorRenderAtBaselineSizes", [1920,1080]),
("Pricing", "PricingListEditorAndBelowCostWarningRenderAtBaselineSizes", [1920,1080]),
("DialogHost", "FeatureDialogsRenderAndManagersRecoverAfterValidationFailure", [1920]),
("SalesCatalogAuthority", "LiveCatalogAndSelectionsKeepNativeKeyboardPathsAndMixedDirectionValues", [1920,1080]),
("SalesCatalogAuthority", "SelectionReviewStaysVisibleWhileOptionsScrollInReceptionShell", [1920,1080]),
("SalesCatalogAuthority", "CustomDimensionsFitTogetherAndKeepPendingAndInvalidGuidance", [1920,1080]),
("Customers", "ReceptionCustomerSelectionRendersRustSearchResults", [1920,1080]),
("Customers", "DetailEditKeepsHistoryAndCancelDiscardsDraft", [1920,1080]),
("Customers", "CustomerSyncStateRendersAndBlocksUnresolvedEdits", [1920,1080]),
("QuotationEvaluation", "LiveQuotationShowsReturnedSummaryAndClearsInvalidInputTotals", [1920,1080]),
("QuotationDraft", "MixedDraftSaveReopenAndFurnitureEditingUseTheRenderedControls", [1920,1080]),
("QuotationLifecycle", "ConfirmedQuotationActionsRenderForBothRoles", [1920,1080]),
("DiscountApproval", "ConfirmedApprovalControlsRenderAndRejectThroughTheServerReply", [1920,1080]),
("ReceptionQuotations", "ReceptionListFiltersAndDetailActionsRespectStatus", []),
("ReceptionQuotations", "ConversionConfirmsSnapshotAndCancelPreservesQuotation", []),
("OrderAuthority", "ConfirmedOrderActionsRenderForBothRoles", [1920,1080]),
("ReceptionOrders", "ReceptionNavigationFiltersReadyAndCustomerDocuments", []),
("WorkOrderAuthority", "ConfirmedProductionInputsRenderWithKeyboardAccess", [1920,1080]),
("WorkOrderAuthority", "OrderNotesKeepEmptyStateCompactAndPreserveWrappedAuthorityText", [1920,1080]),
("Users", "UsersNavigationFiltersAndEditorPageWorkInTheRenderedShell", []),
("Users", "SameDisplayNameShowsDistinctLoginIdentitiesInRowsAndConfirmation", [1920d,1080d]),
("Users", "CreateValidationIdentifiesPasswordAndAuthorityFailureUsesFormSummary", [1920d,1080d]),
("Users", "FailedRefreshLabelsRetainedRowsAndRetryRestoresCurrentState", [1920d,1080d]),
("AccountSwitch", "ToolbarOpensTheDialogAndCancelKeepsTheAccount", [1920,1080,"manager"]),
("CustomerDocuments", "SavedDocumentsRenderAndNativePrintRoutesDenyAccess", [1920,1080]),
("CustomerDocuments", "MultiPageNativeExportPreservesArabicAndTotals", [1920,1080])
};
foreach (var task in tasks) {
 if (args.Length > 0 && !args.Contains(task.Type)) continue;
 Console.WriteLine($"START {task.Type}.{task.Method}");
 try { var type=typeof(WpfTestHost).Assembly.GetType($"Eitmad.WindowsShell.Tests.Rendered.{task.Type}RenderedTests")!; type.GetMethod(task.Method)!.Invoke(Activator.CreateInstance(type),task.Args); Console.WriteLine("PASS"); }
 catch(Exception e) { Console.WriteLine("INCOMPLETE " + (e.InnerException ?? e)); }
}
}}
