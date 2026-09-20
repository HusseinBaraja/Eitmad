using System.IO;
using System.Windows;
using System.Windows.Automation;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using System.Windows.Threading;
using Eitmad.WindowsShell.Features.Operations;
using Eitmad.WindowsShell.Features.Authentication;

namespace Eitmad.WindowsShell.Tests.Rendered;

internal static class WpfTestHost
{
    private static readonly Lazy<Dispatcher> TestDispatcher = new(StartDispatcher);

    public static void Run(double width, double height, Action<MainWindow> test, bool showSignIn = false)
    {
        TestDispatcher.Value.Invoke(() =>
        {
            var window = new MainWindow(
                new OperationsViewModel(),
                showSignIn ? new RenderedSessionController() : null,
                showSignIn: showSignIn)
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

    internal static void SignIn(MainWindow window, string username)
    {
        FindByName<TextBox>(window, "UsernameBox").Text = username;
        FindByName<PasswordBox>(window, "PasswordInput").Password = "synthetic-password";
        FindByName<Button>(window, "SignInButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
        PumpDispatcher();
    }

    public static T FindByName<T>(DependencyObject root, string name) where T : FrameworkElement
    {
        var matches = Descendants<T>(root).Where(element => element.Name == name).ToList();
        return matches.Count == 1 ? matches[0] : matches.Single(element => element.IsVisible);
    }

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

    public static void Capture(FrameworkElement element, string name)
    {
        var directory = Environment.GetEnvironmentVariable("EITMAD_UI_CAPTURE_DIR");
        if (string.IsNullOrWhiteSpace(directory)) return;

        Directory.CreateDirectory(directory);
        // Popup roots carry the RTL transform that is absent on their child alone.
        element = PresentationSource.FromVisual(element)?.RootVisual as FrameworkElement ?? element;
        var bitmap = new RenderTargetBitmap((int)Math.Ceiling(element.ActualWidth),
            (int)Math.Ceiling(element.ActualHeight), 96, 96, PixelFormats.Pbgra32);
        bitmap.Render(element);
        var encoder = new PngBitmapEncoder();
        encoder.Frames.Add(BitmapFrame.Create(bitmap));
        using var stream = File.Create(Path.Combine(directory, name + ".png"));
        encoder.Save(stream);
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
            application.Resources.MergedDictionaries.Add(new ResourceDictionary
            {
                Source = new Uri("pack://application:,,,/Eitmad.WindowsShell;component/Resources/OperationsControls.xaml"),
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

    private sealed class RenderedSessionController : IDesktopSessionController
    {
        public event EventHandler<SessionEndedEventArgs>? SessionEnded
        {
            add { }
            remove { }
        }

        public Task StartAsync(CancellationToken cancellationToken = default) => Task.CompletedTask;

        public Task<AuthenticatedSurface> SignInAsync(
            string username,
            string password,
            CancellationToken cancellationToken = default) => Task.FromResult(
                username == "receptionist" ? AuthenticatedSurface.Receptionist : AuthenticatedSurface.Manager);

        public Task SignOutAsync(CancellationToken cancellationToken = default) => Task.CompletedTask;
        public Task StopAsync(CancellationToken cancellationToken = default) => Task.CompletedTask;
        public ValueTask DisposeAsync() => ValueTask.CompletedTask;
    }
}
