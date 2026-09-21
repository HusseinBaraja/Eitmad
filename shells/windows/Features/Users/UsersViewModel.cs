using System.Collections.ObjectModel;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.Shell;

namespace Eitmad.WindowsShell.Features.Users;

public sealed record UserPreview(
    Guid AccountId,
    Guid UserId,
    string Name,
    string Role,
    bool IsActive,
    string Username,
    long Revision)
{
    public string Status => IsActive ? "نشط" : "غير نشط";

    public static UserPreview FromContract(DesktopAccountSummary account) => new(
        account.AccountId,
        account.UserId,
        account.DisplayName,
        account.Role == DesktopAccountRole.Manager ? "مدير" : "موظف الاستقبال",
        account.Active,
        account.Username,
        account.Revision);
}

public sealed class UsersViewModel : ObservableObject
{
    private readonly List<UserPreview> users = [];
    private IEngineShellBridge? engine;
    private string searchText = "", selectedRole = "كل الأدوار", selectedStatus = "كل الحالات";
    private bool isEditorOpen, isDeactivationOpen, isBusy;
    private string editorName = "", editorRole = "مدير", editorError = "", editorTitle = "", deactivationName = "";
    private string editorUsername = "";
    private UserPreview? editing, deactivating;

    public IReadOnlyList<string> Roles { get; } = ["مدير", "موظف الاستقبال"];
    public IReadOnlyList<string> RoleOptions { get; } = ["كل الأدوار", "مدير", "موظف الاستقبال"];
    public IReadOnlyList<string> StatusOptions { get; } = ["كل الحالات", "نشط", "غير نشط"];
    public ObservableCollection<UserPreview> VisibleUsers { get; } = [];
    public string SearchText { get => searchText; set { if (Set(ref searchText, value)) Refresh(); } }
    public string SelectedRole { get => selectedRole; set { if (Set(ref selectedRole, value)) Refresh(); } }
    public string SelectedStatus { get => selectedStatus; set { if (Set(ref selectedStatus, value)) Refresh(); } }
    public bool IsEditorOpen { get => isEditorOpen; private set { if (Set(ref isEditorOpen, value)) Raise(nameof(IsListVisible)); } }
    public bool IsListVisible => !IsEditorOpen;
    public bool IsEditing => editing is not null;
    public bool IsBusy { get => isBusy; private set => Set(ref isBusy, value); }
    public string EditorUsername { get => editorUsername; set => Set(ref editorUsername, value); }
    public bool IsDeactivationOpen { get => isDeactivationOpen; private set => Set(ref isDeactivationOpen, value); }
    public string EditorName { get => editorName; set => Set(ref editorName, value); }
    public string EditorRole { get => editorRole; set => Set(ref editorRole, value); }
    public string EditorError { get => editorError; private set => Set(ref editorError, value); }
    public string EditorTitle { get => editorTitle; private set => Set(ref editorTitle, value); }
    public string DeactivationName { get => deactivationName; private set => Set(ref deactivationName, value); }

    public void Attach(IEngineShellBridge bridge) => engine = bridge;

    public async Task<bool> LoadAsync(CancellationToken cancellationToken = default)
    {
        if (engine is null || IsBusy) return false;
        IsBusy = true;
        try
        {
            var response = await engine.QueryAsync(
                Query.ForDesktopAccountList(new ListDesktopAccounts()),
                cancellationToken);
            var page = response.Outcome.Status == CommandOutcomeStatus.Succeeded
                ? response.Outcome.Payload.AsDesktopAccounts()
                : null;
            if (page is null)
            {
                EditorError = FailureMessage(response.Outcome.Payload.Code);
                return false;
            }
            users.Clear();
            users.AddRange((page.Accounts ?? []).Select(UserPreview.FromContract));
            EditorError = "";
            Refresh();
            return true;
        }
        catch
        {
            EditorError = "تعذر تحميل المستخدمين. حاول مرة أخرى.";
            return false;
        }
        finally
        {
            IsBusy = false;
        }
    }

    private void Refresh()
    {
        var query = PreviewText.NormalizeSearch(SearchText.Trim());
        VisibleUsers.Clear();
        foreach (var user in users.Where(user =>
            (PreviewText.NormalizeSearch(user.Name).Contains(query, StringComparison.OrdinalIgnoreCase)
             || PreviewText.NormalizeSearch(user.Username).Contains(query, StringComparison.OrdinalIgnoreCase))
            && (SelectedRole == "كل الأدوار" || user.Role == SelectedRole)
            && (SelectedStatus == "كل الحالات" || user.Status == SelectedStatus)))
            VisibleUsers.Add(user);
    }

    public void BeginEdit(UserPreview? user = null)
    {
        editing = user;
        Raise(nameof(IsEditing));
        EditorUsername = user?.Username ?? "";
        EditorName = user?.Name ?? "";
        EditorRole = user?.Role ?? Roles[0];
        EditorError = "";
        EditorTitle = user is null ? "إضافة مستخدم" : "تعديل المستخدم";
        IsEditorOpen = true;
    }

    public void CancelEditor()
    {
        IsEditorOpen = false;
        editing = null;
        EditorError = "";
        Raise(nameof(IsEditing));
    }

    public async Task<bool> ApplyAsync(string password, CancellationToken cancellationToken = default)
    {
        if (!IsEditorOpen || engine is null || IsBusy) return false;
        if (string.IsNullOrWhiteSpace(EditorName)
            || string.IsNullOrWhiteSpace(EditorUsername)
            || !Roles.Contains(EditorRole)
            || (editing is null && password.Length < 12))
        {
            EditorError = "أدخل الاسم واسم المستخدم واختر الدور. يجب أن تتكون كلمة المرور من 12 حرفًا على الأقل.";
            return false;
        }

        var role = EditorRole == "مدير" ? DesktopAccountRole.Manager : DesktopAccountRole.Receptionist;
        var command = editing is null
            ? Command.ForDesktopAccountCreate(new CreateDesktopAccount
            {
                DisplayName = EditorName.Trim(),
                Username = EditorUsername.Trim(),
                Password = password,
                Role = role,
            })
            : Command.ForDesktopAccountUpdate(new UpdateDesktopAccount
            {
                AccountId = editing.AccountId,
                ExpectedRevision = editing.Revision,
                DisplayName = EditorName.Trim(),
                Role = role,
            });
        if (!await SubmitAsync(command, cancellationToken)) return false;
        CancelEditor();
        return await LoadAsync(cancellationToken);
    }

    public void BeginDeactivation(UserPreview user)
    {
        if (!users.Contains(user) || !user.IsActive) return;
        deactivating = user;
        DeactivationName = user.Name;
        EditorError = "";
        IsDeactivationOpen = true;
    }

    public void CancelDeactivation()
    {
        IsDeactivationOpen = false;
        deactivating = null;
    }

    public async Task<bool> DeactivateAsync(CancellationToken cancellationToken = default)
    {
        if (deactivating is null || engine is null || IsBusy) return false;
        var target = deactivating;
        if (!await SubmitAsync(Command.ForDesktopAccountDeactivate(new DeactivateDesktopAccount
        {
            AccountId = target.AccountId,
            ExpectedRevision = target.Revision,
        }), cancellationToken)) return false;
        CancelDeactivation();
        return await LoadAsync(cancellationToken);
    }

    private async Task<bool> SubmitAsync(Command command, CancellationToken cancellationToken)
    {
        IsBusy = true;
        try
        {
            var response = await engine!.SubmitCommandAsync(command, Guid.NewGuid(), cancellationToken);
            if (response.Outcome.Status == CommandOutcomeStatus.Succeeded)
            {
                EditorError = "";
                return true;
            }
            EditorError = FailureMessage(response.Outcome.Payload.Code);
            return false;
        }
        catch
        {
            EditorError = "تعذر حفظ التغيير. حاول مرة أخرى.";
            return false;
        }
        finally
        {
            IsBusy = false;
        }
    }

    private static string FailureMessage(string? code) => code switch
    {
        ProtocolIds.ErrorCodes.EitmadErrorDesktopAccountLastManagerV1 => "يجب أن يبقى مدير نشط واحد على الأقل.",
        ProtocolIds.ErrorCodes.EitmadErrorDesktopAccountRevisionConflictV1 => "تغير المستخدم على جهاز آخر. راجع البيانات وحاول مرة أخرى.",
        ProtocolIds.ErrorCodes.EitmadErrorAuthorizationDeniedV1 => "ليس لديك صلاحية لإدارة المستخدمين.",
        ProtocolIds.ErrorCodes.EitmadErrorDesktopAccountInvalidV1 => "تحقق من البيانات. قد يكون اسم المستخدم مستخدمًا بالفعل.",
        _ => "تعذر إكمال العملية. حاول مرة أخرى.",
    };
}
