using System.Windows;
using System.Windows.Automation;
using System.Windows.Controls;
using System.Windows.Data;
using Eitmad.WindowsShell.Controls;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class FormFieldRenderedTests
{
    [TestMethod]
    public void LabelTracksInputReplacementWithoutChangingBindingTiming()
    {
        WpfTestHost.Run(780, 745, window =>
        {
            var input = new TextBox();
            input.SetBinding(TextBox.TextProperty, new Binding(nameof(Value.Text)) { Source = new Value(), UpdateSourceTrigger = UpdateSourceTrigger.LostFocus });
            var field = new FormField { Label = "اسم المادة", Content = input, IsRequired = true };
            window.Content = field;
            WpfTestHost.CompleteLayout(window);
            var label = (Label)field.Template.FindName("PART_Label", field);
            Assert.AreSame(input, label.Target);
            Assert.AreSame(label, AutomationProperties.GetLabeledBy(input));
            Assert.AreEqual("اسم المادة", AutomationProperties.GetName(label));
            Assert.AreEqual(UpdateSourceTrigger.LostFocus, input.GetBindingExpression(TextBox.TextProperty)!.ParentBinding.UpdateSourceTrigger);
            var replacement = new TextBox();
            field.Content = replacement;
            WpfTestHost.CompleteLayout(window);
            Assert.AreSame(replacement, label.Target);
            Assert.AreSame(label, AutomationProperties.GetLabeledBy(replacement));
            Assert.IsNull(AutomationProperties.GetLabeledBy(input));
        });
    }

    [TestMethod]
    public void OptionalHelpAndLongErrorsRenderWithoutOwningValidation()
    {
        WpfTestHost.Run(780, 745, window =>
        {
            var field = new FormField { Width = 260, Label = "سعر البيع", Content = new TextBox(), HelpText = "أدخل السعر للمقاس المحدد مع الحفاظ على بيانات المنتج", ErrorText = "يرجى مراجعة القيمة المدخلة ثم المحاولة مرة أخرى قبل حفظ التغييرات", FontSize = 24 };
            window.Content = field;
            WpfTestHost.CompleteLayout(window);
            var error = (TextBlock)field.Template.FindName("Error", field);
            var help = (TextBlock)field.Template.FindName("Help", field);
            Assert.AreEqual(TextWrapping.Wrap, error.TextWrapping);
            Assert.AreEqual(field.ErrorText, error.Text);
            Assert.IsFalse(Validation.GetHasError((TextBox)field.Content));
            field.HelpText = "";
            field.ErrorText = "";
            WpfTestHost.CompleteLayout(window);
            Assert.AreEqual(Visibility.Collapsed, help.Visibility);
            Assert.AreEqual(Visibility.Collapsed, error.Visibility);
        });
    }
    [TestMethod]
    [DataRow(1338d, 753d)]
    [DataRow(780d, 745d)]
    public void FeatureEditorsKeepTheirNamedInputAndLabel(double width, double height)
    {
        WpfTestHost.Run(width, height, window =>
        {
            foreach (var (page, action, inputName) in new[] { ("Materials", "إضافة مادة خام", "EditorNameBox"), ("Parts", "إضافة جزء", "EditorNameBox"), ("Furniture", "إضافة منتج", "FurnitureNameBox"), ("Products", "إضافة منتج", "ProductNameBox"), ("Pricing", "تعديل سعر البيع", "PriceInput") })
            {
                WpfTestHost.FindByName<Button>(window, page + "NavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                WpfTestHost.CompleteLayout(window);
                WpfTestHost.Descendants<Button>(window).First(button => button.IsVisible && AutomationProperties.GetName(button) == action).RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                WpfTestHost.CompleteLayout(window);
                var input = WpfTestHost.Descendants<TextBox>(window).Single(box => box.IsVisible && box.Name == inputName);
                Assert.IsNotNull(AutomationProperties.GetLabeledBy(input), page);
                Assert.IsNotNull(input.GetBindingExpression(TextBox.TextProperty), page);
                WpfTestHost.Capture(window, $"form-{page}-{width}");
            }
        });
    }
    public sealed class Value { public string Text { get; set; } = "خشب"; }
}
