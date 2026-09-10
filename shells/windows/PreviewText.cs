using System.Globalization;
using System.Text;

namespace Eitmad.WindowsShell;

// Text conversion for transient preview input. Never rewrite stored user text.
internal static class PreviewText
{
    public static string NormalizeSearch(string value)
    {
        var decomposed = value.Normalize(NormalizationForm.FormD);
        var normalized = new StringBuilder(decomposed.Length);
        foreach (var character in decomposed)
        {
            var category = CharUnicodeInfo.GetUnicodeCategory(character);
            if (character == '\u0640'
                || category is UnicodeCategory.NonSpacingMark
                    or UnicodeCategory.SpacingCombiningMark
                    or UnicodeCategory.EnclosingMark)
            {
                continue;
            }

            normalized.Append(character switch
            {
                '\u0622' or '\u0623' or '\u0625' or '\u0671' => '\u0627',
                '\u0649' => '\u064A',
                '\u0629' => '\u0647',
                _ => character,
            });
        }

        return normalized.ToString().Normalize(NormalizationForm.FormC);
    }

    public static string NormalizeNumericInput(string value)
    {
        var normalized = new StringBuilder(value.Length);
        foreach (var character in value)
        {
            normalized.Append(character switch
            {
                >= '\u0660' and <= '\u0669' => (char)('0' + character - '\u0660'),
                >= '\u06F0' and <= '\u06F9' => (char)('0' + character - '\u06F0'),
                '\u066B' => '.',
                '\u066C' => ',',
                _ => character,
            });
        }

        return normalized.ToString();
    }
}
