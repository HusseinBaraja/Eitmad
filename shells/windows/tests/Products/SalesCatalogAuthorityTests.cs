using System.Text.Json;
using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.Reception;
using Eitmad.WindowsShell.Features.Furniture;
using Eitmad.WindowsShell.Features.Products;
using Eitmad.WindowsShell.Tests.TestDoubles;

namespace Eitmad.WindowsShell.Tests.Products;

[TestClass]
public sealed class SalesCatalogAuthorityTests
{
    internal static CatalogEntry Entry(bool furniture = false)
    {
        var target = furniture ? PriceTarget.ForFurniture(new FurnitureReference { Scope = new() { Kind = "organization", Id = Guid.NewGuid() }, FurnitureId = Guid.NewGuid(), VariantId = Guid.NewGuid(), Revision = 1, SchemaVersion = 1 })
            : PriceTarget.ForProduct(new ProductReference { Scope = new() { Kind = "organization", Id = Guid.NewGuid() }, ProductId = Guid.NewGuid(), VariantId = Guid.NewGuid(), Revision = 1, SchemaVersion = 1 });
        var dims = new FurnitureDimensions { WidthMm = 1200, HeightMm = 2000, DepthMm = 600 };
        return new() {
            Price = new() { Target = JsonSerializer.Deserialize<Dictionary<string, object>>(JsonSerializer.Serialize(target))!, Currency = "YER", SellingPriceYer = 12500, Revision = 1, ConfirmedAt = 100, Colors = [], Handles = [] },
            Name = furniture ? "خزانة تجريبية" : "مرتبة تجريبية", CategoryName = furniture ? "خزائن" : "مراتب", Description = "صنف من كتالوج مؤكد", VariantName = "نموذج A-12",
            Colors = [], Handles = [], Dimensions = furniture ? dims : null!,
            Customization = furniture ? new() { Minimum = dims, Maximum = new() { WidthMm = 1500, HeightMm = 2000, DepthMm = 600 } } : null!
        };
    }
    internal static QueryResponseEnvelope Response(QueryResult value) => new() { Outcome = new() { Status = CommandOutcomeStatus.Succeeded, Payload = value } };
    internal static QueryResponseEnvelope Failure(string code) => new() { Outcome = new() { Status = CommandOutcomeStatus.Failed, Payload = new() { Code = code } } };
    internal static QueryResponseEnvelope Handle(Query query, CatalogEntry entry)
    {
        if (query.AsSalesCatalogList() is not null) return Response(QueryResult.ForSalesCatalog(new() { Items = [entry], Categories = [entry.CategoryName], ServerAvailable = false }));
        if (query.AsSalesCatalogGet() is not null) return Response(QueryResult.ForSalesCatalogItem(new() { Variants = [entry], ServerAvailable = false }));
        if (query.AsSalesCatalogCheck() is { } check) return Response(QueryResult.ForSalesConfiguration(new() {
            Entry = entry, Dimensions = check.Dimensions, Price = new() { Snapshot = entry.Price, UnitPriceYer = 12777, TotalYer = 25554 }, AdditionsYer = 277, ServerAvailable = false
        }));
        return Failure(ProtocolIds.ErrorCodes.EitmadErrorContractInvalidV1);
    }
    private static SalesCatalogViewModel Model(SalesCatalogClient client)
    {
        var model = new SalesCatalogViewModel(new FurnitureViewModel(), new ProductsViewModel()); model.AttachCatalogClient(client); return model;
    }
    [TestMethod]
    public async Task LiveCatalogUsesRustTotalsRechecksAddAndKeepsSnapshotDuringEditing()
    {
        await using var engine = new FakeEngine(); var entry = Entry(); engine.QueryHandler = q => Handle(q, entry);
        await using var client = new SalesCatalogClient(engine); var model = Model(client);
        await model.ActivateCatalogAsync(); Assert.HasCount(1, model.VisibleItems); Assert.IsTrue(model.CatalogStatus.Contains("قد تكون قديمة"));
        model.Select(model.VisibleItems[0]); await model.LastCatalogOperation;
        var selection = model.ProductSelection!; Assert.IsFalse(selection.CanAdd);
        selection.SelectedVariant = selection.Variants[0]; selection.Quantity = 2; await model.LastCatalogOperation;
        Assert.IsTrue(selection.CanAdd); Assert.AreEqual(12777m, selection.UnitPrice); Assert.AreEqual(25554m, selection.LineTotal);
        Assert.IsTrue(await model.AddValidatedSelectionAsync(true)); Assert.AreEqual(25554m, model.QuotationLines[0].LineTotal);
        selection.Quantity = 3; Assert.IsFalse(selection.CanAdd); Assert.AreEqual(25554m, model.QuotationLines[0].LineTotal);
        engine.QueryHandler = q => q.AsSalesCatalogCheck() is not null ? Failure(ProtocolIds.ErrorCodes.EitmadErrorPricingRevisionConflictV1) : Handle(q, entry);
        Assert.IsFalse(await model.AddValidatedSelectionAsync(true)); Assert.HasCount(1, model.QuotationLines); Assert.IsTrue(selection.Guidance.Contains("تغير سعر البيع"));
        model.EditLine(model.QuotationLines[0]); await model.LastCatalogOperation;
        await model.RefreshSelectionAsync(); Assert.IsTrue(model.ProductSelection!.IsEditing);
        Assert.AreEqual("حفظ التعديلات", model.ProductSelection.ActionLabel);
        Assert.IsFalse(model.ProductSelection.CanAdd); Assert.AreEqual(25554m, model.QuotationLines[0].LineTotal);
        await model.DeactivateCatalogAsync(); Assert.IsFalse(model.IsSelecting); Assert.HasCount(0, model.QuotationLines);
    }
    [TestMethod]
    public async Task SearchAndCategoryGoToOneBoundedRustPageAndDenialClearsProjection()
    {
        await using var engine = new FakeEngine(); var entry = Entry(); engine.QueryHandler = q => Handle(q, entry);
        await using var client = new SalesCatalogClient(engine); var model = Model(client); await model.ActivateCatalogAsync();
        ListSalesCatalog? seen = null;
        engine.QueryHandler = q => { seen = q.AsSalesCatalogList(); return Response(QueryResult.ForSalesCatalog(new() { Items = [], Categories = [entry.CategoryName], Next = Guid.NewGuid() })); };
        model.SearchText = "مرتبه A-12"; model.SelectedCategory = entry.CategoryName; await model.LastCatalogOperation;
        Assert.AreEqual("مرتبه A-12", seen!.Term); Assert.AreEqual(entry.CategoryName, seen.Category); Assert.AreEqual(30, seen.Limit); Assert.IsTrue(model.HasNextPage);
        var queries = 0; engine.QueryHandler = _ => { queries++; return Failure(ProtocolIds.ErrorCodes.EitmadErrorAuthorizationDeniedV1); };
        await model.NextPageAsync(); Assert.AreEqual(1, queries); Assert.HasCount(0, model.VisibleItems); Assert.IsFalse(model.HasNextPage); Assert.IsFalse(model.IsEmpty); Assert.IsTrue(model.CatalogStatus.Contains("صلاحية"));
    }
    [TestMethod]
    public async Task LateSelectionAndValidationCannotReturnAfterSessionEndsOrInputChanges()
    {
        await using var engine = new FakeEngine(); var entry = Entry(true); engine.QueryHandler = q => Handle(q, entry);
        await using var client = new SalesCatalogClient(engine); var model = Model(client); await model.ActivateCatalogAsync();
        model.Select(model.VisibleItems[0]); await model.LastCatalogOperation;
        var selection = model.Selection!; selection.SelectedSize = selection.Sizes[0]; await model.LastCatalogOperation;
        Assert.IsTrue(selection.CanCustomize); Assert.IsTrue(selection.CanAdd);
        var pending = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var entered = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        engine.QueryBarrier = q => { if (q.AsSalesCatalogCheck() is not null) { entered.TrySetResult(); return pending.Task; } return Task.CompletedTask; };
        selection.WidthCm = "١٣٠٫٥"; var oldCheck = model.LastCatalogOperation; await entered.Task.WaitAsync(TimeSpan.FromSeconds(3));
        Assert.IsFalse(selection.CanAdd);
        selection.WidthCm = "invalid"; await model.LastCatalogOperation; pending.SetResult(); await oldCheck;
        Assert.IsFalse(selection.CanAdd); Assert.IsTrue(selection.Guidance.Contains("السنتيمتر"));
        await model.DeactivateCatalogAsync(); Assert.IsFalse(model.IsSelecting);
    }
    [TestMethod]
    public async Task EngineDisconnectionAndArchivedItemRemainExplicitAndCannotAdd()
    {
        await using var engine = new FakeEngine(); var entry = Entry(); engine.QueryHandler = q => Handle(q, entry);
        await using var client = new SalesCatalogClient(engine); var model = Model(client); await model.ActivateCatalogAsync();
        model.Select(model.VisibleItems[0]); await model.LastCatalogOperation;
        model.ProductSelection!.SelectedVariant = model.ProductSelection.Variants[0]; await model.LastCatalogOperation;
        engine.QueryHandler = _ => throw new System.IO.IOException("Synthetic disconnection");
        Assert.IsFalse(await model.AddValidatedSelectionAsync(true)); Assert.IsTrue(model.ProductSelection.Guidance.Contains("تعذر الاتصال"));
        engine.QueryHandler = _ => Failure(ProtocolIds.ErrorCodes.EitmadErrorPricingReferenceInvalidV1);
        await model.RefreshSelectionAsync(); Assert.IsFalse(model.IsSelecting); Assert.IsTrue(model.SelectionNotice.Contains("مؤرشف"));
    }
}
