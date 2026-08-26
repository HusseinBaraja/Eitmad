using System.Collections.ObjectModel;
using System.Globalization;
using System.IO;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.LocalIpc;
using Eitmad.Platform.Windows.ProcessSupervision;

namespace Eitmad.WindowsShell.Features.Operations;

public sealed class OperationsViewModel : ObservableObject
{
    private const string LocaleKey = ProtocolIds.ConfigKeys.EitmadConfigLocalePrimaryV1;
    private static readonly Guid ReferenceMarkerId = new("8b8ab1ab-731b-46f5-926a-3b5b2f8f6310");
    private readonly Dictionary<string, long> lastEventTime = [];
    private long configRevision = -1;
    private string selectedLocale = "ar-YE";
    private string connectionBanner = "جاري تشغيل محرك الاعتماد…";
    private string connectionDetail = "يتم الآن إنشاء قناة آمنة محلية";
    private string connectionTone = "Pending";
    private bool showConnectionBanner = true;
    private bool restartExhausted;
    private bool isSavingConfiguration;
    private bool isSavingReferenceMarker;
    private long referenceMarkerRevision = -1;
    private string referenceMarkerLabel = "مرجع REF-١٢";
    private string referenceMarkerStatus = "بانتظار لقطة المحرك";
    private StatusCard engineCard = new("المحرك", "قيد التشغيل", "بانتظار الجاهزية", "Pending");
    private StatusCard syncCard = new("المزامنة", "بانتظار المحرك", "لا توجد حالة حالية", "Muted");
    private StatusCard updateCard = new("التحديثات", "بانتظار المحرك", "السياسة يملكها محرك Rust", "Muted");

    public OperationsViewModel()
    {
        SaveConfigurationCommand = new AsyncCommand(SaveConfigurationAsync, () => CanSaveConfiguration, ObserveCommandFailure);
        RestartCommand = new AsyncCommand(RestartAsync, () => RestartExhausted && RestartEngine is not null, ObserveCommandFailure);
        SaveReferenceMarkerCommand = new AsyncCommand(SaveReferenceMarkerAsync, () => CanSaveReferenceMarker, ObserveCommandFailure);
    }

    public Func<UpdateConfiguration, Guid, Task>? SubmitConfigurationPatch { get; set; }
    public Func<UpsertReferenceMarker, Guid, Task>? SubmitReferenceMarker { get; set; }
    public Func<Task>? RestartEngine { get; set; }

    public StatusCard EngineCard { get => engineCard; private set => Set(ref engineCard, value); }
    public StatusCard SyncCard { get => syncCard; private set => Set(ref syncCard, value); }
    public StatusCard UpdateCard { get => updateCard; private set => Set(ref updateCard, value); }
    public string ConnectionBanner { get => connectionBanner; private set => Set(ref connectionBanner, value); }
    public string ConnectionDetail { get => connectionDetail; private set => Set(ref connectionDetail, value); }
    public string ConnectionTone { get => connectionTone; private set => Set(ref connectionTone, value); }
    public bool ShowConnectionBanner { get => showConnectionBanner; private set => Set(ref showConnectionBanner, value); }
    public bool RestartExhausted
    {
        get => restartExhausted;
        private set
        {
            if (Set(ref restartExhausted, value))
            {
                RestartCommand.Refresh();
            }
        }
    }
    public bool IsSavingConfiguration { get => isSavingConfiguration; private set => Set(ref isSavingConfiguration, value); }
    public long ConfigRevision => configRevision;
    public string ConfigurationRevisionLabel => configRevision < 0 ? "غير متاح" : $"الإصدار {configRevision}";
    public string CurrentDateLabel => DateTime.Now.ToString("dddd · d MMMM yyyy", CultureInfo.GetCultureInfo("ar-YE"));
    public bool CanSaveConfiguration => configRevision >= 0 && !IsSavingConfiguration;
    public bool CanSaveReferenceMarker => SubmitReferenceMarker is not null
        && !IsSavingReferenceMarker
        && !string.IsNullOrWhiteSpace(ReferenceMarkerLabel);
    public bool IsSavingReferenceMarker
    {
        get => isSavingReferenceMarker;
        private set
        {
            if (Set(ref isSavingReferenceMarker, value)) SaveReferenceMarkerCommand.Refresh();
        }
    }
    public string ReferenceMarkerStatus { get => referenceMarkerStatus; private set => Set(ref referenceMarkerStatus, value); }

    public string ReferenceMarkerLabel
    {
        get => referenceMarkerLabel;
        set
        {
            if (Set(ref referenceMarkerLabel, value)) SaveReferenceMarkerCommand.Refresh();
        }
    }

    public string SelectedLocale
    {
        get => selectedLocale;
        set
        {
            if (Set(ref selectedLocale, value))
            {
                SaveConfigurationCommand.Refresh();
            }
        }
    }

    public ObservableCollection<ConfigItem> Configuration { get; } = [];
    public ObservableCollection<JobItem> Jobs { get; } = [];
    public ObservableCollection<ActivityItem> Activity { get; } = [];
    public ObservableCollection<ReferenceMarkerItem> ReferenceMarkers { get; } = [];
    public AsyncCommand SaveConfigurationCommand { get; }
    public AsyncCommand RestartCommand { get; }
    public AsyncCommand SaveReferenceMarkerCommand { get; }

    public void ObserveSupervision(EngineSupervisionSnapshot snapshot)
    {
        var lifecycle = snapshot.LastLifecycle;
        var tone = lifecycle?.Health switch
        {
            HealthStatus.Healthy => "Success",
            HealthStatus.Degraded => "Warning",
            HealthStatus.Unhealthy => "Danger",
            _ => "Pending",
        };
        var health = lifecycle?.Health switch
        {
            HealthStatus.Healthy => "سليم",
            HealthStatus.Degraded => "يحتاج انتباهًا",
            HealthStatus.Unhealthy => "غير سليم",
            _ => MapSupervisionState(snapshot.State),
        };
        EngineCard = new(
            "المحرك",
            health,
            lifecycle is null
                ? $"المحاولة {snapshot.RestartCount} · الجيل {snapshot.Generation}"
                : lifecycle.Ready ? "جاهز لاستقبال الطلبات" : "غير جاهز بعد",
            tone);

        RestartExhausted = snapshot.State == EngineSupervisionState.RestartExhausted;
        switch (snapshot.IpcHealth)
        {
            case EngineIpcHealthState.Connected when lifecycle?.Ready == true:
                ShowConnectionBanner = false;
                ConnectionTone = "Success";
                break;
            case EngineIpcHealthState.ReconnectExhausted:
                ShowUnavailable("تعذر استعادة الاتصال بالمحرك", "أغلق التطبيق وافتحه من جديد. بياناتك بقيت لدى محرك Rust.", "Danger");
                break;
            case EngineIpcHealthState.Connecting:
                ShowUnavailable("نعيد الاتصال بالمحرك…", "ستُحدّث اللوحة تلقائيًا بعد استعادة الجلسة.", "Pending");
                break;
            default:
                ShowUnavailable(
                    RestartExhausted ? "توقفت محاولات إعادة تشغيل المحرك" : "المحرك غير متاح الآن",
                    RestartExhausted ? "تكررت الأعطال خلال فترة قصيرة. راجع رمز الخطأ ثم أعد المحاولة." : "سيحاول التطبيق استعادة الخدمة تلقائيًا.",
                    RestartExhausted ? "Danger" : "Warning");
                break;
        }

        if (snapshot.LastError is { } error)
        {
            AddError(error, DateTimeOffset.UtcNow.ToUnixTimeMilliseconds());
        }
    }

    public void ObserveStartupFailure(string detail)
    {
        EngineCard = new("المحرك", "تعذر البدء", "لم يتم العثور على محرك Rust أو تعذر تشغيله", "Danger");
        ShowUnavailable("تعذر تشغيل محرك الاعتماد", detail, "Danger");
    }

    public void ObserveConfiguration(ConfigSnapshot snapshot, long observedAt = long.MaxValue)
    {
        if (snapshot.Revision < configRevision || IsStale("configuration", observedAt))
        {
            return;
        }

        configRevision = snapshot.Revision;
        Configuration.Clear();
        foreach (var entry in snapshot.Entries ?? [])
        {
            var value = FormatConfigValue(entry.Value);
            Configuration.Add(new ConfigItem(
                entry.Key,
                MapConfigLabel(entry.Key),
                value,
                entry.Value.Kind,
                entry.Sensitivity,
                entry.RestartRequirement));
            if (entry.Key == LocaleKey && entry.Value.Value?.String is { Length: > 0 } locale)
            {
                SelectedLocale = locale;
            }
        }

        Raise(nameof(ConfigRevision));
        Raise(nameof(ConfigurationRevisionLabel));
        SaveConfigurationCommand.Refresh();
    }

    public void ObserveConfigurationUnavailable()
    {
        configRevision = -1;
        Configuration.Clear();
        Raise(nameof(ConfigRevision));
        Raise(nameof(ConfigurationRevisionLabel));
        SaveConfigurationCommand.Refresh();
    }

    public void ObserveReferenceMarkers(ReferenceMarkerPage page)
    {
        ReferenceMarkers.Clear();
        foreach (var marker in page.Items ?? [])
        {
            ReferenceMarkers.Add(new ReferenceMarkerItem(
                marker.Id,
                marker.Label,
                marker.Revision,
                marker.SyncState == ReferenceMarkerSyncState.Confirmed ? "متزامن" : "بانتظار المزامنة",
                DateTimeOffset.FromUnixTimeMilliseconds(marker.UpdatedAt).ToString("HH:mm", CultureInfo.GetCultureInfo("ar-YE"))));
            if (marker.Id == ReferenceMarkerId)
            {
                referenceMarkerRevision = marker.Revision;
                ReferenceMarkerLabel = marker.Label;
            }
        }
        ReferenceMarkerStatus = ReferenceMarkers.Count == 0 ? "لا توجد علامة محفوظة" : "اللقطة محدّثة";
        SaveReferenceMarkerCommand.Refresh();
    }

    public void ObserveReferenceMarker(ReferenceMarker marker)
    {
        ReplaceById(
            ReferenceMarkers,
            new ReferenceMarkerItem(
                marker.Id,
                marker.Label,
                marker.Revision,
                marker.SyncState == ReferenceMarkerSyncState.Confirmed ? "متزامن" : "بانتظار المزامنة",
                DateTimeOffset.FromUnixTimeMilliseconds(marker.UpdatedAt).ToString("HH:mm", CultureInfo.GetCultureInfo("ar-YE"))),
            value => value.Id);
        if (marker.Id == ReferenceMarkerId)
        {
            referenceMarkerRevision = marker.Revision;
            ReferenceMarkerLabel = marker.Label;
        }
        ReferenceMarkerStatus = "تم تحديث العلامة";
        SaveReferenceMarkerCommand.Refresh();
    }

    public void ObserveReferenceMarkerChanged(ReferenceMarkerChangeNotice notice)
    {
        if (notice.MarkerId == ReferenceMarkerId)
        {
            ReferenceMarkerStatus = $"وصل التغيير {notice.Revision} · نحدّث اللقطة";
        }
    }

    public void ObserveReferenceMarkersUnavailable()
    {
        ReferenceMarkers.Clear();
        ReferenceMarkerStatus = "الميزة المرجعية غير متاحة";
    }

    public void ObserveSync(SyncStatus status, long observedAt = long.MaxValue)
    {
        if (IsStale("sync", observedAt))
        {
            return;
        }

        var payload = status.Payload;
        SyncCard = status.Kind switch
        {
            SyncStatusKind.Current => new("المزامنة", "محدّث", "كل السجلات المتاحة متزامنة", "Success", 1),
            SyncStatusKind.Syncing => new("المزامنة", "تجري الآن", SyncProgress(payload), "Accent", CalculateProgress(payload?.Completed, payload?.Total)),
            SyncStatusKind.Queued => new("المزامنة", "في الانتظار", $"{payload?.Records ?? 0} سجل بانتظار الإرسال", "Warning"),
            SyncStatusKind.Conflicted => new("المزامنة", "تعارضات", $"{payload?.Records ?? 0} سجل يحتاج إلى مراجعة", "Danger"),
            SyncStatusKind.Failed => new("المزامنة", "تعذرت", payload?.Reason ?? "راجع تفاصيل الخطأ", "Danger"),
            _ => new("المزامنة", "دون اتصال", "العمل المحلي متاح حسب وضع المنتج", "Muted"),
        };
    }

    public void ObserveSyncUnavailable(string errorCode) =>
        SyncCard = new("المزامنة", "غير متاحة", errorCode, "Muted");

    public void ObserveUpdate(UpdateState state, long observedAt = long.MaxValue)
    {
        if (IsStale("update", observedAt))
        {
            return;
        }

        var progress = (state.Payload?.ProgressBps ?? 0) / 10_000d;
        UpdateCard = state.Kind switch
        {
            UpdateStateKind.Idle => new("التحديثات", "محدّث", "لا توجد عملية تحديث نشطة", "Success"),
            UpdateStateKind.Available => new("التحديثات", "متاح", $"الإصدار {state.Payload?.Version}", "Accent"),
            UpdateStateKind.Downloading => new("التحديثات", "تنزيل", $"{progress:P0} · {state.Payload?.Version}", "Accent", progress),
            UpdateStateKind.Failed or UpdateStateKind.RecoveryRequired => new("التحديثات", "تحتاج تدخّلًا", state.Payload?.ErrorCode ?? "تعذر التحديث", "Danger", progress),
            UpdateStateKind.Paused => new("التحديثات", "متوقف مؤقتًا", state.Payload?.Version ?? "—", "Warning", progress),
            UpdateStateKind.Succeeded => new("التحديثات", "اكتمل", state.Payload?.Version ?? "—", "Success", 1),
            _ => new("التحديثات", MapUpdateState(state.Kind), state.Payload?.Version ?? "تتم إدارة السياسة في Rust", "Pending", progress),
        };
    }

    public void ObserveUpdateUnavailable(string errorCode) =>
        UpdateCard = new("التحديثات", "غير متاحة", errorCode, "Muted");

    public void ObserveJob(BackgroundJobStatus job, long observedAt)
    {
        if (IsStale($"job:{job.JobId}", observedAt))
        {
            return;
        }

        var item = new JobItem(
            job.JobId,
            MapJobKind(job.JobKind),
            MapJobState(job.State),
            job.TotalUnits is { } total ? $"{job.CompletedUnits:N0} من {total:N0}" : $"{job.CompletedUnits:N0} وحدة",
            CalculateProgress(job.CompletedUnits, job.TotalUnits),
            job.State switch
            {
                BackgroundJobState.Succeeded => "Success",
                BackgroundJobState.Failed => "Danger",
                BackgroundJobState.Cancelled => "Muted",
                _ => "Accent",
            });
        ReplaceById(Jobs, item, value => value.Id);
        if (job.Error is { } error)
        {
            AddError(error, observedAt);
        }
    }

    public void ObserveNotification(Notification notification, long observedAt)
    {
        if (Activity.Any(item => item.Id == notification.NotificationId))
        {
            return;
        }

        Activity.Insert(0, new ActivityItem(
            notification.NotificationId,
            LocalizeMessage(notification.MessageId),
            FormatParameters(notification.Parameters),
            FormatTime(observedAt),
            notification.Severity switch
            {
                NotificationSeverity.Success => "Success",
                NotificationSeverity.Warning => "Warning",
                NotificationSeverity.Error => "Danger",
                _ => "Accent",
            },
            "إشعار"));
        TrimActivity();
    }

    public void ObserveError(ScopedError scoped, long observedAt) => AddError(scoped.Error, observedAt);

    public void BeginResynchronization(string stream)
    {
        lastEventTime.Remove(stream);
        ShowUnavailable("نحدّث الحالة من المصدر…", "طلب المحرك إعادة مزامنة الاشتراك، وستحل اللقطة الجديدة محل الحالة المؤقتة.", "Pending");
        if (stream is "jobs" or "notifications" or "errors")
        {
            if (stream == "jobs") Jobs.Clear();
            if (stream is "notifications" or "errors") Activity.Clear();
        }
    }

    public void MarkSnapshotsCurrent()
    {
        ShowConnectionBanner = false;
        ConnectionTone = "Success";
    }

    private async Task SaveConfigurationAsync()
    {
        if (SubmitConfigurationPatch is null || !CanSaveConfiguration)
        {
            return;
        }

        IsSavingConfiguration = true;
        SaveConfigurationCommand.Refresh();
        try
        {
            await SubmitConfigurationPatch(
                new UpdateConfiguration
                {
                    ExpectedRevision = configRevision,
                    Changes =
                    [
                        new ConfigChange
                        {
                            Key = LocaleKey,
                            Value = new ConfigWriteValue
                            {
                                Kind = ConfigWriteValueKind.Text,
                                Value = selectedLocale,
                            },
                        },
                    ],
                },
                Guid.NewGuid());
        }
        catch (Exception error) when (error is InvalidOperationException or IOException or EngineIpcException)
        {
            ShowUnavailable("تعذر حفظ رقعة الإعدادات", "احتفظ التطبيق بالقيمة الحالية. حدّث اللقطة ثم أعد المحاولة.", "Danger");
        }
        finally
        {
            IsSavingConfiguration = false;
            SaveConfigurationCommand.Refresh();
        }
    }

    private async Task RestartAsync()
    {
        if (RestartEngine is not null)
        {
            await RestartEngine();
        }
    }

    private async Task SaveReferenceMarkerAsync()
    {
        if (SubmitReferenceMarker is null || !CanSaveReferenceMarker)
        {
            return;
        }

        IsSavingReferenceMarker = true;
        try
        {
            await SubmitReferenceMarker(
                new UpsertReferenceMarker
                {
                    MarkerId = ReferenceMarkerId,
                    ExpectedRevision = referenceMarkerRevision < 0 ? null : referenceMarkerRevision,
                    Label = ReferenceMarkerLabel,
                },
                Guid.NewGuid());
        }
        catch (Exception error) when (error is InvalidOperationException or IOException or EngineIpcException)
        {
            ReferenceMarkerStatus = "تعذر الحفظ. حدّث اللقطة ثم أعد المحاولة.";
        }
        finally
        {
            IsSavingReferenceMarker = false;
        }
    }

    private bool IsStale(string stream, long observedAt)
    {
        if (observedAt == long.MaxValue)
        {
            return false;
        }

        if (lastEventTime.TryGetValue(stream, out var current) && observedAt < current)
        {
            return true;
        }

        lastEventTime[stream] = observedAt;
        return false;
    }

    private void AddError(ContractError error, long observedAt)
    {
        var id = error.CorrelationId == Guid.Empty ? Guid.NewGuid() : error.CorrelationId;
        if (Activity.Any(item => item.Id == id))
        {
            return;
        }

        Activity.Insert(0, new ActivityItem(
            id,
            LocalizeMessage(error.MessageId),
            error.Code,
            FormatTime(observedAt),
            "Danger",
            "خطأ"));
        TrimActivity();
    }

    private void ShowUnavailable(string title, string detail, string tone)
    {
        ConnectionBanner = title;
        ConnectionDetail = detail;
        ConnectionTone = tone;
        ShowConnectionBanner = true;
    }

    private void ObserveCommandFailure(Exception _) =>
        ShowUnavailable("تعذر تنفيذ الإجراء", "احتفظ التطبيق بالحالة الحالية. أعد المحاولة بعد استعادة اتصال المحرك.", "Danger");

    private void TrimActivity()
    {
        while (Activity.Count > 50)
        {
            Activity.RemoveAt(Activity.Count - 1);
        }
    }

    private static void ReplaceById<T, TId>(ObservableCollection<T> values, T value, Func<T, TId> id)
        where TId : notnull
    {
        var index = values.Select((item, position) => (item, position)).FirstOrDefault(item => EqualityComparer<TId>.Default.Equals(id(item.item), id(value))).position;
        if (values.Count > 0 && index < values.Count && EqualityComparer<TId>.Default.Equals(id(values[index]), id(value)))
        {
            values[index] = value;
        }
        else
        {
            values.Insert(0, value);
        }
    }

    private static double CalculateProgress(long? completed, long? total) =>
        total is > 0 ? Math.Clamp((double)(completed ?? 0) / total.Value, 0, 1) : 0;

    private static string SyncProgress(SyncStatusPayload? payload) => payload?.Total is > 0
        ? $"{payload.Completed ?? 0:N0} من {payload.Total:N0} سجل"
        : $"{payload?.Completed ?? 0:N0} سجل";

    private static string FormatConfigValue(ConfigReadValue value) => value.Kind switch
    {
        ConfigReadValueKind.Boolean => value.Value?.Bool == true ? "نعم" : "لا",
        ConfigReadValueKind.Integer => value.Value?.Integer?.ToString(CultureInfo.CurrentCulture) ?? "—",
        ConfigReadValueKind.TextList => string.Join("، ", value.Value?.StringArray ?? []),
        ConfigReadValueKind.Redacted => "••••••••",
        _ => value.Value?.String ?? "—",
    };

    private static string MapConfigLabel(string key) => key == LocaleKey ? "لغة الواجهة الأساسية" : key;
    private static string MapSupervisionState(EngineSupervisionState state) => state switch
    {
        EngineSupervisionState.Starting => "يبدأ الآن",
        EngineSupervisionState.RestartDelay => "يعيد التشغيل",
        EngineSupervisionState.RestartExhausted => "توقفت المحاولات",
        EngineSupervisionState.Stopping => "يتوقف بأمان",
        EngineSupervisionState.Faulted => "تعذر التشغيل",
        _ => "غير متاح",
    };

    private static string MapUpdateState(UpdateStateKind state) => state switch
    {
        UpdateStateKind.Checking => "يتحقق",
        UpdateStateKind.Preflight => "فحص قبل التثبيت",
        UpdateStateKind.Verifying => "يتحقق من الحزمة",
        UpdateStateKind.Ready => "جاهز للتثبيت",
        UpdateStateKind.Installing => "يثبّت الآن",
        UpdateStateKind.InstallationHandoff => "انتقل إلى المثبّت",
        UpdateStateKind.Revoked => "تم سحبه",
        _ => "قيد المعالجة",
    };

    private static string MapJobKind(string kind) => kind switch
    {
        "sync" => "مزامنة السجلات",
        "update" => "تجهيز التحديث",
        "export" => "تصدير البيانات",
        _ => kind,
    };

    private static string MapJobState(BackgroundJobState state) => state switch
    {
        BackgroundJobState.Queued => "في الانتظار",
        BackgroundJobState.Running => "يعمل الآن",
        BackgroundJobState.Succeeded => "اكتمل",
        BackgroundJobState.Failed => "تعذر",
        _ => "أُلغي",
    };

    private static string LocalizeMessage(string messageId) => messageId switch
    {
        ProtocolIds.MessageIds.EitmadNotificationSyncCompleteV1 => "اكتملت المزامنة",
        ProtocolIds.MessageIds.EitmadNotificationUpdateReadyV1 => "التحديث جاهز",
        _ => messageId,
    };

    private static string FormatParameters(IEnumerable<ErrorParameter>? parameters) =>
        string.Join(" · ", (parameters ?? []).Select(parameter => parameter.Name));

    private static string FormatTime(long unixMillis) => unixMillis is <= 0 or long.MaxValue
        ? "الآن"
        : DateTimeOffset.FromUnixTimeMilliseconds(unixMillis).LocalDateTime.ToString("HH:mm", CultureInfo.CurrentCulture);
}
