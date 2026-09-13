using Eitmad.WindowsShell.Features.Users;
namespace Eitmad.WindowsShell.Tests.Users;
[TestClass]
public sealed class UsersPresentationTests
{
    [TestMethod]
    public void SearchAndFiltersCombineWithoutChangingNames()
    {
        var vm = new UsersViewModel { SearchText = "احمد", SelectedRole = "موظف الاستقبال", SelectedStatus = "نشط" };
        Assert.AreEqual("أحمد علي", vm.VisibleUsers.Single().Name);
        vm.SelectedStatus = "غير نشط";
        Assert.AreEqual(0, vm.VisibleUsers.Count);
    }
    [TestMethod]
    public void CancelPreservesPreviewAndDeactivationRetainsTheUser()
    {
        var vm = new UsersViewModel();
        var user = vm.VisibleUsers[1];
        vm.BeginEdit(user); vm.EditorName = "تغيير"; vm.CancelEditor();
        Assert.AreEqual(user, vm.VisibleUsers[1]);
        vm.BeginDeactivation(user); vm.CancelDeactivation();
        Assert.IsTrue(vm.VisibleUsers[1].IsActive);
        vm.BeginDeactivation(user); vm.DeactivatePreview();
        Assert.AreEqual(4, vm.VisibleUsers.Count);
        Assert.IsFalse(vm.VisibleUsers.Single(item => item.Name == user.Name).IsActive);
        vm.BeginEdit(vm.VisibleUsers[1]); vm.EditorRole = "النجار";
        Assert.IsTrue(vm.ApplyPreview());
        Assert.IsFalse(vm.VisibleUsers[1].IsActive);
        Assert.IsTrue(new UsersViewModel().VisibleUsers[1].IsActive);
    }
    [TestMethod]
    public void EditChangesNameRoleAndStatusButPreservesUsername()
    {
        var vm = new UsersViewModel();
        var user = vm.VisibleUsers.Last();
        vm.BeginEdit(user);
        Assert.IsTrue(vm.IsEditing);
        vm.EditorName = "سالم حسن";
        vm.EditorUsername = "changed";
        vm.EditorRole = "موظف الاستقبال";
        vm.EditorStatus = "نشط";
        Assert.IsTrue(vm.ApplyPreview());
        var edited = vm.VisibleUsers.Last();
        Assert.AreEqual(user.Username, edited.Username);
        Assert.AreEqual("سالم حسن", edited.Name);
        Assert.AreEqual("موظف الاستقبال", edited.Role);
        Assert.IsTrue(edited.IsActive);
        Assert.IsTrue(vm.IsListVisible);
    }
    [TestMethod]
    public void AddRequiresNameAndOneOfTheThreeRoles()
    {
        var vm = new UsersViewModel(); vm.BeginEdit();
        Assert.IsFalse(vm.ApplyPreview());
        vm.EditorName = "سالم أحمد"; vm.EditorRole = "غير معروف";
        Assert.IsFalse(vm.ApplyPreview());
        vm.EditorRole = "النجار";
        Assert.IsFalse(vm.ApplyPreview());
        vm.EditorUsername = "s.ahmad";
        vm.EditorStatus = "غير نشط";
        Assert.IsTrue(vm.ApplyPreview());
        Assert.AreEqual("s.ahmad", vm.VisibleUsers.Last().Username);
        Assert.IsFalse(vm.VisibleUsers.Last().IsActive);
        Assert.AreEqual(5, vm.VisibleUsers.Count);
    }
}
