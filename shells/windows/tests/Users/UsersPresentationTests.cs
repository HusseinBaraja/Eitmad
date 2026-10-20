using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.Users;
using Eitmad.WindowsShell.Tests.TestDoubles;

namespace Eitmad.WindowsShell.Tests.Users;

[TestClass]
public sealed class UsersPresentationTests
{
    [TestMethod]
    public async Task SessionClearDiscardsAnAccountReadAlreadyInFlight()
    {
        var engine = Engine();
        var release = new TaskCompletionSource();
        engine.QueryBarrier = _ => release.Task;
        var vm = new UsersViewModel(); vm.Attach(engine);
        var loading = vm.LoadAsync();
        vm.ClearSession();
        release.SetResult();
        Assert.IsFalse(await loading);
        Assert.AreEqual(0, vm.VisibleUsers.Count);
        Assert.IsFalse(vm.IsEditorOpen);
        Assert.IsFalse(vm.IsDeactivationOpen);
    }

    private static FakeEngine Engine()
    {
        var engine = new FakeEngine();
        engine.DesktopAccounts.AddRange([
            new DesktopAccountSummary { AccountId = Guid.NewGuid(), UserId = Guid.NewGuid(), DisplayName = "محمد سالم", Username = "m.salem", Role = DesktopAccountRole.Manager, Active = true, Revision = 1 },
            new DesktopAccountSummary { AccountId = Guid.NewGuid(), UserId = Guid.NewGuid(), DisplayName = "أحمد علي", Username = "a.ali", Role = DesktopAccountRole.Receptionist, Active = true, Revision = 3 },
            new DesktopAccountSummary { AccountId = Guid.NewGuid(), UserId = Guid.NewGuid(), DisplayName = "عمر سعيد", Username = "o.saeed", Role = DesktopAccountRole.Receptionist, Active = false, Revision = 2 },
        ]);
        return engine;
    }

    [TestMethod]
    [DataRow(true)]
    [DataRow(false)]
    public async Task FailedRefreshRetainsLastKnownRowsBlocksChangesAndRetryReplacesThem(bool transportFailure)
    {
        var engine = Engine();
        var vm = new UsersViewModel();
        vm.Attach(engine);
        Assert.IsTrue(await vm.LoadAsync());
        vm.SearchText = "احمد";
        var retained = vm.VisibleUsers.Single();
        engine.ThrowQueries = transportFailure;
        if (!transportFailure)
            engine.QueryHandler = _ => new QueryResponseEnvelope
            {
                Outcome = new QueryOutcome { Status = CommandOutcomeStatus.Failed, Payload = new QueryResult() },
            };

        Assert.IsFalse(await vm.LoadAsync());
        Assert.IsTrue(vm.HasLoadFailure);
        Assert.IsTrue(vm.ShowTable);
        Assert.AreEqual(retained, vm.VisibleUsers.Single());
        Assert.IsFalse(vm.CanManageAccounts);
        Assert.IsTrue(vm.CanRetry);
        vm.BeginEdit(retained);
        vm.BeginDeactivation(retained);
        Assert.IsFalse(vm.IsEditorOpen);
        Assert.IsFalse(vm.IsDeactivationOpen);
        Assert.IsFalse(await vm.ApplyAsync("synthetic-password"));
        Assert.IsFalse(await vm.DeactivateAsync());
        Assert.IsNull(engine.LastCommand);
        vm.CancelEditor();
        Assert.IsTrue(vm.HasLoadFailure);

        engine.ThrowQueries = false;
        engine.QueryHandler = null;
        engine.DesktopAccounts[1].Active = false;
        var release = new TaskCompletionSource();
        engine.QueryBarrier = _ => release.Task;
        var retry = vm.LoadAsync();
        Assert.IsTrue(vm.IsBusy);
        Assert.IsFalse(vm.CanRetry);
        Assert.IsFalse(vm.CanManageAccounts);
        Assert.IsFalse(await vm.LoadAsync());
        Assert.IsTrue(vm.HasLoadFailure);
        release.SetResult();
        Assert.IsTrue(await retry);
        Assert.IsFalse(vm.HasLoadFailure);
        Assert.IsTrue(vm.CanManageAccounts);
        Assert.AreEqual("احمد", vm.SearchText);
        Assert.IsFalse(vm.VisibleUsers.Single().IsActive);
    }

    [TestMethod]
    public async Task InitialFailureDoesNotShowAnEmptyResultAndDenialRemovesRetainedAccounts()
    {
        var engine = Engine();
        var vm = new UsersViewModel();
        vm.Attach(engine);
        engine.ThrowQueries = true;
        Assert.IsFalse(await vm.LoadAsync());
        Assert.IsFalse(vm.ShowTable);
        Assert.IsFalse(vm.CanManageAccounts);
        engine.ThrowQueries = false;
        Assert.IsTrue(await vm.LoadAsync());
        var previous = vm.VisibleUsers[0];
        engine.QueryHandler = _ => new QueryResponseEnvelope
        {
            Outcome = new QueryOutcome
            {
                Status = CommandOutcomeStatus.Failed,
                Payload = new QueryResult { Code = ProtocolIds.ErrorCodes.EitmadErrorAuthorizationDeniedV1 },
            },
        };
        Assert.IsFalse(await vm.LoadAsync());
        Assert.IsFalse(vm.ShowTable);
        Assert.AreEqual(0, vm.VisibleUsers.Count);
        Assert.IsFalse(vm.CanManageAccounts);
        vm.SearchText = "";
        Assert.AreEqual(0, vm.VisibleUsers.Count);
        vm.BeginEdit(previous);
        Assert.IsFalse(vm.IsEditorOpen);
        engine.QueryHandler = null;
        Assert.IsTrue(await vm.LoadAsync());
        Assert.IsTrue(vm.ShowTable);
        Assert.IsTrue(vm.CanManageAccounts);
    }

    [TestMethod]
    public async Task SearchAndFiltersUseRustReturnedAccounts()
    {
        var vm = new UsersViewModel();
        vm.Attach(Engine());
        Assert.IsTrue(await vm.LoadAsync());
        vm.SearchText = "احمد";
        vm.SelectedRole = "موظف الاستقبال";
        vm.SelectedStatus = "نشط";
        Assert.AreEqual("أحمد علي", vm.VisibleUsers.Single().Name);
        vm.SelectedStatus = "غير نشط";
        Assert.AreEqual(0, vm.VisibleUsers.Count);
    }

    [TestMethod]
    public async Task CreateEditAndDeactivateUseTypedEngineOperations()
    {
        var engine = Engine();
        var vm = new UsersViewModel();
        vm.Attach(engine);
        await vm.LoadAsync();

        vm.BeginEdit();
        vm.EditorName = "سالم أحمد";
        vm.EditorUsername = "s.ahmad";
        vm.EditorRole = "موظف الاستقبال";
        Assert.IsTrue(await vm.ApplyAsync("temporary-password"));
        var createCommand = engine.LastCommand?.AsDesktopAccountCreate();
        Assert.IsNotNull(createCommand);
        Assert.IsTrue(createCommand.Password == "temporary-password");
        var created = vm.VisibleUsers.Single(user => user.Username == "s.ahmad");
        Assert.IsTrue(created.IsActive);

        vm.BeginEdit(created);
        vm.EditorName = "سالم محمد";
        vm.EditorUsername = "cannot-change";
        vm.EditorRole = "مدير";
        Assert.IsTrue(await vm.ApplyAsync(""));
        var updateCommand = engine.LastCommand?.AsDesktopAccountUpdate();
        Assert.IsNotNull(updateCommand);
        Assert.AreEqual(created.AccountId, updateCommand.AccountId);
        Assert.AreEqual(1L, updateCommand.ExpectedRevision);
        var edited = vm.VisibleUsers.Single(user => user.AccountId == created.AccountId);
        Assert.AreEqual("s.ahmad", edited.Username);
        Assert.AreEqual("سالم محمد", edited.Name);
        Assert.AreEqual("مدير", edited.Role);
        Assert.AreEqual(2, edited.Revision);

        vm.BeginDeactivation(edited);
        Assert.IsTrue(await vm.DeactivateAsync());
        var deactivateCommand = engine.LastCommand?.AsDesktopAccountDeactivate();
        Assert.IsNotNull(deactivateCommand);
        Assert.AreEqual(edited.AccountId, deactivateCommand.AccountId);
        Assert.AreEqual(2L, deactivateCommand.ExpectedRevision);
        Assert.IsFalse(vm.VisibleUsers.Single(user => user.AccountId == created.AccountId).IsActive);
    }

    [TestMethod]
    public async Task SameDisplayNameKeepsDeactivationIdentityAndTargetDistinct()
    {
        var engine = Engine();
        engine.DesktopAccounts[1].DisplayName = engine.DesktopAccounts[0].DisplayName;
        var vm = new UsersViewModel();
        vm.Attach(engine);
        Assert.IsTrue(await vm.LoadAsync());
        var first = vm.VisibleUsers[0];
        var second = vm.VisibleUsers[1];

        vm.BeginDeactivation(first);
        Assert.AreEqual(first.Name, vm.DeactivationName);
        Assert.AreEqual(first.Username, vm.DeactivationUsername);
        vm.CancelDeactivation();
        Assert.IsNull(engine.LastCommand);

        vm.BeginDeactivation(second);
        Assert.AreEqual(second.Name, vm.DeactivationName);
        Assert.AreEqual(second.Username, vm.DeactivationUsername);
        Assert.IsTrue(await vm.DeactivateAsync());
        var command = engine.LastCommand?.AsDesktopAccountDeactivate();
        Assert.IsNotNull(command);
        Assert.AreEqual(second.AccountId, command.AccountId);
        Assert.AreEqual(3L, command.ExpectedRevision);
        Assert.IsTrue(vm.VisibleUsers.Single(user => user.AccountId == first.AccountId).IsActive);
        Assert.IsFalse(vm.VisibleUsers.Single(user => user.AccountId == second.AccountId).IsActive);
        Assert.AreEqual(first.Role, vm.VisibleUsers.Single(user => user.AccountId == first.AccountId).Role);
        Assert.AreEqual(second.Role, vm.VisibleUsers.Single(user => user.AccountId == second.AccountId).Role);
    }

    [TestMethod]
    [DataRow("")]
    [DataRow("short")]
    public async Task ShortPasswordOnlyMarksPasswordAndPreservesValidInputs(string password)
    {
        var engine = Engine();
        var vm = new UsersViewModel();
        vm.Attach(engine);
        vm.BeginEdit();
        vm.EditorName = "مستخدم تجريبي";
        vm.EditorUsername = "synthetic.user";
        vm.EditorRole = "موظف الاستقبال";

        Assert.IsFalse(await vm.ApplyAsync(password));
        Assert.IsTrue(vm.IsEditorOpen);
        Assert.AreEqual("مستخدم تجريبي", vm.EditorName);
        Assert.AreEqual("synthetic.user", vm.EditorUsername);
        Assert.AreEqual("موظف الاستقبال", vm.EditorRole);
        Assert.AreEqual("", vm.EditorNameError);
        Assert.AreEqual("", vm.EditorUsernameError);
        Assert.AreEqual("", vm.EditorRoleError);
        Assert.AreEqual("", vm.EditorError);
        Assert.IsTrue(vm.EditorPasswordError.Length > 0);
        Assert.IsNull(engine.LastCommand);

        vm.ClearPasswordError();
        Assert.AreEqual("", vm.EditorPasswordError);
        Assert.IsTrue(await vm.ApplyAsync("synthetic-password"));
        Assert.IsNotNull(engine.LastCommand?.AsDesktopAccountCreate());
    }

    [TestMethod]
    public async Task AuthorityFailureUsesSummaryAndPreservesInputsWithoutFieldErrors()
    {
        var engine = Engine();
        engine.CommandHandler = _ => new CommandResponseEnvelope
        {
            Outcome = new CommandOutcome
            {
                Status = CommandOutcomeStatus.Failed,
                Payload = new CommandResult { Code = ProtocolIds.ErrorCodes.EitmadErrorAuthorizationDeniedV1 },
            },
        };
        var vm = new UsersViewModel();
        vm.Attach(engine);
        vm.BeginEdit();
        vm.EditorName = "مستخدم تجريبي";
        vm.EditorUsername = "synthetic.user";
        Assert.IsFalse(await vm.ApplyAsync("synthetic-password"));
        Assert.IsTrue(vm.IsEditorOpen);
        Assert.AreEqual("مستخدم تجريبي", vm.EditorName);
        Assert.AreEqual("synthetic.user", vm.EditorUsername);
        Assert.IsTrue(vm.EditorError.Length > 0);
        Assert.AreEqual("", vm.EditorNameError);
        Assert.AreEqual("", vm.EditorUsernameError);
        Assert.AreEqual("", vm.EditorPasswordError);
        Assert.AreEqual("", vm.EditorRoleError);
    }

    [TestMethod]
    public async Task ValidationAndServerDenialKeepTheEditorState()
    {
        var engine = Engine();
        var vm = new UsersViewModel();
        vm.Attach(engine);
        await vm.LoadAsync();
        vm.BeginEdit();
        vm.EditorRole = "";
        Assert.IsFalse(await vm.ApplyAsync("short"));
        Assert.IsTrue(vm.IsEditorOpen);
        Assert.IsTrue(vm.EditorNameError.Length > 0);
        Assert.IsTrue(vm.EditorUsernameError.Length > 0);
        Assert.IsTrue(vm.EditorPasswordError.Length > 0);
        Assert.IsTrue(vm.EditorRoleError.Length > 0);
        Assert.AreEqual("", vm.EditorError);

        engine.CommandHandler = _ => new CommandResponseEnvelope
        {
            RequestId = Guid.NewGuid(),
            CorrelationId = Guid.NewGuid(),
            Outcome = new CommandOutcome
            {
                Status = CommandOutcomeStatus.Failed,
                Payload = new CommandResult { Code = ProtocolIds.ErrorCodes.EitmadErrorAuthorizationDeniedV1 },
            },
        };
        vm.CancelEditor();
        Assert.AreEqual("", vm.EditorNameError);
        Assert.AreEqual("", vm.EditorUsernameError);
        Assert.AreEqual("", vm.EditorPasswordError);
        Assert.AreEqual("", vm.EditorRoleError);
        var manager = vm.VisibleUsers.Single(user => user.Role == "مدير");
        vm.BeginDeactivation(manager);
        Assert.IsFalse(await vm.DeactivateAsync());
        StringAssert.Contains(vm.EditorError, "مالك");
        Assert.IsTrue(vm.IsDeactivationOpen);
    }
}
