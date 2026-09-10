using System.Globalization;
using Eitmad.WindowsShell.Controls;

namespace Eitmad.WindowsShell.Features.Operations;

/// <summary>Existing synthetic dashboard preview rows, with typed table sort values.</summary>
public sealed record DashboardQuotationPreview(string Number, string Customer, string Product,
    string StatusLabel, PresentationTone Tone, decimal Total, DateOnly Date)
{
    public string AmountText => Total.ToString("N0", CultureInfo.InvariantCulture);
    public string DateLabel => Date.ToString("yyyy/MM/dd", CultureInfo.InvariantCulture);
    public static IReadOnlyList<DashboardQuotationPreview> Rows { get; } =
    [
        new("Q-2025-0518", "شركة ديكور المنزل", "مطبخ مودرن خشب بلوط", "جديد", PresentationTone.Information, 24560m, new DateOnly(2025, 5, 18)),
        new("Q-2025-0517", "أبواب الخليج", "غرفة نوم كلاسيك", "قيد المراجعة", PresentationTone.Warning, 18750m, new DateOnly(2025, 5, 18)),
        new("Q-2025-0516", "مؤسسة الواجهة", "دولاب ملابس 6 أبواب", "بانتظار الموافقة", PresentationTone.Warning, 32900m, new DateOnly(2025, 5, 17)),
        new("Q-2025-0515", "أحمد السليم", "وحدة تلفاز حديثة", "مكتمل", PresentationTone.Success, 15300m, new DateOnly(2025, 5, 17)),
        new("Q-2025-0514", "نورة العتيبي", "تسريحة مع مرآة", "مرفوض", PresentationTone.Danger, 9800m, new DateOnly(2025, 5, 16)),
    ];
}
