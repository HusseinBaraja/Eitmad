using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.RawMaterials;

namespace Eitmad.WindowsShell.Tests.RawMaterials;

internal static class MaterialFixtures
{
    public static MaterialSnapshot Snapshot()
    {
        var scope = new ScopeRef { Kind = "organization", Id = Guid.NewGuid() };
        var categories = new[] { "ألواح خشبية", "أخشاب طبيعية", "أقمشة ومفروشات" }
            .Select(name => new MaterialCategory { Id = Guid.NewGuid(), Scope = scope, Name = name, Revision = 1 }).ToArray();
        var units = new[] { "لوح", "متر", "كيلوجرام", "قطعة" }
            .Select(name => new MaterialUnit { Id = Guid.NewGuid(), Scope = scope, Name = name, Symbol = name, Numerator = 1, Denominator = 1, Revision = 1 }).ToArray();
        Material Row(string name, int category, int unit, long cost, bool archived = false) => new()
        {
            Id = Guid.NewGuid(), Scope = scope, Name = name, CategoryId = categories[category].Id,
            UnitId = units[unit].Id, CurrentCostYer = cost, Archived = archived, Revision = 1,
        };
        return new(new MaterialReferences { Categories = categories, Units = units },
        [
            Row("لوح MDF سماكة 18 مم", 0, 0, 25_000), Row("خشب زان مجفف", 1, 1, 8_000),
            Row("قماش كتان بيج", 2, 1, 3_500), Row("خشب سويدي مقاس 2×4", 1, 1, 5_200, true),
        ]);
    }

    public static RawMaterialsViewModel Model()
    {
        var model = new RawMaterialsViewModel(); var data = Snapshot();
        model.ApplyDurableData(data.References, data.Materials);
        return model;
    }
}
