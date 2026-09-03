using System.Collections.ObjectModel;
using System.Globalization;
using System.Text;
using Eitmad.WindowsShell.Features.Operations;

namespace Eitmad.WindowsShell.Features.Orders;

/// <summary>Owns synthetic manager-order list, filters, and read-only detail state.</summary>
public sealed class OrdersViewModel : ObservableObject
{
    public const string AllStatuses = "كل الحالات";
    public const string NewStatus = "جديد";
    public const string InProductionStatus = "قيد الإنتاج";
    public const string ReadyStatus = "جاهز";
    public const string DeliveredStatus = "تم التسليم";
    public const string CancelledStatus = "ملغي";
    public const string AllDates = "كل التواريخ";
    public const string Today = "اليوم";
    public const string LastSevenDays = "آخر 7 أيام";
    public const string LastThirtyDays = "آخر 30 يوماً";

    private readonly List<OrderListItem> orders;
    private string searchText = string.Empty;
    private string selectedStatus = AllStatuses;
    private string selectedDate = AllDates;
    private OrderListItem? selectedOrder;

    public OrdersViewModel()
    {
        var today = DateOnly.FromDateTime(DateTime.Today);
        orders =
        [
            new(
                Guid.Parse("8f7e8f4d-6861-4c2c-9d03-3d2a8bd1e220"),
                "ORD-2026-0087",
                "شركة المها للتجهيزات",
                today,
                OrderStatus.New,
                35_000m,
                [
                    new("خزانة السكينة", "ثلاثة أبواب", "180 × 220 × 60 سم", "جوزي", "نحاسي", 2, 210_000m),
                    new("مكتب العمل الهادئ", "بوحدة أدراج", "140 × 75 × 70 سم", "بني", "معدن أسود", 1, 95_000m),
                ]),
            new(
                Guid.Parse("0d6dedb3-4682-4383-8947-bf626e6fa21c"),
                "ORD-2026-0084",
                "منزل عائلة الصبري",
                today.AddDays(-2),
                OrderStatus.InProduction,
                20_000m,
                [new("سرير وادي ظهر", "مزدوج", "200 × 180 سم", "أبيض مطفي", "مقبض قياسي", 2, 150_000m)]),
            new(
                Guid.Parse("6374de1e-f4db-49f5-a9d0-01b37a588280"),
                "ORD-2026-0079",
                "مؤسسة أروى للمفروشات",
                today.AddDays(-5),
                OrderStatus.Ready,
                0m,
                [new("طاولة ضيافة نُحاس", "طقم 6 مقاعد", "180 × 90 × 76 سم", "جوزي", "نحاسي", 3, 82_000m)]),
            new(
                Guid.Parse("7a9b1752-38a9-4528-a614-e70a49c88dd2"),
                "ORD-2026-0068",
                "استراحة وادي بنا",
                today.AddDays(-18),
                OrderStatus.Delivered,
                25_000m,
                [new("مقعد المجلس القديم", "ثلاثي", "220 × 85 × 78 سم", "بني", "بدون مقبض", 4, 67_000m)]),
            new(
                Guid.Parse("07803583-f847-48b4-803c-b4f234360c5a"),
                "ORD-2026-0051",
                "فندق سماء صنعاء",
                today.AddDays(-42),
                OrderStatus.Cancelled,
                40_000m,
                [new("مكتبة جدارية", "خمسة أقسام", "240 × 230 × 40 سم", "أبيض", "معدن أسود", 5, 240_000m)]),
        ];

        StatusOptions = [AllStatuses, NewStatus, InProductionStatus, ReadyStatus, DeliveredStatus, CancelledStatus];
        DateOptions = [AllDates, Today, LastSevenDays, LastThirtyDays];
        VisibleOrders = [];
        RefreshVisibleOrders();
    }

    public IReadOnlyList<string> StatusOptions { get; }

    public IReadOnlyList<string> DateOptions { get; }

    public ObservableCollection<OrderListItem> VisibleOrders { get; }

    public string SearchText
    {
        get => searchText;
        set
        {
            if (Set(ref searchText, value ?? string.Empty))
            {
                RefreshVisibleOrders();
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
                RefreshVisibleOrders();
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
                RefreshVisibleOrders();
            }
        }
    }

    public OrderListItem? SelectedOrder
    {
        get => selectedOrder;
        private set
        {
            if (Set(ref selectedOrder, value))
            {
                Raise(nameof(IsListVisible));
                Raise(nameof(IsDetailVisible));
            }
        }
    }

    public bool IsListVisible => SelectedOrder is null;

    public bool IsDetailVisible => SelectedOrder is not null;

    public bool HasNoVisibleOrders => VisibleOrders.Count == 0;

    public string VisibleCountLabel => VisibleOrders.Count switch
    {
        1 => "طلب واحد",
        2 => "طلبان",
        _ => $"{VisibleOrders.Count.ToString(CultureInfo.InvariantCulture)} طلبات",
    };

    public void OpenOrder(OrderListItem order) => SelectedOrder = order;

    public void CloseOrder() => SelectedOrder = null;

    private void RefreshVisibleOrders()
    {
        var normalizedSearch = NormalizeArabic(SearchText.Trim());
        var today = DateOnly.FromDateTime(DateTime.Today);
        var filtered = orders.Where(order =>
            MatchesSearch(order, normalizedSearch)
            && MatchesStatus(order)
            && MatchesDate(order, today));

        VisibleOrders.Clear();
        foreach (var order in filtered)
        {
            VisibleOrders.Add(order);
        }

        Raise(nameof(HasNoVisibleOrders));
        Raise(nameof(VisibleCountLabel));
    }

    private static bool MatchesSearch(OrderListItem order, string normalizedSearch) =>
        string.IsNullOrEmpty(normalizedSearch)
        || NormalizeArabic(order.Number).Contains(normalizedSearch, StringComparison.OrdinalIgnoreCase)
        || NormalizeArabic(order.Customer).Contains(normalizedSearch, StringComparison.OrdinalIgnoreCase);

    private bool MatchesStatus(OrderListItem order) => SelectedStatus switch
    {
        AllStatuses => true,
        NewStatus => order.Status == OrderStatus.New,
        InProductionStatus => order.Status == OrderStatus.InProduction,
        ReadyStatus => order.Status == OrderStatus.Ready,
        DeliveredStatus => order.Status == OrderStatus.Delivered,
        CancelledStatus => order.Status == OrderStatus.Cancelled,
        _ => false,
    };

    private bool MatchesDate(OrderListItem order, DateOnly today) => SelectedDate switch
    {
        AllDates => true,
        Today => order.Date == today,
        LastSevenDays => order.Date >= today.AddDays(-6),
        LastThirtyDays => order.Date >= today.AddDays(-29),
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
