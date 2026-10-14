using System.Windows;
using System.Windows.Controls;
using Eitmad.Contracts;
using Eitmad.WindowsShell.Controls;
using Eitmad.WindowsShell.Features.Reception;
using Eitmad.WindowsShell.Features.Orders;
using Eitmad.WindowsShell.Tests.Rendered;
using Eitmad.WindowsShell.Tests.Furniture;
using Eitmad.WindowsShell.Tests.Products;
using Eitmad.WindowsShell.Tests.Home;
static class Supplement {
static void Shot(Eitmad.WindowsShell.MainWindow w,string name) { WpfTestHost.CompleteLayout(w); WpfTestHost.Capture(w,name); }
static void Nav(Eitmad.WindowsShell.MainWindow w,string name) { WpfTestHost.FindByName<Button>(w,name).RaiseEvent(new RoutedEventArgs(Button.ClickEvent)); WpfTestHost.CompleteLayout(w); }
public static void Run() {
WpfTestHost.Run(1920,1080,w=>{
var f = new FurnitureFixtures(); f.Seed(); Nav(w,"FurnitureNavButton"); w.FurnitureSurface.ViewModel.ApplyDurableData(f.Snapshot()); Shot(w,"audit-furniture-list");
var m=w.FurnitureSurface.ViewModel; m.BeginEdit(m.VisibleFurniture.First()); m.MoveToParts(); Shot(w,"audit-furniture-parts");
m.MoveToVariants(); Shot(w,"audit-furniture-sizes");
m.BeginEditVariant(m.Variants[0]); Shot(w,"audit-furniture-size-edit");
foreach(var s in WpfTestHost.Descendants<ScrollViewer>(w).Where(s=>s.IsVisible && s.ScrollableHeight>0)) s.ScrollToBottom(); Shot(w,"audit-furniture-size-edit-bottom");
m.CancelVariantEditor(); m.CancelEditor();
Nav(w,"MaterialsNavButton"); var r=w.RawMaterialsSurface.ViewModel; r.BeginManageCategories(); Shot(w,"audit-material-categories");r.CloseReferenceManager();r.BeginAddCategory();Shot(w,"audit-material-category-add");r.CancelReferenceEditor();r.BeginCreate();Shot(w,"audit-material-add");r.CancelEditor();
Nav(w,"ProductsNavButton"); var p=w.ProductsSurface.ViewModel; p.ApplyDurableData(ProductsPresentationTests.Data());p.BeginCreate();Shot(w,"audit-product-add"); p.HasVariants=false;
foreach(var s in WpfTestHost.Descendants<ScrollViewer>(w.ProductsSurface).Where(s=>s.IsVisible && s.ScrollableHeight>0)) s.ScrollToBottom();Shot(w,"audit-product-without-variants");p.CancelEditor();
Nav(w,"WorkOrdersNavButton");Shot(w,"audit-workorders-shell-list");var work=w.WorkOrdersSurface.ViewModel; if(work.VisibleWorkOrders.Count>0){work.OpenWorkOrder(work.VisibleWorkOrders[0]);Shot(w,"audit-workorders-shell-detail");}
});
WpfTestHost.Run(1920,1080,w=>{
var engine=HomeStateTests.Engine(HomeStateTests.Data());var home=new Eitmad.WindowsShell.Features.Home.HomeViewModel(engine); home.ActivateAsync().GetAwaiter().GetResult();w.ManagerHome.DataContext=home; Shot(w,"audit-home-manager-top");
foreach(var s in WpfTestHost.Descendants<ScrollViewer>(w.DashboardSurface).Where(s=>s.IsVisible))s.ScrollToTop();Shot(w,"audit-home-manager-start");
w.ResponsiveRoot.Visibility=Visibility.Collapsed;w.ReceptionistSurface.Visibility=Visibility.Visible;w.ReceptionistSurface.AttachHome(home);WpfTestHost.CompleteLayout(w);w.ReceptionistSurface.ProductsAction.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));Shot(w,"audit-reception-catalog");
});
new HomeRenderedTests().HomeSectionResultsRenderWithoutFalseEmptyMessagesForBothRoles(1920,1080);
WpfTestHost.Run(1920,1080,w=>{
foreach(var kind in new[]{"quotation","order","draft","multipage"}){
var saved=CustomerDocumentsRenderedTests.Saved(kind=="order");
if(kind=="draft"){saved.IsDraft=true;saved.CanPrint=false;saved.Number=null!;saved.Status="مسودة";saved.IssuedAt=null;saved.ValidUntil=null;}
if(kind=="multipage"){saved.Customer.Name="شركة الأثاث التجريبية";saved.Lines=Enumerable.Range(0,36).Select(i=>new CustomerDocumentLine{Name=$"خزانة تجريبية A-{i}",Description="نص عربي طويل لقياس التفاف السطر",VariantName="بابان",Dimensions=new(){WidthMm=1200,HeightMm=1800,DepthMm=600},Quantity=1,UnitPriceYer=12500,TotalYer=12500}).ToArray();saved.SubtotalYer=450000;saved.DiscountYer=22500;saved.TotalYer=427500;}
var preview=new PrintPreview{Document=kind=="order"?OrderCustomerDocument.CreateSaved(saved):QuotationCustomerDocument.CreateSaved(saved),CanPrint=saved.CanPrint};w.Content=preview;WpfTestHost.CompleteLayout(w);preview.Pages.FitToMaxPagesAcross(1);WpfTestHost.CompleteLayout(w);
for(int i=1;i<=preview.Pages.PageCount;i++){preview.Pages.GoToPage(i);Shot(w,$"audit-print-{kind}-page-{i}");}
}
});
new ReceptionQuotationsRenderedTests().NewQuotationWindowStartsInCatalogAndCanReturnToReview();
new QuotationFinalActionsRenderedTests().RequiredFieldsPreviewBackAndCustomerOnlyPagination();
}
}
