using System.Text;

namespace Eitmad.WindowsShell.Features.Customers;

// Early field feedback only. Rust customer contracts remain authoritative.
internal static class CustomerInputValidation
{
    public static bool IsNameValid(string value) => IsRequiredTextValid(value, 256);

    public static bool IsSearchTermValid(string value) =>
        value.Trim() == value && Encoding.UTF8.GetByteCount(value) <= 256
        && !value.Any(IsUnsafeCharacter);

    public static bool IsAddressValid(string value) => IsOptionalTextValid(value, 512);

    public static bool IsNotesValid(string value) => IsOptionalTextValid(value, 2_048);

    public static bool IsPhoneValid(string value)
    {
        if (!IsRequiredTextValid(value, 64)) return false;
        var hasDigit = false;
        for (var index = 0; index < value.Length; index++)
        {
            var character = value[index];
            if (character is >= '0' and <= '9' or >= '\u0660' and <= '\u0669' or >= '\u06f0' and <= '\u06f9')
            {
                hasDigit = true;
                continue;
            }
            if (character == '+' && index == 0 || character is ' ' or '-' or '(' or ')') continue;
            return false;
        }
        return hasDigit;
    }

    private static bool IsRequiredTextValid(string value, int maximumBytes) =>
        value.Length > 0 && value.Trim() == value && Encoding.UTF8.GetByteCount(value) <= maximumBytes
        && !value.Any(IsUnsafeCharacter);

    private static bool IsOptionalTextValid(string value, int maximumBytes) =>
        value.Length > 0 && value.Trim() == value && Encoding.UTF8.GetByteCount(value) <= maximumBytes
        && !value.Any(character => IsDirectionalMark(character)
            || char.IsControl(character) && character is not ('\n' or '\r' or '\t'));

    private static bool IsUnsafeCharacter(char character) =>
        char.IsControl(character) || IsDirectionalMark(character);

    private static bool IsDirectionalMark(char character) =>
        character is '\u061c' or >= '\u200e' and <= '\u200f'
            or >= '\u202a' and <= '\u202e' or >= '\u2066' and <= '\u2069';
}
