using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.Products;
namespace Eitmad.WindowsShell.Tests.Products;
[TestClass]
public sealed class ProductsPresentationTests
{
    internal static ProductSnapshot Data(long revision = 1)
    {
        var scope = new ScopeRef { Kind = "organization", Id = Guid.Parse("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa") };
        var category = new ProductCategory { Id = Guid.Parse("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb"), Scope = scope, Name = "المراتب", Revision = 1 };
        var product = new Product { Id = Guid.Parse("cccccccc-cccc-cccc-cccc-cccccccccccc"), Scope = scope, CategoryId = category.Id, CategoryName = category.Name, Name = "مرتبة طبية", Description = "منتج جاهز", Notes = "ملاحظة المورد", Revision = revision, Variants = [new Eitmad.Contracts.ProductVariant { Id = Guid.Parse("dddddddd-dddd-dddd-dddd-dddddddddddd"), Name = "مفرد", PurchaseCostYer = 55000 }, new Eitmad.Contracts.ProductVariant { Id = Guid.Parse("eeeeeeee-eeee-eeee-eeee-eeeeeeeeeeee"), Name = "مزدوج", PurchaseCostYer = 80000 }] };
        return new(new ProductCategories { Items = [category] }, [product], true, true);
    }
    [TestMethod]
    public void RefreshPreservesReviewedRevisionVariantAndCategoryIdentity()
    {
        var model = new ProductsViewModel(); model.ApplyDurableData(Data()); model.BeginEdit(model.VisibleProducts.Single());
        model.EditorName = "تعديل محلي"; var variant = model.Variants[0].Id; model.ApplyDurableData(Data(2));
        var input = model.SaveInput(); Assert.AreEqual(1L, input.ExpectedRevision); Assert.AreEqual(variant, input.Variants[0].Id); Assert.AreEqual("تعديل محلي", input.Name);
        model.RemoveVariant(model.Variants[1]); Assert.IsTrue(model.SaveInput().Variants[1].Archived);
        model.BeginDuplicate(model.VisibleProducts.Single()); input = model.SaveInput(); Assert.IsNull(input.Id); Assert.IsNull(input.ExpectedRevision); Assert.AreNotEqual(variant, input.Variants[0].Id);
    }
    [TestMethod]
    public void RedactedProjectionAndSessionResetClearInternalEditorData()
    {
        var model = new ProductsViewModel(); var data = Data(); model.ApplyDurableData(data); model.BeginEdit(model.VisibleProducts.Single());
        data.Products[0].Notes = ""; foreach (var v in data.Products[0].Variants) v.PurchaseCostYer = null;
        model.ApplyDurableData(data with { CanManage = false, CanReadCosts = false }); Assert.IsFalse(model.IsEditorOpen); Assert.AreEqual("", model.Notes); Assert.HasCount(0, model.Variants); Assert.AreEqual("—", model.VisibleProducts.Single().PurchaseCostLabel);
        model.ClearSession(); Assert.HasCount(0, model.VisibleProducts); Assert.HasCount(0, model.Categories); Assert.IsFalse(model.CanManage);
    }
    [TestMethod]
    public void ProductDefinitionsDoNotPublishSellingPricesOrNewSalesSelections()
    {
        var model = new ProductsViewModel(); model.ApplyDurableData(Data()); Assert.IsFalse(model.GetSalesCatalogItems().Any()); Assert.IsNull(model.GetSalesSelection(model.VisibleProducts.Single().Id));
        model.RequestArchive(model.VisibleProducts.Single()); var input = model.ArchiveInput(); Assert.IsTrue(input.Archived); Assert.AreEqual(1L, input.ExpectedRevision); Assert.IsFalse(model.VisibleProducts.Single().IsArchived);
    }
}
