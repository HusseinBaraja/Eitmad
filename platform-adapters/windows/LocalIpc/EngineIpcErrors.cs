using Eitmad.Contracts;

namespace Eitmad.Platform.Windows.LocalIpc;

public enum EngineIpcFailureKind
{
    EngineUnavailable,
    Timeout,
    CommandOutcomeUnknown,
    ConnectionLost,
    PayloadTooLarge,
    AuthenticationRejected,
    VersionMismatch,
    ProtocolViolation,
    EngineStopping,
    SubscriptionUnsupported,
    ResyncRequired,
    SubscriptionBackpressure,
}

public sealed class EngineIpcException : Exception
{
    internal EngineIpcException(
        EngineIpcFailureKind kind,
        string message,
        ContractError? contractError = null,
        Exception? innerException = null)
        : base(message, innerException)
    {
        Kind = kind;
        ContractError = contractError;
    }

    public EngineIpcFailureKind Kind { get; }
    public ContractError? ContractError { get; }
}
