using System.Collections.ObjectModel;
using System.Globalization;
using System.Text;
using Eitmad.WindowsShell.Features.Operations;

namespace Eitmad.WindowsShell.Features.Quotations;

/// <summary>Owns synthetic list, detail, and discount-approval state for the manager preview.</summary>
public sealed class QuotationsViewModel : ObservableObject
{
    public const string AllStatuses = "كل الحالات";
    public const string DraftStatus = "مسودة";
    public const string ActiveStatus = "نشط";
    public const string ConvertedStatus = "محوّل";
    public const string ClosedStatus = "ملغي / منتهي";
    public const string AllDates = "كل التواريخ";
    public const string Today = "اليوم";
    public const string LastSevenDays = "آخر 7 أيام";
    public const string LastThirtyDays = "آخر 30 يوماً";

    private readonly List<QuotationListItem> quotations;
    private string searchText = string.Empty;
    private string selectedStatus = AllStatuses;
    private string selectedDate = AllDates;
    private QuotationListItem? selectedQuotation;

    public QuotationsViewModel()
    {
        var today = DateOnly.FromDateTime(DateTime.Today);
        quotations =
        [
            new(
                Guid.Parse("f8240507-6295-4550-9159-1268622c42f4"),
                "QT-2026-0142",
                "شركة المها للتجهيزات",
                today,
                QuotationStatus.Draft,
                72_000m,
                [
                    new("خزانة السكينة", "عرض 180 سم", "جوزي", "نحاسي", 2, 200_000m),
                    new("مكتب العمل الهادئ", "عرض 140 سم", "بني", "معدن أسود", 1, 80_000m),
                ],
                requiresDiscountApproval: true),
            new(
                Guid.Parse("a774247c-55f8-4881-9974-77fc0716a50f"),
                "QT-2026-0141",
                "منزل عائلة الصبري",
                today.AddDays(-2),
                QuotationStatus.Active,
                15_000m,
                [new("سرير وادي ظهر", "مقاس مزدوج", "أبيض", "مقبض قياسي", 2, 145_000m)]),
            new(
                Guid.Parse("d08c9040-26fc-4864-8bb6-81c35565618d"),
                "QT-2026-0138",
                "مؤسسة أروى للمفروشات",
                today.AddDays(-5),
                QuotationStatus.Converted,
                0m,
                [new("طاولة ضيافة نُحاس", "طقم 6 مقاعد", "جوزي", "نحاسي", 3, 78_000m)]),
            new(
                Guid.Parse("ae36a2da-ff40-4455-8fb9-a4ebea36bd17"),
                "QT-2026-0129",
                "استراحة وادي بنا",
                today.AddDays(-16),
                QuotationStatus.Cancelled,
                25_000m,
                [new("مقعد المجلس القديم", "ثلاثي", "بني", "مقبض قياسي", 4, 62_000m)]),
            new(
                Guid.Parse("95f4e255-5b3e-4db9-9fc3-cc97d6f0e2ef"),
                "QT-2026-0117",
                "فندق سماء صنعاء",
                today.AddDays(-37),
                QuotationStatus.Expired,
                40_000m,
                [new("مكتبة جدارية", "عرض 240 سم", "أبيض", "معدن أسود", 5, 235_000m)]),
        ];

        StatusOptions = [AllStatuses, DraftStatus, ActiveStatus, ConvertedStatus, ClosedStatus];
        DateOptions = [AllDates, Today, LastSevenDays, LastThirtyDays];
        VisibleQuotations = [];
        RefreshVisibleQuotations();
    }

    public IReadOnlyList<string> StatusOptions { get; }

    public IReadOnlyList<string> DateOptions { get; }

    public ObservableCollection<QuotationListItem> VisibleQuotations { get; }

    public string SearchText
    {
        get => searchText;
        set
        {
            if (Set(ref searchText, value ?? string.Empty))
            {
                RefreshVisibleQuotations();
            }
        }
    }

    public string SelectedStatus
    {
        get => selectedStatus;
        set
        {
            if (Set(ref selectedStatus, value ?? AllStatuses))
            {
                RefreshVisibleQuotations();
            }
        }
    }

    public string SelectedDate
    {
        get => selectedDate;
        set
        {
            if (Set(ref selectedDate, value ?? AllDates))
            {
                RefreshVisibleQuotations();
            }
        }
    }

    public QuotationListItem? SelectedQuotation
    {
        get => selectedQuotation;
        private set
        {
            if (Set(ref selectedQuotation, value))
            {
                Raise(nameof(IsListVisible));
                Raise(nameof(IsDetailVisible));
            }
        }
    }

    public bool IsListVisible => SelectedQuotation is null;

    public bool IsDetailVisible => SelectedQuotation is not null;

    public bool HasNoVisibleQuotations => VisibleQuotations.Count == 0;

    public string VisibleCountLabel => VisibleQuotations.Count switch
    {
        1 => "عرض سعر واحد",
        2 => "عرضا سعر",
        _ => $"{VisibleQuotations.Count.ToString(CultureInfo.InvariantCulture)} عروض أسعار",
    };

    public void OpenQuotation(QuotationListItem quotation) => SelectedQuotation = quotation;

    public void CloseQuotation() => SelectedQuotation = null;

    public void ApproveDiscount() => SelectedQuotation?.DecideDiscount(DiscountApprovalDecision.Approved);

    public void RejectDiscount() => SelectedQuotation?.DecideDiscount(DiscountApprovalDecision.Rejected);

    private void RefreshVisibleQuotations()
    {
        var normalizedSearch = NormalizeArabic(SearchText.Trim());
        var today = DateOnly.FromDateTime(DateTime.Today);
        var filtered = quotations.Where(quotation =>
            MatchesSearch(quotation, normalizedSearch)
            && MatchesStatus(quotation)
            && MatchesDate(quotation, today));

        VisibleQuotations.Clear();
        foreach (var quotation in filtered)
        {
            VisibleQuotations.Add(quotation);
        }

        Raise(nameof(HasNoVisibleQuotations));
        Raise(nameof(VisibleCountLabel));
    }

    private static bool MatchesSearch(QuotationListItem quotation, string normalizedSearch) =>
        string.IsNullOrEmpty(normalizedSearch)
        || NormalizeArabic(quotation.Number).Contains(normalizedSearch, StringComparison.OrdinalIgnoreCase)
        || NormalizeArabic(quotation.Customer).Contains(normalizedSearch, StringComparison.OrdinalIgnoreCase);

    private bool MatchesStatus(QuotationListItem quotation) => SelectedStatus switch
    {
        AllStatuses => true,
        DraftStatus => quotation.Status == QuotationStatus.Draft,
        ActiveStatus => quotation.Status == QuotationStatus.Active,
        ConvertedStatus => quotation.Status == QuotationStatus.Converted,
        ClosedStatus => quotation.Status is QuotationStatus.Cancelled or QuotationStatus.Expired,
        _ => false,
    };

    private bool MatchesDate(QuotationListItem quotation, DateOnly today) => SelectedDate switch
    {
        AllDates => true,
        Today => quotation.Date == today,
        LastSevenDays => quotation.Date >= today.AddDays(-6),
        LastThirtyDays => quotation.Date >= today.AddDays(-29),
        _ => false,
    };

    private static string NormalizeArabic(string value)
    {
        var normalized = new StringBuilder(value.Length);
        foreach (var character in value.Normalize(NormalizationForm.FormD))
        {
            if (CharUnicodeInfo.GetUnicodeCategory(character) == UnicodeCategory.NonSpacingMark || character == '\u0640')
            {
                continue;
            }

            normalized.Append(character switch
            {
                '\u0622' or '\u0623' or '\u0625' => '\u0627',
                '\u0649' => '\u064A',
                '\u0629' => '\u0647',
                _ => character,
            });
        }

        return normalized.ToString().Normalize(NormalizationForm.FormC);
    }
}
