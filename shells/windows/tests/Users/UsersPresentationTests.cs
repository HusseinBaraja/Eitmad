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
    public void AddRequiresNameAndOneOfTheThreeRoles()
    {
        var vm = new UsersViewModel(); vm.BeginEdit();
        Assert.IsFalse(vm.ApplyPreview());
        vm.EditorName = "سالم أحمد"; vm.EditorRole = "غير معروف";
        Assert.IsFalse(vm.ApplyPreview());
        vm.EditorRole = "النجار";
        Assert.IsTrue(vm.ApplyPreview());
        Assert.AreEqual(5, vm.VisibleUsers.Count);
    }
}
