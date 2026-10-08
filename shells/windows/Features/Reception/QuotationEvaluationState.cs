using System.Globalization;
using Eitmad.Contracts;

namespace Eitmad.WindowsShell.Features.Reception;

public sealed partial class SalesCatalogViewModel
{
    private CancellationTokenSource? evaluationCancellation;
    private long evaluationVersion;
    private QuotationEvaluation? evaluation;
    public QuotationEvaluation? Evaluation => evaluation;
    internal Task LastQuotationEvaluation { get; private set; } = Task.CompletedTask;
    public string SubtotalLabel => catalogClient is not null && evaluation?.Totals is null ? "—" : Subtotal.ToString("N0", CultureInfo.InvariantCulture);

    private void ClearQuotationEvaluation()
    {
        ++evaluationVersion; evaluationCancellation?.Cancel(); evaluation = null; RaiseEvaluation();
    }

    /// <summary>Immediately removes obsolete totals and cancels work for the previous quotation intent.</summary>
    private void QueueQuotationEvaluation()
    {
        evaluationCancellation?.Cancel(); evaluationCancellation?.Dispose(); evaluationCancellation = new();
        var version = ++evaluationVersion; evaluation = null;
        RaiseEvaluation();
        if (catalogClient is null || !catalogActive) return;
        // Input conversion preserves exact basis points. Domain validity remains in Rust.
        if (!isDiscountValid) return;
        var lines = new List<QuotationLineIntent>();
        foreach (var line in QuotationLines)
        {
            var configuration = line.Intent;
            if (configuration is null) { QuotationNotice = "حدّث الأصناف واختر إعداداتها من الكتالوج."; return; }
            lines.Add(new() { Id = line.Id, Configuration = configuration });
        }
        var input = new EvaluateQuotation {
            Customer = SelectedCustomer?.Id is { } id ? new() { Id = id, Revision = SelectedCustomer.Revision } : null!,
            Lines = lines.ToArray(), DiscountBasisPoints = (long)(discountPercent * 100m),
        };
        LastQuotationEvaluation = EvaluateQuotationAsync(input, version, evaluationCancellation.Token);
    }

    /// <summary>Applies a reply only to the unchanged quotation in the current live session.</summary>
    private async Task EvaluateQuotationAsync(EvaluateQuotation input, long version, CancellationToken token)
    {
        QuotationNotice = "جارٍ تقييم عرض السعر...";
        try
        {
            await Task.Delay(200, token);
            var result = await catalogClient!.EvaluateAsync(input, token);
            if (version != evaluationVersion || token.IsCancellationRequested || !catalogActive) return;
            evaluation = result.Value;
            if (evaluation is not null) ApplyEvaluatedLines(evaluation);
            QuotationNotice = !result.Succeeded ? SalesCatalogClient.Message(result.Failure)
                : evaluation!.Errors.Length > 0 ? EvaluationMessage(evaluation.Errors[0])
                : evaluation.ServerAvailable ? "تم تقييم عرض السعر — لم يُحفظ أو يصدر."
                : "تم التقييم بآخر كتالوج مؤكد — الخادم غير متصل؛ لم يُحفظ أو يصدر عرض السعر.";
            RaiseEvaluation();
        }
        catch (OperationCanceledException) when (token.IsCancellationRequested) { }
    }

    private void RaiseEvaluation()
    {
        Raise(nameof(Evaluation)); Raise(nameof(Subtotal)); Raise(nameof(SubtotalLabel));
        RaiseDiscountState();
    }

    private void ApplyEvaluatedLines(QuotationEvaluation value)
    {
        applyingDraft = true;
        foreach (var result in value.Lines)
        {
            var index = QuotationLines.ToList().FindIndex(line => line.Id == result.Id);
            if (index >= 0) QuotationLines[index] = QuotationLines[index] with { SavedEvaluation = result };
        }
        applyingDraft = false;
    }

    private static string EvaluationMessage(QuotationFieldError error) => error.Field switch {
        QuotationField.Customer => "اختر عميلاً محفوظاً من نتائج البحث.",
        QuotationField.CustomerRevision => "تغيرت بيانات العميل. اختر العميل مجدداً.",
        QuotationField.PriceRevision => "تغير سعر البيع. حدّث الصنف وراجع السعر الجديد.",
        QuotationField.Target => "تغير الصنف أو لم يعد متاحاً. حدّث الصنف واختر من جديد.",
        QuotationField.Quantity => "تحقق من كمية الصنف.",
        QuotationField.Dimensions => "تحقق من المقاسات المسموحة للصنف.",
        QuotationField.ColorId or QuotationField.HandleId => "تحقق من الخيارات المتاحة للصنف.",
        QuotationField.Lines => "أضف صنفاً واحداً على الأقل.",
        QuotationField.DiscountBasisPoints => "أدخل نسبة من 0 إلى 100 بمنزلتين عشريتين كحد أقصى.",
        _ => "تعذر تقييم عرض السعر. تحقق من الأصناف والكميات.",
    };
}
