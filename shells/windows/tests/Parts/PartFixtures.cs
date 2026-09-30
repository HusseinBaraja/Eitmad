using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.Parts;
using Eitmad.WindowsShell.Features.RawMaterials;
using Eitmad.WindowsShell.Tests.TestDoubles;

namespace Eitmad.WindowsShell.Tests.Parts;

// Fixed synthetic Rust responses, without domain calculation in the fixture.
internal sealed class PartFixtures
{
    public ScopeRef Scope { get; } = new() { Kind = "organization", Id = Guid.NewGuid() };
    public PartCategory Category { get; }
    public MaterialCategory MaterialCategory { get; }
    public MaterialUnit[] Units { get; }
    public Material[] Materials { get; }
    public List<Part> Parts { get; } = [];
    public PartCost Cost { get; }
    public PartFixtures()
    {
        Category = new PartCategory { Id = Guid.NewGuid(), Scope = Scope, Name = "خزانة ملابس", Revision = 1 };
        MaterialCategory = new MaterialCategory { Id = Guid.NewGuid(), Scope = Scope, Name = "ألواح وحواف", Revision = 1 };
        Units = [new MaterialUnit { Id = Guid.NewGuid(), Scope = Scope, Name = "متر مربع", Symbol = "m²", Dimension = UnitDimension.Area, Numerator = 1, Denominator = 1, Revision = 1 },
            new MaterialUnit { Id = Guid.NewGuid(), Scope = Scope, Name = "متر", Symbol = "m", Dimension = UnitDimension.Length, Numerator = 1, Denominator = 1, Revision = 1 }];
        Materials = [new Material { Id = Guid.NewGuid(), Scope = Scope, Name = "MDF 18mm", CategoryId = MaterialCategory.Id, UnitId = Units[0].Id, CurrentCostYer = 7250, Revision = 1 },
            new Material { Id = Guid.NewGuid(), Scope = Scope, Name = "شريط حافة Edge Band", CategoryId = MaterialCategory.Id, UnitId = Units[1].Id, CurrentCostYer = 250, Revision = 1 }];
        Cost = new PartCost { TotalCostYer = 9450, Rows = [Row(0,"1.2",8700),Row(1,"3",750)] };
        var id = Guid.NewGuid();
        Parts.Add(new Part { Id = id, Scope = Scope, Name = "Wardrobe Side Panel", CategoryId = Category.Id, Description = "جانب خزانة تجريبي", Revision = 1,
            Composition = new CompositionReference { PartId = id, Scope = Scope, Revision = 1, SchemaVersion = 1 }, Cost = Cost });
    }
    private CostedUsage Row(int i,string quantity,long cost) => new()
    {
        Material = Materials[i], Unit = Units[i], CostUnit = Units[i], CostYer = cost,
        Usage = new PartUsage { MaterialId = Materials[i].Id, MaterialRevision = 1, UnitId = Units[i].Id, UnitRevision = 1, Quantity = quantity },
    };
    public PartProjection[] Projections() => Parts.Select(p => new PartProjection { Part = p, CurrentCost = Cost }).ToArray();
    public PartSnapshot Snapshot() => new(new PartCategories { Items = [Category] },Projections(),
        new MaterialSnapshot(new MaterialReferences { Categories = [MaterialCategory], Units = Units },Materials));
    public FakeEngine Engine()
    {
        var engine = new FakeEngine();
        engine.QueryHandler = query => Success(query.Kind switch
        {
            Query.PartCategoryListKind => QueryResult.ForPartCategories(new PartCategories { Items = [Category] }),
            Query.MaterialReferenceListKind => QueryResult.ForMaterialReferences(new MaterialReferences { Categories = [MaterialCategory], Units = Units }),
            Query.MaterialListKind => QueryResult.ForMaterials(new MaterialPage { Items = Materials }),
            Query.PartListKind => QueryResult.ForParts(new PartPage { Items = Projections().Where(p => query.AsPartList()!.Term.Length == 0 || p.Part.Name.Contains(query.AsPartList()!.Term,StringComparison.OrdinalIgnoreCase)).ToArray() }),
            Query.PartCostKind => QueryResult.ForPartCost(Cost),
            _ => throw new InvalidOperationException("Unexpected fixture query."),
        });
        engine.CommandHandler = command =>
        {
            if (command.AsPartSave() is { } input)
            {
                var id = input.Id ?? Guid.NewGuid();
                var part = new Part { Id = id, Scope = Scope, Name = input.Name, CategoryId = input.CategoryId, Description = input.Description,
                    Archived = input.Archived, Revision = (input.ExpectedRevision ?? 0) + 1, Cost = Cost,
                    Composition = new CompositionReference { PartId = id, Scope = Scope, Revision = (input.ExpectedRevision ?? 0) + 1, SchemaVersion = 1 } };
                Parts.RemoveAll(p => p.Id == id); Parts.Add(part);
            }
            return new CommandResponseEnvelope { RequestId = Guid.NewGuid(), CorrelationId = Guid.NewGuid(), Outcome = new CommandOutcome { Status = CommandOutcomeStatus.Succeeded, Payload = new CommandResult() } };
        };
        return engine;
    }
    public static QueryResponseEnvelope Success(QueryResult result) => new()
    { RequestId = Guid.NewGuid(), CorrelationId = Guid.NewGuid(), Outcome = new QueryOutcome { Status = CommandOutcomeStatus.Succeeded, Payload = result } };
}
