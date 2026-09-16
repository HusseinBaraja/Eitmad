using System.Collections.ObjectModel;
using System.Globalization;

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

    private ObservableCollection<QuotationListItem> quotations;
    private string searchText = string.Empty;
    private string selectedStatus = AllStatuses;
    private string selectedDate = AllDates;
    private QuotationListItem? selectedQuotation;

    public QuotationsViewModel(bool isReceptionist = false)
    {
        IsReceptionist = isReceptionist;
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

        if (isReceptionist)
            quotations.Add(new(Guid.Parse("7d438102-f09d-4e1a-b0e5-d4f72d540143"), "QT-2026-0143", "عميل تجريبي للموافقة", today,
                QuotationStatus.WaitingApproval, 12_000m,
                [new("طاولة ضيافة", "طقم 6 مقاعد", "جوزي", "نحاسي", 1, 100_000m)], requiresDiscountApproval: true, phone: "000000043"));
        StatusOptions = isReceptionist
            ? [AllStatuses, DraftStatus, ActiveStatus, "بانتظار الموافقة", ConvertedStatus, ClosedStatus]
            : [AllStatuses, "بانتظار الموافقة", DraftStatus, ActiveStatus, ConvertedStatus, ClosedStatus];
        DateOptions = [AllDates, Today, LastSevenDays, LastThirtyDays];
        VisibleQuotations = [];
        RefreshVisibleQuotations();
    }

    public ObservableCollection<QuotationListItem> PreviewQuotations => quotations;
    public void UsePreviewQuotations(ObservableCollection<QuotationListItem> items)
    {
        quotations = items;
        quotations.CollectionChanged += (_, e) =>
        {
            if (SelectedQuotation is { } selected) SelectedQuotation = quotations.FirstOrDefault(item => item.Id == selected.Id);
            if (e.NewItems is not null) foreach (QuotationListItem item in e.NewItems) ObserveDecision(item);
            RefreshVisibleQuotations();
        };
        foreach (var item in quotations) ObserveDecision(item);
        RefreshVisibleQuotations();
    }
    private void ObserveDecision(QuotationListItem item) =>
        System.ComponentModel.PropertyChangedEventManager.AddHandler(item, (_, _) => { Raise(nameof(ShowManagerApproval)); RefreshVisibleQuotations(); }, nameof(QuotationListItem.ApprovalDecision));
    private bool approvalsOnly;
    public bool ApprovalsOnly { get => approvalsOnly; set { if (Set(ref approvalsOnly, value)) { Raise(nameof(ListTitle)); Raise(nameof(EmptyTitle)); Raise(nameof(EmptyDescription)); RefreshVisibleQuotations(); } } }
    public string EmptyTitle => ApprovalsOnly ? "لا توجد طلبات خصم معلقة" : "لا توجد عروض أسعار مطابقة";
    public string EmptyDescription => ApprovalsOnly ? "تظهر هنا طلبات الاستقبال المؤقتة في هذه الجلسة. راجع عوامل التصفية أيضاً." : "غيّر البحث أو عوامل التصفية.";
    public string ListTitle => ApprovalsOnly ? "موافقات الخصم" : "عروض الأسعار";
    public void OpenApprovals()
    {
        CloseQuotation(); SearchText = ""; SelectedDate = AllDates; SelectedStatus = AllStatuses; ApprovalsOnly = true;
    }

    public string ListSubtitle => IsReceptionist ? "بيانات تجريبية للمعاينة فقط" : "معاينة مؤقتة — عروض الاستقبال وطلبات الخصم في هذه الجلسة فقط";
    public bool IsReceptionist { get; }
    public bool ShowManagerApproval => !IsReceptionist && SelectedQuotation?.HasPendingDiscountApproval == true;
    public string SearchName => IsReceptionist ? "البحث برقم عرض السعر أو العميل أو رقم الهاتف" : "البحث برقم عرض السعر أو العميل";

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
                Raise(nameof(ShowManagerApproval));
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

    public void ApproveDiscount() { if (IsReceptionist) return; SelectedQuotation?.DecideDiscount(DiscountApprovalDecision.Approved); Raise(nameof(ShowManagerApproval)); RefreshVisibleQuotations(); }

    public void RejectDiscount() { if (IsReceptionist) return; SelectedQuotation?.DecideDiscount(DiscountApprovalDecision.Rejected); Raise(nameof(ShowManagerApproval)); RefreshVisibleQuotations(); }

    private void RefreshVisibleQuotations()
    {
        var normalizedSearch = PreviewText.NormalizeSearch(SearchText.Trim());
        var today = DateOnly.FromDateTime(DateTime.Today);
        var filtered = quotations.Where(quotation =>
            MatchesSearch(quotation, normalizedSearch)
            && (!ApprovalsOnly || quotation.HasPendingDiscountApproval)
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

    private bool MatchesSearch(QuotationListItem quotation, string normalizedSearch) =>
        string.IsNullOrEmpty(normalizedSearch)
        || PreviewText.NormalizeSearch(quotation.Number).Contains(normalizedSearch, StringComparison.OrdinalIgnoreCase)
        || PreviewText.NormalizeSearch(quotation.Customer).Contains(normalizedSearch, StringComparison.OrdinalIgnoreCase)
        || (IsReceptionist && PreviewText.NormalizeNumericInput(quotation.Phone).Contains(PreviewText.NormalizeNumericInput(normalizedSearch), StringComparison.OrdinalIgnoreCase));

    private bool MatchesStatus(QuotationListItem quotation) => SelectedStatus switch
    {
        AllStatuses => true,
        DraftStatus => quotation.Status == QuotationStatus.Draft,
        ActiveStatus => quotation.Status == QuotationStatus.Active,
        "بانتظار الموافقة" => quotation.HasPendingDiscountApproval,
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
}
