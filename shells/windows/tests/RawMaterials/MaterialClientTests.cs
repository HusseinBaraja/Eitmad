using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.RawMaterials;
using Eitmad.WindowsShell.Tests.TestDoubles;

namespace Eitmad.WindowsShell.Tests.RawMaterials;

[TestClass]
public sealed class MaterialClientTests
{
    [TestMethod]
    public async Task LoadsTypedReferencesAndPagesAndSubmitsStableIdsAndRevision()
    {
        await using var engine = new FakeEngine();
        var scope = new ScopeRef { Kind = "organization", Id = Guid.NewGuid() };
        var category = new MaterialCategory
            { Id = Guid.NewGuid(), Scope = scope, Name = "أخشاب", Revision = 2 };
        var unit = new MaterialUnit
            { Id = Guid.NewGuid(), Scope = scope, Name = "متر", Symbol = "م",
              Dimension = UnitDimension.Length, Numerator = 1, Denominator = 1, Revision = 1 };
        var material = new Material
            { Id = Guid.NewGuid(), Scope = scope, Name = "خشب زان", CategoryId = category.Id,
              UnitId = unit.Id, CurrentCostYer = 8_000, Revision = 3 };
        engine.QueryHandler = query => Success(query.Kind == Query.MaterialReferenceListKind
            ? QueryResult.ForMaterialReferences(new MaterialReferences { Categories = [category], Units = [unit] })
            : QueryResult.ForMaterials(new MaterialPage { Items = [material] }));
        engine.CommandHandler = _ => new CommandResponseEnvelope
        {
            RequestId = Guid.NewGuid(), CorrelationId = Guid.NewGuid(),
            Outcome = new CommandOutcome { Status = CommandOutcomeStatus.Succeeded, Payload = new CommandResult() },
        };
        await using var client = new MaterialClient(engine);

        var snapshot = await client.LoadAsync("اخشاب");
        Assert.IsTrue(snapshot.Succeeded);
        Assert.AreEqual(material.Id, snapshot.Value!.Materials.Single().Id);
        Assert.AreEqual(category.Id, snapshot.Value.References.Categories.Single().Id);

        var failure = await client.SaveAsync(new SaveMaterial
        {
            Id = material.Id, ExpectedRevision = material.Revision, Name = material.Name,
            CategoryId = category.Id, UnitId = unit.Id, CurrentCostYer = 8_500,
        });
        Assert.AreEqual(MaterialFailureKind.None, failure);
        var sent = engine.LastCommand!.AsMaterialSave()!;
        Assert.AreEqual(material.Id, sent.Id);
        Assert.AreEqual(3, sent.ExpectedRevision);
        Assert.AreEqual(category.Id, sent.CategoryId);
        Assert.AreEqual(unit.Id, sent.UnitId);
        Assert.AreEqual(8_500, sent.CurrentCostYer);
    }

    [TestMethod]
    public async Task ConflictKeepsUnsavedDataForReview()
    {
        await using var engine = new FakeEngine
        {
            CommandHandler = _ => new CommandResponseEnvelope
            {
                RequestId = Guid.NewGuid(), CorrelationId = Guid.NewGuid(),
                Outcome = new CommandOutcome { Status = CommandOutcomeStatus.Failed,
                    Payload = new CommandResult { Code = ProtocolIds.ErrorCodes.EitmadErrorMaterialRevisionConflictV1 } },
            },
        };
        await using var client = new MaterialClient(engine);
        var failure = await client.SaveAsync(new SaveMaterialCategory
            { Id = Guid.NewGuid(), ExpectedRevision = 1, Name = "أخشاب" });
        Assert.AreEqual(MaterialFailureKind.Conflict, failure);
        StringAssert.Contains(MaterialClient.ArabicMessage(failure), "لم تُحفظ تعديلاتك");
    }

    private static QueryResponseEnvelope Success(QueryResult payload) => new()
    {
        RequestId = Guid.NewGuid(), CorrelationId = Guid.NewGuid(),
        Outcome = new QueryOutcome { Status = CommandOutcomeStatus.Succeeded, Payload = payload },
    };
}
