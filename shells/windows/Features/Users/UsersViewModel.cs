using System.Collections.ObjectModel;

namespace Eitmad.WindowsShell.Features.Users;

// Synthetic presentation state only. Account authority remains in Rust.
public sealed record UserPreview(string Name, string Role, bool IsActive = true)
{
    public string Status => IsActive ? "نشط" : "غير نشط";
}

public sealed class UsersViewModel : ObservableObject
{
    private readonly List<UserPreview> users =
    [
        new("محمد سالم", "مدير"),
        new("أحمد علي", "موظف الاستقبال"),
        new("خالد حسن", "النجار"),
        new("عمر سعيد", "النجار", false),
    ];
    public IReadOnlyList<string> Roles { get; } = ["مدير", "موظف الاستقبال", "النجار"];
    public IReadOnlyList<string> RoleOptions { get; } = ["كل الأدوار", "مدير", "موظف الاستقبال", "النجار"];
    public IReadOnlyList<string> StatusOptions { get; } = ["كل الحالات", "نشط", "غير نشط"];
    public ObservableCollection<UserPreview> VisibleUsers { get; } = [];
    private string searchText = "", selectedRole = "كل الأدوار", selectedStatus = "كل الحالات";
    public string SearchText { get => searchText; set { if (Set(ref searchText, value)) Refresh(); } }
    public string SelectedRole { get => selectedRole; set { if (Set(ref selectedRole, value)) Refresh(); } }
    public string SelectedStatus { get => selectedStatus; set { if (Set(ref selectedStatus, value)) Refresh(); } }
    private bool isEditorOpen, isDeactivationOpen;
    private string editorName = "", editorRole = "مدير", editorError = "", editorTitle = "", deactivationName = "";
    private UserPreview? editing, deactivating;
    public bool IsEditorOpen { get => isEditorOpen; private set => Set(ref isEditorOpen, value); }
    public bool IsDeactivationOpen { get => isDeactivationOpen; private set => Set(ref isDeactivationOpen, value); }
    public string EditorName { get => editorName; set => Set(ref editorName, value); }
    public string EditorRole { get => editorRole; set => Set(ref editorRole, value); }
    public string EditorError { get => editorError; private set => Set(ref editorError, value); }
    public string EditorTitle { get => editorTitle; private set => Set(ref editorTitle, value); }
    public string DeactivationName { get => deactivationName; private set => Set(ref deactivationName, value); }
    public UsersViewModel() => Refresh();
    private void Refresh()
    {
        var query = PreviewText.NormalizeSearch(SearchText.Trim());
        VisibleUsers.Clear();
        foreach (var user in users.Where(user =>
            PreviewText.NormalizeSearch(user.Name).Contains(query, StringComparison.OrdinalIgnoreCase)
            && (SelectedRole == "كل الأدوار" || user.Role == SelectedRole)
            && (SelectedStatus == "كل الحالات" || user.Status == SelectedStatus)))
            VisibleUsers.Add(user);
    }
    public void BeginEdit(UserPreview? user = null)
    {
        editing = user;
        EditorName = user?.Name ?? "";
        EditorRole = user?.Role ?? Roles[0];
        EditorError = "";
        EditorTitle = user is null ? "إضافة مستخدم" : "تعديل المستخدم";
        IsEditorOpen = true;
    }
    public void CancelEditor() { IsEditorOpen = false; editing = null; }
    public bool ApplyPreview()
    {
        if (!IsEditorOpen) return false;
        if (string.IsNullOrWhiteSpace(EditorName) || !Roles.Contains(EditorRole))
        {
            EditorError = "أدخل الاسم واختر الدور.";
            return false;
        }
        var replacement = new UserPreview(EditorName.Trim(), EditorRole, editing?.IsActive ?? true);
        if (editing is null) users.Add(replacement);
        else users[users.IndexOf(editing)] = replacement;
        CancelEditor();
        Refresh();
        return true;
    }
    public void BeginDeactivation(UserPreview user)
    {
        if (!users.Contains(user) || !user.IsActive) return;
        deactivating = user;
        DeactivationName = user.Name;
        IsDeactivationOpen = true;
    }
    public void CancelDeactivation() { IsDeactivationOpen = false; deactivating = null; }
    public void DeactivatePreview()
    {
        if (deactivating is null) return;
        users[users.IndexOf(deactivating)] = deactivating with { IsActive = false };
        CancelDeactivation();
        Refresh();
    }
}
