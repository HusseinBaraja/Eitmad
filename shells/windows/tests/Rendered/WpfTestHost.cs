using System.Windows;
using System.Windows.Automation;
using System.Windows.Media;
using System.Windows.Threading;
using Eitmad.WindowsShell.Features.Operations;

namespace Eitmad.WindowsShell.Tests.Rendered;

internal static class WpfTestHost
{
    private static readonly Lazy<Dispatcher> TestDispatcher = new(StartDispatcher);

    public static void Run(double width, double height, Action<MainWindow> test)
    {
        TestDispatcher.Value.Invoke(() =>
        {
            var window = new MainWindow(new OperationsViewModel())
            {
                Width = width,
                Height = height,
                Left = -10_000,
                Top = 0,
                ShowInTaskbar = false,
                WindowStartupLocation = WindowStartupLocation.Manual,
            };

            try
            {
                window.Show();
                window.Activate();
                CompleteLayout(window);
                test(window);
            }
            finally
            {
                window.Close();
                PumpDispatcher();
            }
        });
    }

    public static T FindByName<T>(DependencyObject root, string name) where T : FrameworkElement =>
        Descendants<T>(root).Single(element => element.Name == name);

    public static T FindByAutomationName<T>(DependencyObject root, string name) where T : DependencyObject =>
        Descendants<T>(root).First(element => AutomationProperties.GetName(element) == name);

    public static IEnumerable<T> Descendants<T>(DependencyObject root) where T : DependencyObject
    {
        if (root is T match)
        {
            yield return match;
        }

        for (var index = 0; index < VisualTreeHelper.GetChildrenCount(root); index++)
        {
            foreach (var descendant in Descendants<T>(VisualTreeHelper.GetChild(root, index)))
            {
                yield return descendant;
            }
        }
    }

    public static T Ancestor<T>(DependencyObject child) where T : DependencyObject
    {
        var current = VisualTreeHelper.GetParent(child);
        while (current is not null)
        {
            if (current is T match)
            {
                return match;
            }

            current = VisualTreeHelper.GetParent(current);
        }

        throw new InvalidOperationException($"No {typeof(T).Name} ancestor was found.");
    }

    public static void CompleteLayout(FrameworkElement root)
    {
        root.UpdateLayout();
        PumpDispatcher();
        root.UpdateLayout();
    }

    public static void PumpDispatcher() =>
        Dispatcher.CurrentDispatcher.Invoke(() => { }, DispatcherPriority.ApplicationIdle);

    private static Dispatcher StartDispatcher()
    {
        var ready = new TaskCompletionSource<Dispatcher>(TaskCreationOptions.RunContinuationsAsynchronously);
        var thread = new Thread(() =>
        {
            var application = new Application { ShutdownMode = ShutdownMode.OnExplicitShutdown };
            application.Resources.MergedDictionaries.Add(new ResourceDictionary
            {
                Source = new Uri("pack://application:,,,/Eitmad.WindowsShell;component/Resources/OperationsTheme.xaml"),
            });
            application.Resources.MergedDictionaries.Add(new ResourceDictionary
            {
                Source = new Uri("pack://application:,,,/Eitmad.WindowsShell;component/Resources/OperationsIcons.xaml"),
            });
            ready.SetResult(Dispatcher.CurrentDispatcher);
            Dispatcher.Run();
        })
        {
            IsBackground = true,
            Name = "Eitmad WPF test host",
        };
        thread.SetApartmentState(ApartmentState.STA);
        thread.Start();
        return ready.Task.GetAwaiter().GetResult();
    }
}
