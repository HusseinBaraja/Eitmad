//! Typed local request/response transport for native shell adapters.

mod subscriptions;

pub use subscriptions::{
    EventBroker, FeedError, PublishError, PublishedEvent, SubscribeError, SubscriptionFeed,
};

use std::{collections::HashMap, io, sync::Arc, time::Duration};

use async_trait::async_trait;
use eitmad_contracts::{
    PROTOCOL_VERSION,
    commands::{Command, CommandResult},
    errors::{ContractError, ErrorCode, ErrorDetail, MessageId, RetryDisposition},
    events::Subscription,
    identity::{AuthorizationContext, SessionId},
    ipc::{
        HandshakeAccepted, HandshakeOutcome, HandshakeRejection, HandshakeRequest,
        HandshakeResponse, IpcClientMessage, IpcFailureResponse, IpcServerMessage,
        MAX_IPC_FRAME_BYTES, ShutdownResponse,
    },
    queries::{Query, QueryResult},
    transport::{
        CommandOutcome, CommandResponseEnvelope, CorrelationId, EventCursor, EventEnvelope,
        QueryOutcome, QueryResponseEnvelope, SubscriptionCloseReason, SubscriptionClosedEnvelope,
        SubscriptionEnvelope, SubscriptionId, SubscriptionOutcome, SubscriptionResponseEnvelope,
        UnixMillis, UnsubscribeResponse,
    },
    updates::ReleaseVersion,
    versioning::{
        NegotiatedSession, NegotiationOutcome, PeerHello, PeerKind, SupportedProtocol, negotiate,
    },
};
use serde::{Serialize, de::DeserializeOwned};
use subtle::ConstantTimeEq as _;
use tokio::{
    io::{AsyncRead, AsyncReadExt as _, AsyncWrite, AsyncWriteExt as _},
    sync::{mpsc, watch},
};

const MAX_ACTIVE_SUBSCRIPTIONS_PER_CONNECTION: usize = 64;

pub const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone, Debug)]
pub struct DispatchContext {
    pub authorization: AuthorizationContext,
    pub correlation_id: CorrelationId,
    pub deadline: UnixMillis,
}

#[async_trait]
pub trait CommandDispatcher: Send + Sync {
    async fn dispatch_command(
        &self,
        context: DispatchContext,
        command: Command,
    ) -> Result<CommandResult, ContractError>;
}

#[async_trait]
pub trait QueryDispatcher: Send + Sync {
    async fn dispatch_query(
        &self,
        context: DispatchContext,
        query: Query,
    ) -> Result<QueryResult, ContractError>;

    async fn authorize_subscription(
        &self,
        context: SubscriptionContext,
        _subscription: &Subscription,
    ) -> Result<(), ContractError> {
        Err(subscription_unsupported(context.correlation_id))
    }
}

#[derive(Clone, Debug)]
pub struct SubscriptionContext {
    pub authorization: AuthorizationContext,
    pub correlation_id: CorrelationId,
}

pub trait IpcDispatcher: CommandDispatcher + QueryDispatcher {}
impl<T: CommandDispatcher + QueryDispatcher> IpcDispatcher for T {}

pub struct RejectingDispatcher;

#[async_trait]
impl CommandDispatcher for RejectingDispatcher {
    async fn dispatch_command(
        &self,
        context: DispatchContext,
        _command: Command,
    ) -> Result<CommandResult, ContractError> {
        Err(contract_error(
            "eitmad.error.contract-invalid.v1",
            "eitmad.message.contract-invalid.v1",
            context.correlation_id,
            RetryDisposition::Never,
            None,
        ))
    }
}

#[async_trait]
impl QueryDispatcher for RejectingDispatcher {
    async fn dispatch_query(
        &self,
        context: DispatchContext,
        _query: Query,
    ) -> Result<QueryResult, ContractError> {
        Err(contract_error(
            "eitmad.error.contract-invalid.v1",
            "eitmad.message.contract-invalid.v1",
            context.correlation_id,
            RetryDisposition::Never,
            None,
        ))
    }
}

#[derive(Clone)]
pub struct LocalIpcConfiguration {
    pub pipe_name: String,
    pub development_bearer_token: Option<String>,
    pub engine_hello: PeerHello,
}

impl LocalIpcConfiguration {
    #[must_use]
    pub fn development(pipe_name: String, development_bearer_token: Option<String>) -> Self {
        Self {
            pipe_name,
            development_bearer_token,
            engine_hello: default_engine_hello(),
        }
    }
}

#[derive(Clone)]
struct Session {
    negotiated: NegotiatedSession,
    authorization: AuthorizationContext,
}

pub struct LocalIpcServer {
    configuration: LocalIpcConfiguration,
    dispatcher: Arc<dyn IpcDispatcher>,
    shutdown_requests: mpsc::Sender<()>,
    event_broker: EventBroker,
}

impl LocalIpcServer {
    #[must_use]
    pub fn new(
        configuration: LocalIpcConfiguration,
        dispatcher: Arc<dyn IpcDispatcher>,
        shutdown_requests: mpsc::Sender<()>,
    ) -> Self {
        Self {
            configuration,
            dispatcher,
            shutdown_requests,
            event_broker: EventBroker::new(),
        }
    }

    #[must_use]
    pub fn with_event_broker(mut self, event_broker: EventBroker) -> Self {
        self.event_broker = event_broker;
        self
    }

    /// Runs the Windows named-pipe accept loop until cancellation.
    ///
    /// # Errors
    ///
    /// Returns an I/O error when the pipe cannot be created or served.
    #[cfg(windows)]
    pub async fn run(self, mut cancellation: watch::Receiver<bool>) -> io::Result<()> {
        use tokio::net::windows::named_pipe::ServerOptions;

        let path = pipe_path(&self.configuration.pipe_name);
        let mut server = ServerOptions::new().create(&path)?;
        loop {
            tokio::select! {
                result = server.connect() => result?,
                result = cancellation.changed() => {
                    let _ = result;
                    return Ok(());
                }
            }
            let connected = server;
            server = ServerOptions::new().create(&path)?;
            let outcome = self.serve_connection(connected, cancellation.clone()).await;
            if *cancellation.borrow() {
                return Ok(());
            }
            if let Err(error) = outcome {
                if !matches!(
                    error.kind(),
                    io::ErrorKind::BrokenPipe
                        | io::ErrorKind::ConnectionReset
                        | io::ErrorKind::UnexpectedEof
                ) {
                    return Err(error);
                }
            }
        }
    }

    #[cfg(not(windows))]
    pub async fn run(self, _cancellation: watch::Receiver<bool>) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Windows named-pipe IPC is unavailable on this platform",
        ))
    }

    async fn serve_connection<S>(
        &self,
        stream: S,
        mut cancellation: watch::Receiver<bool>,
    ) -> io::Result<()>
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        let (mut reader, mut writer) = tokio::io::split(stream);
        let mut session = None;
        let mut pending = tokio::task::JoinSet::new();
        let (event_sender, mut event_receiver) = mpsc::channel::<SubscriptionDelivery>(256);
        let mut subscriptions = HashMap::new();
        loop {
            tokio::select! {
                biased;
                completed = pending.join_next(), if !pending.is_empty() => {
                    if let Some(Ok(response)) = completed {
                        if !write_frame_or_close(&mut writer, &response).await? {
                            return Ok(());
                        }
                    }
                }
                delivery = event_receiver.recv(), if !subscriptions.is_empty() => {
                    if !write_subscription_delivery(&mut writer, delivery, &mut subscriptions).await? {
                        return Ok(());
                    }
                }
                result = read_client_message(&mut reader) => {
                    let message = match result? {
                        ClientRead::Message(message) => message,
                        ClientRead::CloseWith(response) => {
                            let _ = write_frame_or_close(&mut writer, &response).await?;
                            return Ok(());
                        }
                    };

                    let Some(message) = spawn_request(
                        &mut pending,
                        &self.dispatcher,
                        session.as_ref(),
                        message,
                    ) else {
                        continue;
                    };

                    match message {
                        IpcClientMessage::Subscribe(request) if session.is_some() => {
                            let response = self.subscribe(
                                session.as_ref(),
                                request,
                                &event_sender,
                                &mut subscriptions,
                            ).await;
                            if !write_frame_or_close(&mut writer, &IpcServerMessage::Subscribe(response)).await? {
                                return Ok(());
                            }
                        }
                        IpcClientMessage::Unsubscribe(request) if session.is_some() => {
                            let accepted = if subscriptions.contains_key(&request.subscription_id) {
                                if !close_subscription(
                                    &mut writer,
                                    request.subscription_id,
                                    SubscriptionCloseReason::ClientRequested,
                                    &mut subscriptions,
                                ).await? {
                                    return Ok(());
                                }
                                true
                            } else {
                                false
                            };
                            let response = unsubscribe_response(&request, accepted);
                            if !write_frame_or_close(&mut writer, &response).await? {
                                return Ok(());
                            }
                        }
                        IpcClientMessage::Shutdown(request) => {
                            let subscription_ids = subscriptions.keys().copied().collect::<Vec<_>>();
                            for subscription_id in subscription_ids {
                                if !close_subscription(
                                    &mut writer,
                                    subscription_id,
                                    SubscriptionCloseReason::EngineStopping,
                                    &mut subscriptions,
                                ).await? {
                                    return Ok(());
                                }
                            }
                            while let Some(result) = pending.join_next().await {
                                if let Ok(response) = result {
                                    if !write_frame_or_close(&mut writer, &response).await? {
                                        return Ok(());
                                    }
                                }
                            }
                            let response = IpcServerMessage::Shutdown(ShutdownResponse {
                                request_id: request.request_id,
                                correlation_id: request.correlation_id,
                                accepted: session.is_some(),
                            });
                            if !write_frame_or_close(&mut writer, &response).await? {
                                return Ok(());
                            }
                            if session.is_some() {
                                let _ = self.shutdown_requests.send(()).await;
                            }
                            return Ok(());
                        }
                        other => {
                            let response = self.handle_message(other, &mut session).await;
                            if !write_frame_or_close(&mut writer, &response).await? {
                                return Ok(());
                            }
                        }
                    }
                }
                result = cancellation.changed() => {
                    let _ = result;
                    return Ok(());
                }
            }
        }
    }

    async fn handle_message(
        &self,
        message: IpcClientMessage,
        session: &mut Option<Session>,
    ) -> IpcServerMessage {
        match message {
            IpcClientMessage::Handshake(request) => {
                let response = self.handshake(request);
                if let HandshakeOutcome::Accepted(accepted) = &response.outcome {
                    *session = Some(Session {
                        negotiated: accepted.negotiated.clone(),
                        authorization: accepted.authorization.clone(),
                    });
                }
                IpcServerMessage::Handshake(response)
            }
            IpcClientMessage::Command(request) => {
                IpcServerMessage::Command(self.command(session.as_ref(), request).await)
            }
            IpcClientMessage::Query(request) => {
                IpcServerMessage::Query(self.query(session.as_ref(), request).await)
            }
            IpcClientMessage::Subscribe(request) => {
                IpcServerMessage::Subscribe(SubscriptionResponseEnvelope {
                    request_id: request.request_id,
                    correlation_id: request.correlation_id,
                    outcome: SubscriptionOutcome::Failed(session_invalid(request.correlation_id)),
                })
            }
            IpcClientMessage::Unsubscribe(request) => {
                IpcServerMessage::Unsubscribe(UnsubscribeResponse {
                    request_id: request.request_id,
                    correlation_id: request.correlation_id,
                    subscription_id: request.subscription_id,
                    accepted: false,
                })
            }
            IpcClientMessage::Shutdown(request) => IpcServerMessage::Shutdown(ShutdownResponse {
                request_id: request.request_id,
                correlation_id: request.correlation_id,
                accepted: session.is_some(),
            }),
        }
    }

    fn handshake(&self, request: HandshakeRequest) -> HandshakeResponse {
        let outcome = match &self.configuration.development_bearer_token {
            None => HandshakeOutcome::Rejected(HandshakeRejection::AuthenticationRequired),
            Some(expected) if !tokens_equal(expected, &request.development_bearer_token) => {
                HandshakeOutcome::Rejected(HandshakeRejection::AuthenticationFailed)
            }
            Some(_) => match negotiate(&self.configuration.engine_hello, &request.peer) {
                NegotiationOutcome::Accepted(negotiated) => {
                    let authorization = AuthorizationContext {
                        session_id: SessionId::new(uuid::Uuid::new_v4()),
                        identity: request.asserted_authorization.identity,
                        scope: request.asserted_authorization.scope,
                    };
                    HandshakeOutcome::Accepted(Box::new(HandshakeAccepted {
                        engine: self.configuration.engine_hello.clone(),
                        negotiated,
                        authorization,
                    }))
                }
                NegotiationOutcome::Rejected(rejection) => {
                    HandshakeOutcome::Rejected(HandshakeRejection::Negotiation(rejection))
                }
            },
        };
        HandshakeResponse {
            request_id: request.request_id,
            correlation_id: request.correlation_id,
            outcome,
        }
    }

    async fn command(
        &self,
        session: Option<&Session>,
        request: eitmad_contracts::transport::CommandEnvelope,
    ) -> CommandResponseEnvelope {
        dispatch_command(Arc::clone(&self.dispatcher), session, request).await
    }

    async fn query(
        &self,
        session: Option<&Session>,
        request: eitmad_contracts::transport::QueryEnvelope,
    ) -> QueryResponseEnvelope {
        dispatch_query(Arc::clone(&self.dispatcher), session, request).await
    }

    async fn subscribe(
        &self,
        session: Option<&Session>,
        request: SubscriptionEnvelope,
        deliveries: &mpsc::Sender<SubscriptionDelivery>,
        subscriptions: &mut HashMap<SubscriptionId, ActiveSubscription>,
    ) -> SubscriptionResponseEnvelope {
        let failure = validate_subscription(session, &request);
        let outcome = if let Some(error) = failure {
            SubscriptionOutcome::Failed(error)
        } else {
            let context = SubscriptionContext {
                authorization: request.authorization.clone(),
                correlation_id: request.correlation_id,
            };
            match self
                .dispatcher
                .authorize_subscription(context, &request.subscription)
                .await
            {
                Err(error) => SubscriptionOutcome::Failed(error),
                Ok(()) if subscriptions.len() >= MAX_ACTIVE_SUBSCRIPTIONS_PER_CONNECTION => {
                    SubscriptionOutcome::Failed(subscription_capacity_exceeded(
                        request.correlation_id,
                    ))
                }
                Ok(()) => {
                    match self.event_broker.subscribe(
                        request.authorization.scope,
                        request.subscription,
                        request.resume_after,
                    ) {
                        Err(SubscribeError::ResyncRequired) => SubscriptionOutcome::Failed(
                            subscription_resync_required(request.correlation_id),
                        ),
                        Ok((accepted, feed)) => {
                            let subscription_id = accepted.subscription_id;
                            let handle = tokio::spawn(pump_subscription(
                                feed,
                                subscription_id,
                                request.correlation_id,
                                deliveries.clone(),
                            ))
                            .abort_handle();
                            subscriptions.insert(
                                subscription_id,
                                ActiveSubscription {
                                    handle,
                                    correlation_id: request.correlation_id,
                                    last_delivered_cursor: None,
                                },
                            );
                            SubscriptionOutcome::Succeeded(accepted)
                        }
                    }
                }
            }
        };
        SubscriptionResponseEnvelope {
            request_id: request.request_id,
            correlation_id: request.correlation_id,
            outcome,
        }
    }
}

fn spawn_request(
    pending: &mut tokio::task::JoinSet<IpcServerMessage>,
    dispatcher: &Arc<dyn IpcDispatcher>,
    session: Option<&Session>,
    message: IpcClientMessage,
) -> Option<IpcClientMessage> {
    match message {
        IpcClientMessage::Command(request) if session.is_some() => {
            let dispatcher = Arc::clone(dispatcher);
            let active_session = session.cloned();
            pending.spawn(async move {
                IpcServerMessage::Command(
                    dispatch_command(dispatcher, active_session.as_ref(), request).await,
                )
            });
            None
        }
        IpcClientMessage::Query(request) if session.is_some() => {
            let dispatcher = Arc::clone(dispatcher);
            let active_session = session.cloned();
            pending.spawn(async move {
                IpcServerMessage::Query(
                    dispatch_query(dispatcher, active_session.as_ref(), request).await,
                )
            });
            None
        }
        other => Some(other),
    }
}

fn unsubscribe_response(
    request: &eitmad_contracts::transport::UnsubscribeRequest,
    accepted: bool,
) -> IpcServerMessage {
    IpcServerMessage::Unsubscribe(UnsubscribeResponse {
        request_id: request.request_id,
        correlation_id: request.correlation_id,
        subscription_id: request.subscription_id,
        accepted,
    })
}

struct ActiveSubscription {
    handle: tokio::task::AbortHandle,
    correlation_id: CorrelationId,
    last_delivered_cursor: Option<EventCursor>,
}

async fn close_subscription<W>(
    writer: &mut W,
    subscription_id: SubscriptionId,
    reason: SubscriptionCloseReason,
    subscriptions: &mut HashMap<SubscriptionId, ActiveSubscription>,
) -> io::Result<bool>
where
    W: AsyncWrite + Unpin,
{
    let Some(active) = subscriptions.get(&subscription_id) else {
        return Ok(true);
    };
    let message = IpcServerMessage::SubscriptionClosed(SubscriptionClosedEnvelope {
        subscription_id,
        correlation_id: active.correlation_id,
        last_delivered_cursor: active.last_delivered_cursor,
        reason,
    });
    let written = write_frame_or_close(writer, &message).await;
    if let Some(active) = subscriptions.remove(&subscription_id) {
        active.handle.abort();
    }
    written
}

async fn write_subscription_delivery<W>(
    writer: &mut W,
    delivery: Option<SubscriptionDelivery>,
    subscriptions: &mut HashMap<SubscriptionId, ActiveSubscription>,
) -> io::Result<bool>
where
    W: AsyncWrite + Unpin,
{
    let Some(delivery) = delivery else {
        return Ok(true);
    };
    if !delivery.closed && !subscriptions.contains_key(&delivery.subscription_id) {
        return Ok(true);
    }
    let written = write_frame_or_close(writer, &delivery.message).await?;
    if written {
        if let Some(cursor) = delivery.delivered_cursor {
            if let Some(active) = subscriptions.get_mut(&delivery.subscription_id) {
                active.last_delivered_cursor = Some(cursor);
            }
        }
        if delivery.closed {
            subscriptions.remove(&delivery.subscription_id);
        }
    }
    Ok(written)
}

struct SubscriptionDelivery {
    subscription_id: SubscriptionId,
    message: IpcServerMessage,
    closed: bool,
    delivered_cursor: Option<EventCursor>,
}

async fn pump_subscription(
    mut feed: SubscriptionFeed,
    subscription_id: SubscriptionId,
    correlation_id: CorrelationId,
    deliveries: mpsc::Sender<SubscriptionDelivery>,
) {
    let mut sequence = 0_u64;
    loop {
        match feed.recv().await {
            Ok(published) => {
                sequence = sequence.saturating_add(1);
                let message = IpcServerMessage::Event(EventEnvelope {
                    subscription_id,
                    correlation_id,
                    sequence,
                    cursor: published.cursor,
                    occurred_at: published.occurred_at,
                    event: published.event,
                });
                if deliveries
                    .send(SubscriptionDelivery {
                        subscription_id,
                        message,
                        closed: false,
                        delivered_cursor: Some(published.cursor),
                    })
                    .await
                    .is_err()
                {
                    return;
                }
            }
            Err(FeedError::Backpressure) => {
                let _ = deliveries
                    .send(SubscriptionDelivery {
                        subscription_id,
                        message: IpcServerMessage::SubscriptionClosed(SubscriptionClosedEnvelope {
                            subscription_id,
                            correlation_id,
                            last_delivered_cursor: Some(feed.last_cursor()),
                            reason: SubscriptionCloseReason::Backpressure,
                        }),
                        closed: true,
                        delivered_cursor: None,
                    })
                    .await;
                return;
            }
            Err(FeedError::Closed) => return,
        }
    }
}

async fn dispatch_command(
    dispatcher: Arc<dyn IpcDispatcher>,
    session: Option<&Session>,
    request: eitmad_contracts::transport::CommandEnvelope,
) -> CommandResponseEnvelope {
    let error = validate_request(
        session,
        &request.authorization,
        request.protocol_version,
        request.deadline,
        request.correlation_id,
    );
    let outcome = if let Some(error) = error {
        CommandOutcome::Failed(error)
    } else {
        let context = DispatchContext {
            authorization: request.authorization,
            correlation_id: request.correlation_id,
            deadline: request.deadline,
        };
        let remaining = duration_until(request.deadline);
        match tokio::time::timeout(
            remaining,
            dispatcher.dispatch_command(context, request.command),
        )
        .await
        {
            Ok(Ok(result)) => CommandOutcome::Succeeded(result),
            Ok(Err(error)) => CommandOutcome::Failed(error),
            Err(_) => {
                CommandOutcome::Failed(deadline_exceeded(request.correlation_id, request.deadline))
            }
        }
    };
    CommandResponseEnvelope {
        request_id: request.request_id,
        correlation_id: request.correlation_id,
        outcome,
    }
}

async fn dispatch_query(
    dispatcher: Arc<dyn IpcDispatcher>,
    session: Option<&Session>,
    request: eitmad_contracts::transport::QueryEnvelope,
) -> QueryResponseEnvelope {
    let error = validate_request(
        session,
        &request.authorization,
        request.protocol_version,
        request.deadline,
        request.correlation_id,
    );
    let outcome = if let Some(error) = error {
        QueryOutcome::Failed(error)
    } else {
        let context = DispatchContext {
            authorization: request.authorization,
            correlation_id: request.correlation_id,
            deadline: request.deadline,
        };
        let remaining = duration_until(request.deadline);
        match tokio::time::timeout(remaining, dispatcher.dispatch_query(context, request.query))
            .await
        {
            Ok(Ok(result)) => QueryOutcome::Succeeded(result),
            Ok(Err(error)) => QueryOutcome::Failed(error),
            Err(_) => {
                QueryOutcome::Failed(deadline_exceeded(request.correlation_id, request.deadline))
            }
        }
    };
    QueryResponseEnvelope {
        request_id: request.request_id,
        correlation_id: request.correlation_id,
        outcome,
    }
}

fn validate_request(
    session: Option<&Session>,
    authorization: &AuthorizationContext,
    protocol: eitmad_contracts::versioning::ProtocolVersion,
    deadline: UnixMillis,
    correlation_id: CorrelationId,
) -> Option<ContractError> {
    let Some(session) = session else {
        return Some(session_invalid(correlation_id));
    };
    if &session.authorization != authorization || session.negotiated.protocol != protocol {
        return Some(session_invalid(correlation_id));
    }
    (duration_until(deadline).is_zero()).then(|| deadline_exceeded(correlation_id, deadline))
}

fn validate_subscription(
    session: Option<&Session>,
    request: &SubscriptionEnvelope,
) -> Option<ContractError> {
    let Some(session) = session else {
        return Some(session_invalid(request.correlation_id));
    };
    let capability_negotiated = session.negotiated.protocol.minor >= 1
        && session.negotiated.capabilities.iter().any(|capability| {
            capability.as_str() == "eitmad.capability.local-ipc-subscriptions.v1"
        });
    if session.authorization != request.authorization
        || session.negotiated.protocol != request.protocol_version
    {
        Some(session_invalid(request.correlation_id))
    } else if !capability_negotiated {
        Some(subscription_unsupported(request.correlation_id))
    } else {
        None
    }
}

fn duration_until(deadline: UnixMillis) -> Duration {
    let remaining = deadline.0.saturating_sub(now().0);
    Duration::from_millis(u64::try_from(remaining).unwrap_or_default())
}

fn now() -> UnixMillis {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    UnixMillis(i64::try_from(millis).unwrap_or(i64::MAX))
}

fn tokens_equal(expected: &str, actual: &str) -> bool {
    expected.len() == actual.len() && bool::from(expected.as_bytes().ct_eq(actual.as_bytes()))
}

fn default_engine_hello() -> PeerHello {
    PeerHello {
        peer_kind: PeerKind::Engine,
        product_version: ReleaseVersion::new(
            semver::Version::parse(env!("CARGO_PKG_VERSION")).expect("package version is valid"),
        ),
        protocols: vec![SupportedProtocol {
            major: PROTOCOL_VERSION.major,
            minimum_minor: 0,
            maximum_minor: PROTOCOL_VERSION.minor,
        }],
        capabilities: vec![
            eitmad_contracts::transport::CapabilityId::parse("eitmad.capability.local-ipc.v1")
                .expect("static capability is valid"),
            eitmad_contracts::transport::CapabilityId::parse(
                "eitmad.capability.local-ipc-subscriptions.v1",
            )
            .expect("static capability is valid"),
        ],
        required_capabilities: Vec::new(),
        schemas: Vec::new(),
    }
}

fn session_invalid(correlation_id: CorrelationId) -> ContractError {
    contract_error(
        "eitmad.error.ipc-session-invalid.v1",
        "eitmad.message.ipc-session-invalid.v1",
        correlation_id,
        RetryDisposition::Never,
        None,
    )
}

fn deadline_exceeded(correlation_id: CorrelationId, deadline: UnixMillis) -> ContractError {
    contract_error(
        "eitmad.error.ipc-deadline-exceeded.v1",
        "eitmad.message.ipc-deadline-exceeded.v1",
        correlation_id,
        RetryDisposition::SafeImmediately,
        Some(ErrorDetail::Deadline { deadline }),
    )
}

fn payload_too_large(correlation_id: CorrelationId) -> ContractError {
    contract_error(
        "eitmad.error.ipc-payload-too-large.v1",
        "eitmad.message.ipc-payload-too-large.v1",
        correlation_id,
        RetryDisposition::Never,
        Some(ErrorDetail::PayloadLimit {
            maximum_bytes: MAX_IPC_FRAME_BYTES,
        }),
    )
}

fn subscription_unsupported(correlation_id: CorrelationId) -> ContractError {
    contract_error(
        "eitmad.error.ipc-subscription-unsupported.v1",
        "eitmad.message.ipc-subscription-unsupported.v1",
        correlation_id,
        RetryDisposition::Never,
        None,
    )
}

fn subscription_resync_required(correlation_id: CorrelationId) -> ContractError {
    contract_error(
        "eitmad.error.ipc-subscription-resync-required.v1",
        "eitmad.message.ipc-subscription-resync-required.v1",
        correlation_id,
        RetryDisposition::SafeImmediately,
        None,
    )
}

fn subscription_capacity_exceeded(correlation_id: CorrelationId) -> ContractError {
    contract_error(
        "eitmad.error.ipc-subscription-capacity-exceeded.v1",
        "eitmad.message.ipc-subscription-capacity-exceeded.v1",
        correlation_id,
        RetryDisposition::Never,
        None,
    )
}

fn contract_error(
    code: &str,
    message_id: &str,
    correlation_id: CorrelationId,
    retry: RetryDisposition,
    detail: Option<ErrorDetail>,
) -> ContractError {
    ContractError {
        code: ErrorCode::parse(code).expect("static error code is valid"),
        message_id: MessageId::parse(message_id).expect("static message ID is valid"),
        parameters: Vec::new(),
        retry,
        correlation_id,
        detail,
    }
}

#[cfg(windows)]
fn pipe_path(name: &str) -> String {
    format!(r"\\.\pipe\{name}")
}

#[derive(Debug)]
enum FrameReadError {
    Io(io::Error),
    PayloadTooLarge,
    InvalidJson,
}

enum ClientRead {
    Message(IpcClientMessage),
    CloseWith(IpcServerMessage),
}

async fn read_client_message<R>(reader: &mut R) -> io::Result<ClientRead>
where
    R: AsyncRead + Unpin,
{
    match read_frame(reader).await {
        Ok(message) => Ok(ClientRead::Message(message)),
        Err(FrameReadError::Io(error)) => Err(error),
        Err(FrameReadError::PayloadTooLarge) => Ok(ClientRead::CloseWith(
            IpcServerMessage::Failure(IpcFailureResponse {
                request_id: None,
                error: payload_too_large(CorrelationId::new(uuid::Uuid::new_v4())),
            }),
        )),
        Err(FrameReadError::InvalidJson) => Ok(ClientRead::CloseWith(IpcServerMessage::Failure(
            IpcFailureResponse {
                request_id: None,
                error: contract_error(
                    "eitmad.error.contract-invalid.v1",
                    "eitmad.message.contract-invalid.v1",
                    CorrelationId::new(uuid::Uuid::new_v4()),
                    RetryDisposition::Never,
                    None,
                ),
            },
        ))),
    }
}

async fn read_frame<R, T>(reader: &mut R) -> Result<T, FrameReadError>
where
    R: AsyncRead + Unpin,
    T: DeserializeOwned,
{
    let length = reader.read_u32_le().await.map_err(FrameReadError::Io)?;
    if length > MAX_IPC_FRAME_BYTES {
        return Err(FrameReadError::PayloadTooLarge);
    }
    let mut payload = vec![0; length as usize];
    reader
        .read_exact(&mut payload)
        .await
        .map_err(FrameReadError::Io)?;
    serde_json::from_slice(&payload).map_err(|_| FrameReadError::InvalidJson)
}

async fn write_frame<W, T>(writer: &mut W, value: &T) -> io::Result<()>
where
    W: AsyncWrite + Unpin,
    T: Serialize,
{
    let payload = serde_json::to_vec(value).map_err(io::Error::other)?;
    let length = u32::try_from(payload.len())
        .ok()
        .filter(|length| *length <= MAX_IPC_FRAME_BYTES)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "IPC payload exceeds limit"))?;
    writer.write_u32_le(length).await?;
    writer.write_all(&payload).await?;
    writer.flush().await
}

async fn write_frame_or_close<W, T>(writer: &mut W, value: &T) -> io::Result<bool>
where
    W: AsyncWrite + Unpin,
    T: Serialize,
{
    match write_frame(writer, value).await {
        Ok(()) => Ok(true),
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::InvalidData | io::ErrorKind::Other
            ) =>
        {
            Ok(false)
        }
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eitmad_contracts::{
        commands::{CancelOperation, Command},
        config::{ConfigReadValue, ConfigSnapshot},
        events::{ConfigurationChanges, Subscription},
        identity::{
            AuthenticatedIdentity, DeviceId, PrincipalId, PrincipalKind, ScopeId, ScopeKind,
            ScopeRef,
        },
        ipc::DevelopmentIdentityAssertion,
        queries::GetSyncStatus,
        sync::SyncStatus,
        transport::{IdempotencyKey, OperationId, RequestId, SubscriptionEnvelope},
        versioning::ProtocolVersion,
    };

    struct TestDispatcher;
    struct SlowDispatcher;

    #[async_trait]
    impl CommandDispatcher for TestDispatcher {
        async fn dispatch_command(
            &self,
            _context: DispatchContext,
            command: Command,
        ) -> Result<CommandResult, ContractError> {
            let Command::CancelOperation(request) = command else {
                unreachable!("unexpected command fixture")
            };
            Ok(CommandResult::OperationCancelled {
                operation_id: request.operation_id,
            })
        }
    }

    #[async_trait]
    impl QueryDispatcher for TestDispatcher {
        async fn dispatch_query(
            &self,
            _context: DispatchContext,
            _query: Query,
        ) -> Result<QueryResult, ContractError> {
            Ok(QueryResult::SyncStatus(SyncStatus::Offline))
        }

        async fn authorize_subscription(
            &self,
            _context: SubscriptionContext,
            _subscription: &Subscription,
        ) -> Result<(), ContractError> {
            Ok(())
        }
    }

    #[async_trait]
    impl CommandDispatcher for SlowDispatcher {
        async fn dispatch_command(
            &self,
            _context: DispatchContext,
            command: Command,
        ) -> Result<CommandResult, ContractError> {
            let Command::CancelOperation(request) = command else {
                unreachable!("unexpected command fixture")
            };
            tokio::time::sleep(Duration::from_millis(50)).await;
            Ok(CommandResult::OperationCancelled {
                operation_id: request.operation_id,
            })
        }
    }

    #[async_trait]
    impl QueryDispatcher for SlowDispatcher {
        async fn dispatch_query(
            &self,
            _context: DispatchContext,
            _query: Query,
        ) -> Result<QueryResult, ContractError> {
            tokio::time::sleep(Duration::from_millis(50)).await;
            Ok(QueryResult::SyncStatus(SyncStatus::Offline))
        }
    }

    fn assertion() -> DevelopmentIdentityAssertion {
        DevelopmentIdentityAssertion {
            identity: AuthenticatedIdentity {
                principal_id: PrincipalId::new(uuid::Uuid::new_v4()),
                principal_kind: PrincipalKind::User,
                device_id: Some(DeviceId::new(uuid::Uuid::new_v4())),
                service_id: None,
            },
            scope: ScopeRef {
                kind: ScopeKind::parse("organization").unwrap(),
                id: ScopeId::new(uuid::Uuid::new_v4()),
            },
        }
    }

    fn handshake(protocol: ProtocolVersion, token: &str) -> HandshakeRequest {
        let mut peer = default_engine_hello();
        peer.peer_kind = PeerKind::Shell;
        peer.protocols = vec![SupportedProtocol {
            major: protocol.major,
            minimum_minor: protocol.minor,
            maximum_minor: protocol.minor,
        }];
        HandshakeRequest {
            request_id: RequestId::new(uuid::Uuid::new_v4()),
            correlation_id: CorrelationId::new(uuid::Uuid::new_v4()),
            peer,
            development_bearer_token: token.to_owned(),
            asserted_authorization: assertion(),
        }
    }

    fn service(token: Option<&str>) -> LocalIpcServer {
        let (shutdown, _) = mpsc::channel(1);
        LocalIpcServer::new(
            LocalIpcConfiguration::development("test".to_owned(), token.map(ToOwned::to_owned)),
            Arc::new(TestDispatcher),
            shutdown,
        )
    }

    #[test]
    fn handshake_requires_explicit_development_authentication() {
        let response = service(None).handshake(handshake(PROTOCOL_VERSION, "token"));
        assert!(matches!(
            response.outcome,
            HandshakeOutcome::Rejected(HandshakeRejection::AuthenticationRequired)
        ));
    }

    #[test]
    fn handshake_rejects_version_mismatch() {
        let response = service(Some("token"))
            .handshake(handshake(ProtocolVersion { major: 2, minor: 0 }, "token"));
        assert!(matches!(
            response.outcome,
            HandshakeOutcome::Rejected(HandshakeRejection::Negotiation(_))
        ));
    }

    #[tokio::test]
    async fn query_dispatches_after_session_handshake() {
        let service = service(Some("token"));
        let response = service.handshake(handshake(PROTOCOL_VERSION, "token"));
        let HandshakeOutcome::Accepted(accepted) = response.outcome else {
            panic!("handshake should succeed");
        };
        let session = Session {
            negotiated: accepted.negotiated,
            authorization: accepted.authorization.clone(),
        };
        let request = eitmad_contracts::transport::QueryEnvelope {
            protocol_version: PROTOCOL_VERSION,
            request_id: RequestId::new(uuid::Uuid::new_v4()),
            correlation_id: CorrelationId::new(uuid::Uuid::new_v4()),
            causation_id: None,
            authorization: accepted.authorization,
            deadline: UnixMillis(now().0 + 1_000),
            query: Query::SyncStatus(GetSyncStatus {}),
        };
        assert!(matches!(
            service.query(Some(&session), request).await.outcome,
            QueryOutcome::Succeeded(QueryResult::SyncStatus(SyncStatus::Offline))
        ));
    }

    #[tokio::test]
    async fn subscription_delivers_scoped_events_after_acceptance() {
        let service = service(Some("token"));
        let response = service.handshake(handshake(PROTOCOL_VERSION, "token"));
        let HandshakeOutcome::Accepted(accepted) = response.outcome else {
            panic!("handshake should succeed");
        };
        let session = Session {
            negotiated: accepted.negotiated,
            authorization: accepted.authorization.clone(),
        };
        let scope = accepted.authorization.scope.clone();
        let correlation_id = CorrelationId::new(uuid::Uuid::new_v4());
        let request = SubscriptionEnvelope {
            protocol_version: PROTOCOL_VERSION,
            request_id: RequestId::new(uuid::Uuid::new_v4()),
            correlation_id,
            authorization: accepted.authorization,
            subscription: Subscription::Configuration(ConfigurationChanges {}),
            resume_after: None,
        };
        let (sender, mut receiver) = mpsc::channel(4);
        let mut subscriptions = HashMap::new();
        let response = service
            .subscribe(Some(&session), request, &sender, &mut subscriptions)
            .await;
        let SubscriptionOutcome::Succeeded(accepted) = response.outcome else {
            panic!("subscription should succeed");
        };

        service
            .event_broker
            .publish(
                scope.clone(),
                eitmad_contracts::events::Event::ConfigurationChanged(ConfigSnapshot {
                    schema_version: 1,
                    revision: 1,
                    scope,
                    entries: Vec::new(),
                }),
            )
            .unwrap();
        let delivery = receiver.recv().await.expect("event delivery");
        let IpcServerMessage::Event(event) = delivery.message else {
            panic!("event message expected");
        };
        assert_eq!(event.subscription_id, accepted.subscription_id);
        assert_eq!(event.sequence, 1);
        subscriptions
            .remove(&accepted.subscription_id)
            .unwrap()
            .handle
            .abort();
    }

    #[tokio::test]
    async fn subscription_close_reasons_are_sent_before_abort() {
        for reason in [
            SubscriptionCloseReason::ClientRequested,
            SubscriptionCloseReason::EngineStopping,
        ] {
            let subscription_id = SubscriptionId::new(uuid::Uuid::new_v4());
            let correlation_id = CorrelationId::new(uuid::Uuid::new_v4());
            let cursor = EventCursor::new(uuid::Uuid::new_v4());
            let task = tokio::spawn(std::future::pending::<()>());
            let mut subscriptions = HashMap::from([(
                subscription_id,
                ActiveSubscription {
                    handle: task.abort_handle(),
                    correlation_id,
                    last_delivered_cursor: Some(cursor),
                },
            )]);
            let (mut writer, mut reader) = tokio::io::duplex(1_024);

            assert!(
                close_subscription(&mut writer, subscription_id, reason, &mut subscriptions,)
                    .await
                    .unwrap()
            );
            let message = read_frame::<_, IpcServerMessage>(&mut reader)
                .await
                .unwrap();
            let IpcServerMessage::SubscriptionClosed(closed) = message else {
                panic!("subscription close expected");
            };
            assert_eq!(closed.subscription_id, subscription_id);
            assert_eq!(closed.correlation_id, correlation_id);
            assert_eq!(closed.last_delivered_cursor, Some(cursor));
            assert_eq!(closed.reason, reason);
            assert!(task.await.unwrap_err().is_cancelled());
        }
    }

    #[tokio::test]
    async fn connection_rejects_subscriptions_at_capacity() {
        let service = service(Some("token"));
        let response = service.handshake(handshake(PROTOCOL_VERSION, "token"));
        let HandshakeOutcome::Accepted(accepted) = response.outcome else {
            panic!("handshake should succeed");
        };
        let session = Session {
            negotiated: accepted.negotiated,
            authorization: accepted.authorization.clone(),
        };
        let request = SubscriptionEnvelope {
            protocol_version: PROTOCOL_VERSION,
            request_id: RequestId::new(uuid::Uuid::new_v4()),
            correlation_id: CorrelationId::new(uuid::Uuid::new_v4()),
            authorization: accepted.authorization,
            subscription: Subscription::Configuration(ConfigurationChanges {}),
            resume_after: None,
        };
        let task = tokio::spawn(std::future::pending::<()>());
        let abort = task.abort_handle();
        let mut subscriptions = (0..MAX_ACTIVE_SUBSCRIPTIONS_PER_CONNECTION)
            .map(|_| {
                (
                    SubscriptionId::new(uuid::Uuid::new_v4()),
                    ActiveSubscription {
                        handle: abort.clone(),
                        correlation_id: CorrelationId::new(uuid::Uuid::new_v4()),
                        last_delivered_cursor: None,
                    },
                )
            })
            .collect::<HashMap<_, _>>();
        let (sender, _) = mpsc::channel(1);

        let response = service
            .subscribe(Some(&session), request, &sender, &mut subscriptions)
            .await;
        let SubscriptionOutcome::Failed(error) = response.outcome else {
            panic!("capacity failure expected");
        };
        assert_eq!(
            error.code.as_str(),
            "eitmad.error.ipc-subscription-capacity-exceeded.v1"
        );
        assert_eq!(subscriptions.len(), MAX_ACTIVE_SUBSCRIPTIONS_PER_CONNECTION);
        task.abort();
    }

    #[test]
    fn protocol_1_0_negotiates_without_subscription_support() {
        let service = service(Some("token"));
        let response =
            service.handshake(handshake(ProtocolVersion { major: 1, minor: 0 }, "token"));
        let HandshakeOutcome::Accepted(accepted) = response.outcome else {
            panic!("protocol 1.0 should remain compatible");
        };
        let session = Session {
            negotiated: accepted.negotiated,
            authorization: accepted.authorization.clone(),
        };
        let request = SubscriptionEnvelope {
            protocol_version: ProtocolVersion { major: 1, minor: 0 },
            request_id: RequestId::new(uuid::Uuid::new_v4()),
            correlation_id: CorrelationId::new(uuid::Uuid::new_v4()),
            authorization: accepted.authorization,
            subscription: Subscription::Configuration(ConfigurationChanges {}),
            resume_after: None,
        };
        assert_eq!(
            validate_subscription(Some(&session), &request)
                .unwrap()
                .code
                .as_str(),
            "eitmad.error.ipc-subscription-unsupported.v1"
        );
    }

    #[tokio::test]
    async fn command_dispatches_after_session_handshake() {
        let service = service(Some("token"));
        let response = service.handshake(handshake(PROTOCOL_VERSION, "token"));
        let HandshakeOutcome::Accepted(accepted) = response.outcome else {
            panic!("handshake should succeed");
        };
        let session = Session {
            negotiated: accepted.negotiated,
            authorization: accepted.authorization.clone(),
        };
        let operation_id = OperationId::new(uuid::Uuid::new_v4());
        let request = eitmad_contracts::transport::CommandEnvelope {
            protocol_version: PROTOCOL_VERSION,
            request_id: RequestId::new(uuid::Uuid::new_v4()),
            correlation_id: CorrelationId::new(uuid::Uuid::new_v4()),
            causation_id: None,
            authorization: accepted.authorization,
            deadline: UnixMillis(now().0 + 1_000),
            idempotency_key: IdempotencyKey::new(uuid::Uuid::new_v4()),
            command: Command::CancelOperation(CancelOperation { operation_id }),
        };
        assert!(matches!(
            service.command(Some(&session), request).await.outcome,
            CommandOutcome::Succeeded(CommandResult::OperationCancelled { operation_id: actual })
                if actual == operation_id
        ));
    }

    #[tokio::test]
    async fn pre_handshake_query_fails_structurally() {
        let authorization = AuthorizationContext {
            session_id: SessionId::new(uuid::Uuid::new_v4()),
            identity: assertion().identity,
            scope: assertion().scope,
        };
        let request = eitmad_contracts::transport::QueryEnvelope {
            protocol_version: PROTOCOL_VERSION,
            request_id: RequestId::new(uuid::Uuid::new_v4()),
            correlation_id: CorrelationId::new(uuid::Uuid::new_v4()),
            causation_id: None,
            authorization,
            deadline: UnixMillis(now().0 + 1_000),
            query: Query::SyncStatus(GetSyncStatus {}),
        };
        let QueryOutcome::Failed(error) = service(Some("token")).query(None, request).await.outcome
        else {
            panic!("query should fail");
        };
        assert_eq!(error.code.as_str(), "eitmad.error.ipc-session-invalid.v1");
    }

    #[tokio::test]
    async fn query_dispatch_is_cancelled_at_deadline() {
        let (shutdown, _) = mpsc::channel(1);
        let service = LocalIpcServer::new(
            LocalIpcConfiguration::development("test".to_owned(), Some("token".to_owned())),
            Arc::new(SlowDispatcher),
            shutdown,
        );
        let response = service.handshake(handshake(PROTOCOL_VERSION, "token"));
        let HandshakeOutcome::Accepted(accepted) = response.outcome else {
            panic!("handshake should succeed");
        };
        let session = Session {
            negotiated: accepted.negotiated,
            authorization: accepted.authorization.clone(),
        };
        let request = eitmad_contracts::transport::QueryEnvelope {
            protocol_version: PROTOCOL_VERSION,
            request_id: RequestId::new(uuid::Uuid::new_v4()),
            correlation_id: CorrelationId::new(uuid::Uuid::new_v4()),
            causation_id: None,
            authorization: accepted.authorization,
            deadline: UnixMillis(now().0 + 10),
            query: Query::SyncStatus(GetSyncStatus {}),
        };
        let QueryOutcome::Failed(error) = service.query(Some(&session), request).await.outcome
        else {
            panic!("query should time out");
        };
        assert_eq!(error.code.as_str(), "eitmad.error.ipc-deadline-exceeded.v1");
    }

    #[tokio::test]
    async fn command_dispatch_is_cancelled_at_deadline() {
        let (shutdown, _) = mpsc::channel(1);
        let service = LocalIpcServer::new(
            LocalIpcConfiguration::development("test".to_owned(), Some("token".to_owned())),
            Arc::new(SlowDispatcher),
            shutdown,
        );
        let response = service.handshake(handshake(PROTOCOL_VERSION, "token"));
        let HandshakeOutcome::Accepted(accepted) = response.outcome else {
            panic!("handshake should succeed");
        };
        let session = Session {
            negotiated: accepted.negotiated,
            authorization: accepted.authorization.clone(),
        };
        let request = eitmad_contracts::transport::CommandEnvelope {
            protocol_version: PROTOCOL_VERSION,
            request_id: RequestId::new(uuid::Uuid::new_v4()),
            correlation_id: CorrelationId::new(uuid::Uuid::new_v4()),
            causation_id: None,
            authorization: accepted.authorization,
            deadline: UnixMillis(now().0 + 10),
            idempotency_key: IdempotencyKey::new(uuid::Uuid::new_v4()),
            command: Command::CancelOperation(CancelOperation {
                operation_id: OperationId::new(uuid::Uuid::new_v4()),
            }),
        };
        let CommandOutcome::Failed(error) = service.command(Some(&session), request).await.outcome
        else {
            panic!("command should time out");
        };
        assert_eq!(error.code.as_str(), "eitmad.error.ipc-deadline-exceeded.v1");
    }

    #[tokio::test]
    async fn large_mixed_direction_payload_round_trips() {
        let sample = "خزانة Wardrobe 120 cm - فرع صنعاء".repeat(120_000);
        let value = ConfigReadValue::Text(sample);
        let (mut left, mut right) = tokio::io::duplex(5 * 1024 * 1024);
        let write = tokio::spawn(async move { write_frame(&mut left, &value).await });
        let decoded = read_frame::<_, ConfigReadValue>(&mut right).await.unwrap();
        write.await.unwrap().unwrap();
        assert!(matches!(decoded, ConfigReadValue::Text(text) if text.starts_with("خزانة")));
    }

    #[tokio::test]
    async fn oversized_outbound_payload_closes_only_its_connection() {
        let value = "x".repeat(MAX_IPC_FRAME_BYTES as usize + 1);
        let (mut writer, _) = tokio::io::duplex(16);

        assert!(!write_frame_or_close(&mut writer, &value).await.unwrap());
    }

    #[tokio::test]
    async fn oversized_frame_is_rejected_before_allocation() {
        let (mut left, mut right) = tokio::io::duplex(16);
        left.write_u32_le(MAX_IPC_FRAME_BYTES + 1).await.unwrap();
        assert!(matches!(
            read_frame::<_, IpcClientMessage>(&mut right).await,
            Err(FrameReadError::PayloadTooLarge)
        ));
    }
}
