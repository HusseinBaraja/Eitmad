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
        account.Role switch { DesktopAccountRole.Manager => "مدير", DesktopAccountRole.Receptionist => "موظف الاستقبال", _ => "غير محدد" },
        account.Active,
        account.Username,
        account.Revision);
}

public sealed class UsersViewModel : ObservableObject
{
    private readonly List<UserPreview> users = [];
    private int generation;
    private Guid submissionKey = Guid.NewGuid();
    private IEngineShellBridge? engine;
    private string searchText = "", selectedRole = "كل الأدوار", selectedStatus = "كل الحالات";
    private bool isEditorOpen, isDeactivationOpen, isBusy;
    private string editorName = "", editorRole = "مدير", editorError = "", editorTitle = "", deactivationName = "";
    private string editorUsername = "";
    private string editorNameError = "", editorUsernameError = "", editorPasswordError = "", editorRoleError = "";
    private string deactivationUsername = "";
    private string loadError = "";
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
    public bool IsBusy
    {
        get => isBusy;
        private set
        {
            if (!Set(ref isBusy, value)) return;
            Raise(nameof(CanRetry));
            Raise(nameof(CanManageAccounts));
        }
    }
    public string LoadError
    {
        get => loadError;
        private set
        {
            if (!Set(ref loadError, value)) return;
            Raise(nameof(HasLoadFailure));
            Raise(nameof(LoadNotice));
            Raise(nameof(ShowTable));
            Raise(nameof(CanManageAccounts));
        }
    }
    public bool HasLoadFailure => LoadError.Length > 0;
    public string LoadNotice => HasLoadFailure && users.Count > 0
        ? $"{LoadError} المعروض من آخر تحميل ناجح؛ حالة الحسابات الحالية غير مؤكدة. أعد التحميل قبل إجراء تغييرات."
        : LoadError;
    public bool ShowTable => !HasLoadFailure || users.Count > 0;
    public bool CanRetry => engine is not null && !IsBusy;
    public bool CanManageAccounts => CanRetry && !HasLoadFailure;
    public string EditorUsername { get => editorUsername; set { if (Set(ref editorUsername, value)) { EditorUsernameError = ""; submissionKey = Guid.NewGuid(); } } }
    public bool IsDeactivationOpen { get => isDeactivationOpen; private set => Set(ref isDeactivationOpen, value); }
    public string EditorName { get => editorName; set { if (Set(ref editorName, value)) { EditorNameError = ""; submissionKey = Guid.NewGuid(); } } }
    public string EditorRole { get => editorRole; set { if (Set(ref editorRole, value)) { EditorRoleError = ""; submissionKey = Guid.NewGuid(); } } }
    public string EditorError { get => editorError; private set => Set(ref editorError, value); }
    public string EditorNameError { get => editorNameError; private set => Set(ref editorNameError, value); }
    public string EditorUsernameError { get => editorUsernameError; private set => Set(ref editorUsernameError, value); }
    public string EditorPasswordError { get => editorPasswordError; private set => Set(ref editorPasswordError, value); }
    public string EditorRoleError { get => editorRoleError; private set => Set(ref editorRoleError, value); }
    public string EditorTitle { get => editorTitle; private set => Set(ref editorTitle, value); }
    public string DeactivationName { get => deactivationName; private set => Set(ref deactivationName, value); }
    public string DeactivationUsername { get => deactivationUsername; private set => Set(ref deactivationUsername, value); }

    public void Attach(IEngineShellBridge bridge)
    {
        engine = bridge;
        Raise(nameof(CanRetry));
        Raise(nameof(CanManageAccounts));
    }

    public async Task<bool> LoadAsync(CancellationToken cancellationToken = default)
    {
        if (engine is null || IsBusy) return false;
        IsBusy = true;
        var current = generation;
        try
        {
            var response = await engine.QueryAsync(
                Query.ForDesktopAccountList(new ListDesktopAccounts()),
                cancellationToken);
            if (current != generation) return false;
            var page = response.Outcome.Status == CommandOutcomeStatus.Succeeded
                ? response.Outcome.Payload.AsDesktopAccounts()
                : null;
            if (page is null)
            {
                var denied = response.Outcome.Payload.Code == ProtocolIds.ErrorCodes.EitmadErrorAuthorizationDeniedV1;
                if (denied)
                {
                    users.Clear();
                    Refresh();
                }
                LoadError = denied ? FailureMessage(response.Outcome.Payload.Code) : "تعذر تحميل المستخدمين.";
                return false;
            }
            users.Clear();
            users.AddRange((page.Accounts ?? []).Select(UserPreview.FromContract));
            LoadError = "";
            EditorError = "";
            Refresh();
            return true;
        }
        catch
        {
            if (current == generation) LoadError = "تعذر تحميل المستخدمين. تحقق من الاتصال بالخادم.";
            return false;
        }
        finally
        {
            if (current == generation) IsBusy = false;
        }
    }

    public void ClearSession()
    {
        ++generation; IsBusy = false;
        users.Clear(); VisibleUsers.Clear();
        CancelEditor(); CancelDeactivation();
        EditorName = ""; EditorUsername = ""; DeactivationName = ""; DeactivationUsername = "";
        SearchText = ""; LoadError = "";
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
        if (!CanManageAccounts || (user is not null && !users.Contains(user))) return;
        submissionKey = Guid.NewGuid();
        editing = user;
        Raise(nameof(IsEditing));
        EditorUsername = user?.Username ?? "";
        EditorName = user?.Name ?? "";
        EditorRole = user?.Role ?? Roles[0];
        EditorError = "";
        ClearInputErrors();
        EditorTitle = user is null ? "إضافة مستخدم" : "تعديل المستخدم";
        IsEditorOpen = true;
    }

    public void CancelEditor()
    {
        IsEditorOpen = false;
        editing = null;
        EditorError = "";
        ClearInputErrors();
        Raise(nameof(IsEditing));
    }

    public void ClearPasswordError() { EditorPasswordError = ""; submissionKey = Guid.NewGuid(); }

    private void ClearInputErrors()
    {
        EditorNameError = "";
        EditorUsernameError = "";
        EditorPasswordError = "";
        EditorRoleError = "";
    }

    public async Task<bool> ApplyAsync(string password, CancellationToken cancellationToken = default)
    {
        if (!IsEditorOpen || !CanManageAccounts) return false;
        EditorError = "";
        EditorNameError = string.IsNullOrWhiteSpace(EditorName) ? "أدخل الاسم." : "";
        EditorUsernameError = string.IsNullOrWhiteSpace(EditorUsername) ? "أدخل اسم المستخدم." : "";
        EditorRoleError = !Roles.Contains(EditorRole) ? "اختر الدور." : "";
        EditorPasswordError = editing is null && password.Length < 12
            ? "أدخل كلمة مرور مؤقتة من 12 حرفًا على الأقل." : "";
        if (EditorNameError.Length > 0 || EditorUsernameError.Length > 0
            || EditorRoleError.Length > 0 || EditorPasswordError.Length > 0)
            return false;

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
        if (!CanManageAccounts || !users.Contains(user) || !user.IsActive) return;
        submissionKey = Guid.NewGuid();
        deactivating = user;
        DeactivationName = user.Name;
        DeactivationUsername = user.Username;
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
        if (deactivating is null || !CanManageAccounts) return false;
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
        var current = generation;
        try
        {
            var response = await engine!.SubmitCommandAsync(command, submissionKey, cancellationToken);
            if (current != generation) return false;
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
            if (current == generation) EditorError = "لم يصل تأكيد الحفظ. أعد المحاولة للتحقق من النتيجة.";
            return false;
        }
        finally
        {
            if (current == generation) IsBusy = false;
        }
    }

    private static string FailureMessage(string? code) => code switch
    {
        ProtocolIds.ErrorCodes.EitmadErrorDesktopAccountRevisionConflictV1 => "تغير المستخدم على جهاز آخر. راجع البيانات وحاول مرة أخرى.",
        ProtocolIds.ErrorCodes.EitmadErrorAuthorizationDeniedV1 => "ليس لديك صلاحية لإدارة المستخدمين. تتطلب الحسابات المشتركة صلاحية مالك المنشأة.",
        ProtocolIds.ErrorCodes.EitmadErrorDesktopAccountInvalidV1 => "تحقق من البيانات. قد يكون اسم المستخدم مستخدمًا بالفعل.",
        _ => "تعذر إكمال العملية. حاول مرة أخرى.",
    };
}
