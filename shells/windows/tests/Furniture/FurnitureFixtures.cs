using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.Furniture;
using Eitmad.WindowsShell.Tests.Parts;
using Eitmad.WindowsShell.Tests.TestDoubles;
using Definition = Eitmad.Contracts.Furniture;

namespace Eitmad.WindowsShell.Tests.Furniture;

// Fixed synthetic Rust projections. Domain validation and arithmetic are tested in Rust.
internal sealed class FurnitureFixtures
{
    public PartFixtures Parts { get; } = new();
    public FurnitureCategory Category { get; }
    public List<Definition> Definitions { get; } = [];
    public FurnitureReview Review { get; } = new() { PartsCostYer = 18900, RowCostsYer = [18900], MarginsYer = [181100] };
    /// <summary>Creates a separate synthetic Furniture category for test projections.</summary>
    public FurnitureFixtures() => Category = new() { Id = Guid.NewGuid(), Scope = Parts.Scope, Name = "غرف النوم", Revision = 1 };
    /// <summary>Combines fixed scoped definitions and Part references for presentation tests.</summary>
    public FurnitureSnapshot Snapshot() => new(new FurnitureCategories { Items = [Category] }, Definitions, true, true, Parts.Parts, [Parts.Category], Parts.Parts);
    /// <summary>Creates a presentation model with explicit synthetic authority data.</summary>
    public FurnitureViewModel Model() { var model = new FurnitureViewModel(); model.ApplyDurableData(Snapshot()); return model; }
    /// <summary>Creates one fixed saved definition with immutable Part references.</summary>
    public Definition Seed()
    {
        var value = new Definition
        {
            Id = Guid.NewGuid(),
            Scope = Parts.Scope,
            Name = "خزانة السكينة",
            CategoryId = Category.Id,
            CategoryName = Category.Name,
            Description = "تعريف تجريبي",
            Notes = "ملاحظة داخلية",
            Revision = 1,
            State = FurnitureState.Active,
            PartsCostYer = 18900,
            Parts = [new FurniturePart { Reference = Parts.Parts[0].Composition, Quantity = 2 }],
            Variants = [new Eitmad.Contracts.FurnitureVariant { Id = Guid.NewGuid(), Name = "صغير", Dimensions = new FurnitureDimensions { WidthMm = 1200, HeightMm = 2000, DepthMm = 550 }, SellingPriceYer = 200000, ColorIds = [], HandleIds = [] }],
            Colors = [new FurnitureOption { Id = Guid.NewGuid(), Name = "أبيض", Visual = "#FFFFFF" }],
            Handles = [new FurnitureOption { Id = Guid.NewGuid(), Name = "قياسي", Visual = "Standard" }],
        };
        Definitions.Add(value); return value;
    }
    /// <summary>Creates an explicit sales-only fixture independent of live manager definitions.</summary>
    public static FurnitureViewModel SalesModel() { var model = new FurnitureViewModel(); model.ApplyDurableData(SalesSnapshot()); model.FixtureSalesCatalog = true; return model; }
    /// <summary>Builds synthetic published catalog items for reception tests.</summary>
    public static FurnitureSnapshot SalesSnapshot()
    {
        var f = new FurnitureFixtures(); var p = f.Seed(); p.Id = Guid.Parse("3fc526b4-2b79-45fd-984c-49258f55951d");
        p.Variants = new[] { ("صغير", 1200L, 2000L, 550L, 200000L), ("متوسط", 1600L, 2100L, 550L, 245000L), ("كبير", 2000L, 2200L, 600L, 300000L) }.Select(v => new Eitmad.Contracts.FurnitureVariant { Id = Guid.NewGuid(), Name = v.Item1, Dimensions = new FurnitureDimensions { WidthMm = v.Item2, HeightMm = v.Item3, DepthMm = v.Item4 }, SellingPriceYer = v.Item5, ColorIds = [], HandleIds = [] }).ToArray();
        p.Colors = [new FurnitureOption { Id = Guid.NewGuid(), Name = "أبيض", Visual = "#F7F4EF" }, new FurnitureOption { Id = Guid.NewGuid(), Name = "بني", Visual = "#8B5A3C", Archived = true }, new FurnitureOption { Id = Guid.NewGuid(), Name = "جوزي", Visual = "#4F2C1D", PriceAdjustmentYer = 10000 }];
        p.Handles = [new FurnitureOption { Id = Guid.NewGuid(), Name = "مقبض قياسي", Visual = "Standard" }, new FurnitureOption { Id = Guid.NewGuid(), Name = "معدن أسود", Visual = "BlackMetal", PriceAdjustmentYer = 3000 }, new FurnitureOption { Id = Guid.NewGuid(), Name = "نحاسي", Visual = "Brass", PriceAdjustmentYer = 5000, Archived = true }];
        var definitions = new List<Definition> { p };
        var categories = new List<FurnitureCategory> { f.Category };
        foreach (var (id, name, categoryName, price) in new[] {
            ("0c2ceef3-620b-4d23-a81e-c955572ef440","سرير وادي ظهر","غرف النوم",145000L),
            ("4246f76c-5476-42cb-9ee6-07694245319f","طاولة ضيافة نُحاس","غرف المعيشة",78000L),
            ("e96abff0-5078-464a-b10f-f73563c311d3","مكتب العمل الهادئ","المكاتب",115000L),
        })
        {
            var copy = System.Text.Json.JsonSerializer.Deserialize<Definition>(System.Text.Json.JsonSerializer.Serialize(p))!;
            var category = categories.FirstOrDefault(c => c.Name == categoryName);
            if (category is null) { category = new FurnitureCategory { Id = Guid.NewGuid(), Scope = p.Scope, Name = categoryName, Revision = 1 }; categories.Add(category); }
            copy.Id = Guid.Parse(id); copy.Name = name; copy.CategoryId = category.Id; copy.CategoryName = category.Name;
            foreach (var v in copy.Variants) v.SellingPriceYer = price;
            definitions.Add(copy);
        }
        return new FurnitureSnapshot(new FurnitureCategories { Items = categories.ToArray() }, definitions, true, true, f.Parts.Parts, [f.Parts.Category], f.Parts.Parts);
    }
    /// <summary>Returns typed fixed queries and confirmed synthetic saves for Furniture tests.</summary>
    public FakeEngine Engine()
    {
        var engine = Parts.Engine(); var partQueries = engine.QueryHandler!;
        engine.QueryHandler = query => query.Kind switch
        {
            Query.FurnitureCategoryListKind => PartFixtures.Success(QueryResult.ForFurnitureCategories(new FurnitureCategories { Items = [Category] })),
            Query.FurnitureListKind => PartFixtures.Success(QueryResult.ForFurnitures(new FurniturePage { Items = Definitions.ToArray(), CanManage = true, CanReadCosts = true })),
            Query.FurnitureReviewKind => PartFixtures.Success(QueryResult.ForFurnitureReview(Review)),
            Query.PartCompositionGetKind => PartFixtures.Success(QueryResult.ForPartComposition(Parts.Parts[0])),
            _ => partQueries(query),
        };
        engine.CommandHandler = command =>
        {
            if (command.AsFurnitureSave() is { } input)
            {
                var value = new Definition
                {
                    Id = input.Id ?? Guid.NewGuid(),
                    Scope = Parts.Scope,
                    Name = input.Name,
                    CategoryId = input.CategoryId,
                    CategoryName = Category.Name,
                    Description = input.Description,
                    Notes = input.Notes,
                    Revision = (input.ExpectedRevision ?? 0) + 1,
                    State = input.State,
                    Parts = input.Parts,
                    Variants = input.Variants,
                    Colors = input.Colors,
                    Handles = input.Handles,
                    PartsCostYer = 18900,
                };
                Definitions.RemoveAll(p => p.Id == value.Id); Definitions.Add(value);
                return Success(Result(PurpleKind.FurnitureSaved, value));
            }
            if (command.AsFurnitureCategorySave() is { } category) { Category.Name = category.Name; return Success(Result(PurpleKind.FurnitureCategorySaved, Category)); }
            throw new InvalidOperationException("Unexpected fixture command.");
        };
        return engine;
    }
    /// <summary>Serializes a fixed test value into its typed command result.</summary>
    private static CommandResult Result<T>(PurpleKind kind, T value) =>
        System.Text.Json.JsonSerializer.Deserialize<CommandResult>(
            System.Text.Json.JsonSerializer.Serialize(new { kind, payload = value }))!;
    /// <summary>Wraps a fixed command payload in a confirmed synthetic outcome.</summary>
    public static CommandResponseEnvelope Success(CommandResult value) => new() { RequestId = Guid.NewGuid(), CorrelationId = Guid.NewGuid(), Outcome = new CommandOutcome { Status = CommandOutcomeStatus.Succeeded, Payload = value } };
    /// <summary>Wraps a stable error identifier in a failed synthetic outcome.</summary>
    public static CommandResponseEnvelope Failure(string code) => new() { RequestId = Guid.NewGuid(), CorrelationId = Guid.NewGuid(), Outcome = new CommandOutcome { Status = CommandOutcomeStatus.Failed, Payload = new CommandResult { Code = code } } };
}
