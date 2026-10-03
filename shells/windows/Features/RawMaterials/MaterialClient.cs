using System.IO;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.LocalIpc;
using Eitmad.Platform.Windows.Shell;

namespace Eitmad.WindowsShell.Features.RawMaterials;

public enum MaterialFailureKind { None, Validation, Reference, Conflict, Denied, Unavailable }

public sealed record MaterialResult<T>(T? Value, MaterialFailureKind Failure)
{
    public bool Succeeded => Failure == MaterialFailureKind.None && Value is not null;
    public static MaterialResult<T> Success(T value) => new(value, MaterialFailureKind.None);
    public static MaterialResult<T> Failed(MaterialFailureKind failure) => new(default, failure);
}

public sealed record MaterialSnapshot(MaterialReferences References, IReadOnlyList<Material> Materials);

/// <summary>Thin typed IPC adapter for material definitions and change notifications.</summary>
public sealed class MaterialClient : IAsyncDisposable
{
    private readonly IEngineShellBridge engine;
    private readonly EngineChangeFeed changes;

    public MaterialClient(IEngineShellBridge engine)
    {
        this.engine = engine;
        changes = new EngineChangeFeed(engine, ProtocolIds.Capabilities.EitmadCapabilityMaterialV1,
            Subscription.ForMaterialChangedSubscribe(new MaterialChanges()),
            notice => { if (notice is null || notice.AsMaterialChangedEvent() is not null) SignalChanged(); });
    }

    public event EventHandler? Changed;

    public async Task<MaterialResult<MaterialSnapshot>> LoadAsync(string term, CancellationToken cancellationToken = default)
    {
        if (!engine.SupportsCapability(ProtocolIds.Capabilities.EitmadCapabilityMaterialV1))
            return MaterialResult<MaterialSnapshot>.Failed(MaterialFailureKind.Unavailable);
        try
        {
            var referencesResponse = await engine.QueryAsync(
                Query.ForMaterialReferenceList(new ListMaterialReferences()), cancellationToken);
            var references = referencesResponse.Outcome.Status == CommandOutcomeStatus.Succeeded
                ? referencesResponse.Outcome.Payload.AsMaterialReferences() : null;
            if (references is null)
                return MaterialResult<MaterialSnapshot>.Failed(MapFailure(referencesResponse.Outcome.Payload.Code));

            var items = new List<Material>();
            Guid? after = null;
            do
            {
                var response = await engine.QueryAsync(Query.ForMaterialList(new ListMaterials
                    { Term = term, After = after, Limit = 100 }), cancellationToken);
                var page = response.Outcome.Status == CommandOutcomeStatus.Succeeded
                    ? response.Outcome.Payload.AsMaterials() : null;
                if (page is null) return MaterialResult<MaterialSnapshot>.Failed(MapFailure(response.Outcome.Payload.Code));
                items.AddRange(page.Items);
                after = page.Next;
            } while (after is not null);
            return MaterialResult<MaterialSnapshot>.Success(new MaterialSnapshot(references, items));
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested) { throw; }
        catch (EngineIpcException error) { return MaterialResult<MaterialSnapshot>.Failed(MapFailure(error.ContractError?.Code)); }
        catch (Exception error) when (error is IOException or InvalidOperationException or ObjectDisposedException)
        { return MaterialResult<MaterialSnapshot>.Failed(MaterialFailureKind.Unavailable); }
    }

    public Task<MaterialFailureKind> SaveAsync(SaveMaterial input, CancellationToken cancellationToken = default) =>
        SubmitAsync(Command.ForMaterialSave(input), cancellationToken);
    public Task<MaterialFailureKind> SaveAsync(SaveMaterialCategory input, CancellationToken cancellationToken = default) =>
        SubmitAsync(Command.ForMaterialCategorySave(input), cancellationToken);
    public Task<MaterialFailureKind> SaveAsync(SaveMaterialUnit input, CancellationToken cancellationToken = default) =>
        SubmitAsync(Command.ForMaterialUnitSave(input), cancellationToken);

    private async Task<MaterialFailureKind> SubmitAsync(Command command, CancellationToken cancellationToken)
    {
        try
        {
            var response = await engine.SubmitCommandAsync(command, Guid.NewGuid(), cancellationToken);
            return response.Outcome.Status == CommandOutcomeStatus.Succeeded
                ? MaterialFailureKind.None : MapFailure(response.Outcome.Payload.Code);
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested) { throw; }
        catch (EngineIpcException error) { return MapFailure(error.ContractError?.Code); }
        catch (Exception error) when (error is IOException or InvalidOperationException or ObjectDisposedException)
        { return MaterialFailureKind.Unavailable; }
    }

    public Task ActivateAsync() => changes.ActivateAsync();

    private void SignalChanged() => Changed?.Invoke(this, EventArgs.Empty);

    public ValueTask DisposeAsync() => changes.DisposeAsync();

    public static string ArabicMessage(MaterialFailureKind failure) => failure switch
    {
        MaterialFailureKind.Validation => "تحقق من الاسم والتكلفة والبيانات المطلوبة.",
        MaterialFailureKind.Reference => "المرجع غير نشط أو مستخدم. راجع التصنيف أو الوحدة.",
        MaterialFailureKind.Conflict => "تغيرت البيانات في مكان آخر. لم تُحفظ تعديلاتك. أعد فتح السجل.",
        MaterialFailureKind.Denied => "ليس لديك صلاحية لتعديل المواد الخام.",
        _ => "تعذر الاتصال ببيانات المواد الخام. حاول مرة أخرى.",
    };

    private static MaterialFailureKind MapFailure(string? code) => code switch
    {
        ProtocolIds.ErrorCodes.EitmadErrorMaterialInvalidV1 or ProtocolIds.ErrorCodes.EitmadErrorContractInvalidV1 => MaterialFailureKind.Validation,
        ProtocolIds.ErrorCodes.EitmadErrorMaterialReferenceInvalidV1 => MaterialFailureKind.Reference,
        ProtocolIds.ErrorCodes.EitmadErrorMaterialRevisionConflictV1 => MaterialFailureKind.Conflict,
        ProtocolIds.ErrorCodes.EitmadErrorAuthorizationDeniedV1 => MaterialFailureKind.Denied,
        _ => MaterialFailureKind.Unavailable,
    };
}
