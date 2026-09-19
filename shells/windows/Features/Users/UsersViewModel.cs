using System.Collections.ObjectModel;

namespace Eitmad.WindowsShell.Features.Users;

// Synthetic presentation state only. Account authority remains in Rust.
public sealed record UserPreview(string Name, string Role, bool IsActive = true, string Username = "")
{
    public string Status => IsActive ? "نشط" : "غير نشط";
}

public sealed class UsersViewModel : ObservableObject
{
    private readonly List<UserPreview> users =
    [
        new("محمد سالم", "مدير", Username: "m.salem"),
        new("أحمد علي", "موظف الاستقبال", Username: "a.ali"),
        new("خالد حسن", "النجار", Username: "k.hassan"),
        new("عمر سعيد", "النجار", false, "o.saeed"),
    ];
    public IReadOnlyList<string> Roles { get; } = ["مدير", "موظف الاستقبال", "النجار"];
    public IReadOnlyList<string> RoleOptions { get; } = ["كل الأدوار", "مدير", "موظف الاستقبال", "النجار"];
    public IReadOnlyList<string> StatusOptions { get; } = ["كل الحالات", "نشط", "غير نشط"];
    public IReadOnlyList<string> EditorStatuses { get; } = ["نشط", "غير نشط"];
    public ObservableCollection<UserPreview> VisibleUsers { get; } = [];
    private string searchText = "", selectedRole = "كل الأدوار", selectedStatus = "كل الحالات";
    public string SearchText { get => searchText; set { if (Set(ref searchText, value)) Refresh(); } }
    public string SelectedRole { get => selectedRole; set { if (Set(ref selectedRole, value)) Refresh(); } }
    public string SelectedStatus { get => selectedStatus; set { if (Set(ref selectedStatus, value)) Refresh(); } }
    private bool isEditorOpen, isDeactivationOpen;
    private string editorName = "", editorRole = "مدير", editorError = "", editorTitle = "", deactivationName = "";
    private UserPreview? editing, deactivating;
    public bool IsEditorOpen { get => isEditorOpen; private set { if (Set(ref isEditorOpen, value)) Raise(nameof(IsListVisible)); } }
    public bool IsListVisible => !IsEditorOpen;
    public bool IsEditing => editing is not null;
    private string editorUsername = "", editorStatus = "نشط";
    public string EditorUsername { get => editorUsername; set => Set(ref editorUsername, value); }
    public string EditorStatus { get => editorStatus; set => Set(ref editorStatus, value); }
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
        Raise(nameof(IsEditing));
        EditorUsername = user?.Username ?? "";
        EditorStatus = user?.Status ?? EditorStatuses[0];
        EditorName = user?.Name ?? "";
        EditorRole = user?.Role ?? Roles[0];
        EditorError = "";
        EditorTitle = user is null ? "إضافة مستخدم" : "تعديل المستخدم";
        IsEditorOpen = true;
    }
    public void CancelEditor() { IsEditorOpen = false; editing = null; Raise(nameof(IsEditing)); }
    public bool ApplyPreview()
    {
        if (!IsEditorOpen) return false;
        if (string.IsNullOrWhiteSpace(EditorName) || string.IsNullOrWhiteSpace(EditorUsername) || !Roles.Contains(EditorRole) || !EditorStatuses.Contains(EditorStatus))
        {
            EditorError = "أدخل الاسم واسم المستخدم واختر الدور والحالة.";
            return false;
        }
        var replacement = new UserPreview(EditorName.Trim(), EditorRole, EditorStatus == "نشط", editing?.Username ?? EditorUsername.Trim());
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
