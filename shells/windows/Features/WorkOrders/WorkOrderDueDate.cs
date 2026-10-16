namespace Eitmad.WindowsShell.Features.WorkOrders;

internal static class WorkOrderDueDate
{
    // DatePicker values are Aden calendar dates, independent of the host time zone or DateTime.Kind.
    internal static long EndOfAdenDay(DateTime date) => new DateTimeOffset(
        DateTime.SpecifyKind(date.Date, DateTimeKind.Unspecified)
            .AddTicks(TimeSpan.TicksPerDay - TimeSpan.TicksPerMillisecond),
        TimeSpan.FromHours(3)).ToUnixTimeMilliseconds();
}
