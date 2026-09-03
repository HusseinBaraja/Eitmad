using System.Collections.ObjectModel;
using System.Globalization;
using System.Text;
using Eitmad.WindowsShell.Features.Operations;

namespace Eitmad.WindowsShell.Features.WorkOrders;

/// <summary>Owns synthetic work-order list filters, detail selection, and local status preview.</summary>
public sealed class WorkOrdersViewModel : ObservableObject
{
    public const string AllStatuses = "كل الحالات";
    public const string NewStatus = "جديد";
    public const string InProgressStatus = "قيد التنفيذ";
    public const string CompletedStatus = "مكتمل";
    public const string CancelledStatus = "ملغي";
    public const string AllDueDates = "كل المواعيد";
    public const string Overdue = "متأخر";
    public const string DueToday = "اليوم";
    public const string NextSevenDays = "خلال 7 أيام";

    private readonly List<WorkOrderListItem> workOrders;
    private string searchText = string.Empty;
    private string selectedStatus = AllStatuses;
    private string selectedDueDate = AllDueDates;
    private WorkOrderListItem? selectedWorkOrder;
    private string feedbackMessage = string.Empty;

    public WorkOrdersViewModel()
    {
        var today = DateOnly.FromDateTime(DateTime.Today);
        workOrders =
        [
            new(
                Guid.Parse("c7977750-270a-4d48-981d-6152329f79e6"),
                "WO-024",
                "#1058",
                "أحمد علي",
                "محمد",
                today.AddDays(9),
                WorkOrderStatus.New,
                [new("خزانة كبيرة", "كبير", "200 × 220 × 60 سم", "جوزي", "مقبض نحاسي", 1, FurnitureIllustration.Wardrobe)],
                [new("جانب الخزانة", 2), new("باب الخزانة", 4), new("رف داخلي", 5), new("قاعدة الخزانة", 1)],
                "يُرجى مطابقة اتجاه عروق الخشب بين الأبواب. يفضّل العميل أن تكون المقابض على ارتفاع واحد."),
            new(
                Guid.Parse("7cb4ff41-58dc-413d-b665-50847935dc2b"),
                "WO-023",
                "#1056",
                "هدى صالح",
                "علي",
                today.AddDays(4),
                WorkOrderStatus.InProgress,
                [new("طاولة طعام", "ستة مقاعد", "180 × 90 × 76 سم", "بلوط طبيعي", "بدون مقبض", 1, FurnitureIllustration.Table)],
                [new("سطح الطاولة", 1), new("رجل الطاولة", 4), new("دعامة سفلية", 2)],
                "الحواف مستديرة وآمنة للأطفال حسب اتفاق الطلب."),
            new(
                Guid.Parse("db8884cd-f733-47f4-b65d-c0617ef92b18"),
                "WO-021",
                "#1049",
                "سمير القدسي",
                "يوسف",
                today.AddDays(-2),
                WorkOrderStatus.Completed,
                [new("سرير مزدوج", "ملكي", "200 × 180 سم", "أبيض مطفي", "بدون مقبض", 1, FurnitureIllustration.Bed)],
                [new("لوح الرأس", 1), new("جانب السرير", 2), new("قاعدة شرائح", 1)],
                "ثبت لوح الرأس دون فراغ ظاهر عند الحافة."),
            new(
                Guid.Parse("501e3491-7a34-4811-8e1a-5942e8e250ea"),
                "WO-019",
                "#1043",
                "مها النجار",
                "غير مسند",
                today.AddDays(15),
                WorkOrderStatus.Cancelled,
                [new("خزانة أطفال", "ثلاثة أبواب", "160 × 190 × 55 سم", "أبيض", "مقبض خشبي", 1, FurnitureIllustration.Wardrobe)],
                [new("جانب الخزانة", 2), new("باب الخزانة", 3), new("رف داخلي", 4)],
                "أُلغي أمر العمل قبل بدء التصنيع."),
            new(
                Guid.Parse("66825c39-14f1-411c-9086-e6035a23b994"),
                "WO-018",
                "#1040",
                "منزل آل سلام",
                "خالد",
                today.AddDays(-1),
                WorkOrderStatus.New,
                [new("مكتبة جدارية", "خمسة أقسام", "240 × 230 × 40 سم", "بني داكن", "مقبض أسود", 1, FurnitureIllustration.Wardrobe)],
                [new("جانب المكتبة", 2), new("رف طويل", 10), new("باب سفلي", 4)],
                "راجع فتحة الكهرباء في القسم الأوسط قبل القص."),
        ];

        StatusOptions = [AllStatuses, NewStatus, InProgressStatus, CompletedStatus, CancelledStatus];
        DueDateOptions = [AllDueDates, Overdue, DueToday, NextSevenDays];
        VisibleWorkOrders = [];
        RefreshVisibleWorkOrders();
    }

    public IReadOnlyList<string> StatusOptions { get; }

    public IReadOnlyList<string> DueDateOptions { get; }

    public ObservableCollection<WorkOrderListItem> VisibleWorkOrders { get; }

    public string SearchText
    {
        get => searchText;
        set
        {
            if (Set(ref searchText, value ?? string.Empty))
            {
                RefreshVisibleWorkOrders();
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
                RefreshVisibleWorkOrders();
            }
        }
    }

    public string SelectedDueDate
    {
        get => selectedDueDate;
        set
        {
            if (Set(ref selectedDueDate, value ?? AllDueDates))
            {
                RefreshVisibleWorkOrders();
            }
        }
    }

    public WorkOrderListItem? SelectedWorkOrder
    {
        get => selectedWorkOrder;
        private set
        {
            if (Set(ref selectedWorkOrder, value))
            {
                Raise(nameof(IsListVisible));
                Raise(nameof(IsDetailVisible));
            }
        }
    }

    public string FeedbackMessage
    {
        get => feedbackMessage;
        private set
        {
            if (Set(ref feedbackMessage, value))
            {
                Raise(nameof(HasFeedback));
            }
        }
    }

    public bool IsListVisible => SelectedWorkOrder is null;

    public bool IsDetailVisible => SelectedWorkOrder is not null;

    public bool HasNoVisibleWorkOrders => VisibleWorkOrders.Count == 0;

    public bool HasFeedback => !string.IsNullOrEmpty(FeedbackMessage);

    public string VisibleCountLabel => VisibleWorkOrders.Count switch
    {
        1 => "أمر عمل واحد",
        2 => "أمرا عمل",
        _ => $"{VisibleWorkOrders.Count.ToString(CultureInfo.InvariantCulture)} أوامر عمل",
    };

    public void OpenWorkOrder(WorkOrderListItem workOrder)
    {
        FeedbackMessage = string.Empty;
        SelectedWorkOrder = workOrder;
    }

    public void CloseWorkOrder()
    {
        FeedbackMessage = string.Empty;
        SelectedWorkOrder = null;
    }

    public bool AdvanceSelectedStatus()
    {
        if (SelectedWorkOrder is null || !SelectedWorkOrder.AdvanceStatus())
        {
            return false;
        }

        FeedbackMessage = $"تغيّرت الحالة إلى «{SelectedWorkOrder.StatusLabel}» في المعاينة المحلية فقط.";
        RefreshVisibleWorkOrders();
        return true;
    }

    public void ClearFeedback() => FeedbackMessage = string.Empty;

    private void RefreshVisibleWorkOrders()
    {
        var normalizedSearch = NormalizeArabic(SearchText.Trim());
        var today = DateOnly.FromDateTime(DateTime.Today);
        var filtered = workOrders.Where(workOrder =>
            MatchesSearch(workOrder, normalizedSearch)
            && MatchesStatus(workOrder)
            && MatchesDueDate(workOrder, today));

        VisibleWorkOrders.Clear();
        foreach (var workOrder in filtered)
        {
            VisibleWorkOrders.Add(workOrder);
        }

        Raise(nameof(HasNoVisibleWorkOrders));
        Raise(nameof(VisibleCountLabel));
    }

    private static bool MatchesSearch(WorkOrderListItem workOrder, string normalizedSearch) =>
        string.IsNullOrEmpty(normalizedSearch)
        || NormalizeArabic(workOrder.Number).Contains(normalizedSearch, StringComparison.OrdinalIgnoreCase)
        || NormalizeArabic(workOrder.OrderNumber).Contains(normalizedSearch, StringComparison.OrdinalIgnoreCase)
        || NormalizeArabic(workOrder.Customer).Contains(normalizedSearch, StringComparison.OrdinalIgnoreCase)
        || NormalizeArabic(workOrder.AssignedTo).Contains(normalizedSearch, StringComparison.OrdinalIgnoreCase)
        || workOrder.Furniture.Any(item => NormalizeArabic(item.Name).Contains(normalizedSearch, StringComparison.OrdinalIgnoreCase));

    private bool MatchesStatus(WorkOrderListItem workOrder) => SelectedStatus switch
    {
        AllStatuses => true,
        NewStatus => workOrder.Status == WorkOrderStatus.New,
        InProgressStatus => workOrder.Status == WorkOrderStatus.InProgress,
        CompletedStatus => workOrder.Status == WorkOrderStatus.Completed,
        CancelledStatus => workOrder.Status == WorkOrderStatus.Cancelled,
        _ => false,
    };

    private bool MatchesDueDate(WorkOrderListItem workOrder, DateOnly today) => SelectedDueDate switch
    {
        AllDueDates => true,
        Overdue => workOrder.DueDate < today && workOrder.Status is not WorkOrderStatus.Completed and not WorkOrderStatus.Cancelled,
        DueToday => workOrder.DueDate == today,
        NextSevenDays => workOrder.DueDate >= today && workOrder.DueDate <= today.AddDays(7),
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
