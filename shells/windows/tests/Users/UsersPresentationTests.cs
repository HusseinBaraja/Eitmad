using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.Users;
using Eitmad.WindowsShell.Tests.TestDoubles;

namespace Eitmad.WindowsShell.Tests.Users;

[TestClass]
public sealed class UsersPresentationTests
{
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
    public async Task ValidationAndLastManagerFailureKeepTheEditorState()
    {
        var engine = Engine();
        var vm = new UsersViewModel();
        vm.Attach(engine);
        await vm.LoadAsync();
        vm.BeginEdit();
        Assert.IsFalse(await vm.ApplyAsync("short"));
        Assert.IsTrue(vm.IsEditorOpen);

        engine.CommandHandler = _ => new CommandResponseEnvelope
        {
            RequestId = Guid.NewGuid(),
            CorrelationId = Guid.NewGuid(),
            Outcome = new CommandOutcome
            {
                Status = CommandOutcomeStatus.Failed,
                Payload = new CommandResult { Code = ProtocolIds.ErrorCodes.EitmadErrorDesktopAccountLastManagerV1 },
            },
        };
        vm.CancelEditor();
        var manager = vm.VisibleUsers.Single(user => user.Role == "مدير");
        vm.BeginDeactivation(manager);
        Assert.IsFalse(await vm.DeactivateAsync());
        StringAssert.Contains(vm.EditorError, "مدير نشط");
        Assert.IsTrue(vm.IsDeactivationOpen);
    }
}
