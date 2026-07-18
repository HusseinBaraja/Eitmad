//! Scoped, bounded event replay for local IPC subscriptions.

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use eitmad_contracts::{
    events::{Event, Subscription},
    identity::ScopeRef,
    transport::{EventCursor, SubscriptionAccepted, SubscriptionId, UnixMillis},
};
use tokio::sync::broadcast;
use uuid::Uuid;

pub const MAX_REPLAY_EVENTS: usize = 1_024;
pub const MAX_REPLAY_BYTES: usize = 16 * 1024 * 1024;
const LIVE_CHANNEL_EVENTS: usize = 256;

#[derive(Clone, Debug)]
pub struct PublishedEvent {
    pub cursor: EventCursor,
    pub scope: ScopeRef,
    pub occurred_at: UnixMillis,
    pub event: Event,
}

#[derive(Clone)]
pub struct EventBroker {
    inner: Arc<BrokerInner>,
}

struct BrokerInner {
    state: Mutex<BrokerState>,
    live: broadcast::Sender<PublishedEvent>,
    policy_changes: broadcast::Sender<ScopeRef>,
}

#[derive(Default)]
struct BrokerState {
    entries: VecDeque<ReplayEntry>,
    encoded_bytes: usize,
}

#[derive(Clone)]
struct ReplayEntry {
    cursor: EventCursor,
    scope: ScopeRef,
    stream_kind: &'static str,
    encoded_bytes: usize,
    event: Option<PublishedEvent>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubscribeError {
    ResyncRequired,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PublishError {
    ScopeMismatch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FeedError {
    Backpressure,
    Closed,
}

pub struct SubscriptionFeed {
    broker: EventBroker,
    scope: ScopeRef,
    subscription: Subscription,
    replay: VecDeque<PublishedEvent>,
    live: broadcast::Receiver<PublishedEvent>,
    last_cursor: EventCursor,
}

impl Default for EventBroker {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBroker {
    #[must_use]
    pub fn new() -> Self {
        let (live, _) = broadcast::channel(LIVE_CHANNEL_EVENTS);
        let (policy_changes, _) = broadcast::channel(LIVE_CHANNEL_EVENTS);
        Self {
            inner: Arc::new(BrokerInner {
                state: Mutex::new(BrokerState::default()),
                live,
                policy_changes,
            }),
        }
    }

    /// Signals active subscriptions to reauthorize against committed policy.
    pub fn policy_changed(&self, scope: ScopeRef) {
        let _ = self.inner.policy_changes.send(scope);
    }

    pub(crate) fn subscribe_policy_changes(&self) -> broadcast::Receiver<ScopeRef> {
        self.inner.policy_changes.subscribe()
    }

    /// Publishes one already-authorized, scoped engine event.
    ///
    /// Replay eviction never blocks the producing vertical. Slow subscribers
    /// recover from retained cursors or receive an explicit backpressure close.
    ///
    /// # Errors
    ///
    /// Returns [`PublishError::ScopeMismatch`] when a scoped payload disagrees
    /// with the publisher's authorized scope.
    ///
    /// # Panics
    ///
    /// Panics when another broker operation poisoned the internal lock.
    pub fn publish(&self, scope: ScopeRef, event: Event) -> Result<PublishedEvent, PublishError> {
        if event_scope(&event).is_some_and(|event_scope| event_scope != &scope) {
            return Err(PublishError::ScopeMismatch);
        }
        let published = PublishedEvent {
            cursor: EventCursor::new(Uuid::new_v4()),
            scope: scope.clone(),
            occurred_at: now(),
            event,
        };
        let encoded_bytes = serde_json::to_vec(&published.event).map_or(0, |encoded| encoded.len());
        let mut state = self.inner.state.lock().expect("event broker lock");
        state.push(ReplayEntry {
            cursor: published.cursor,
            scope,
            stream_kind: published.event.subscription_kind(),
            encoded_bytes,
            event: Some(published.clone()),
        });
        let _ = self.inner.live.send(published.clone());
        Ok(published)
    }

    /// Creates an atomic replay-to-live feed for one authorized scope.
    ///
    /// # Errors
    ///
    /// Returns [`SubscribeError::ResyncRequired`] when the cursor is expired,
    /// belongs to another scope, or belongs to another subscription kind.
    ///
    /// # Panics
    ///
    /// Panics when another broker operation poisoned the internal lock.
    pub fn subscribe(
        &self,
        scope: ScopeRef,
        subscription: Subscription,
        resume_after: Option<EventCursor>,
    ) -> Result<(SubscriptionAccepted, SubscriptionFeed), SubscribeError> {
        let mut state = self.inner.state.lock().expect("event broker lock");
        let live = self.inner.live.subscribe();
        let stream_kind = subscription.kind();
        let (stream_cursor, resumed, replay) = if let Some(cursor) = resume_after {
            let Some(position) = state
                .entries
                .iter()
                .position(|entry| entry.cursor == cursor)
            else {
                return Err(SubscribeError::ResyncRequired);
            };
            let entry = &state.entries[position];
            if entry.scope != scope || entry.stream_kind != stream_kind {
                return Err(SubscribeError::ResyncRequired);
            }
            let replay = state
                .entries
                .iter()
                .skip(position + 1)
                .filter(|entry| entry.scope == scope && entry.stream_kind == stream_kind)
                .filter_map(|entry| entry.event.clone())
                .collect();
            (cursor, true, replay)
        } else {
            let cursor = EventCursor::new(Uuid::new_v4());
            state.push(ReplayEntry {
                cursor,
                scope: scope.clone(),
                stream_kind,
                encoded_bytes: 0,
                event: None,
            });
            (cursor, false, VecDeque::new())
        };
        let subscription_id = SubscriptionId::new(Uuid::new_v4());
        Ok((
            SubscriptionAccepted {
                subscription_id,
                stream_cursor,
                resumed,
            },
            SubscriptionFeed {
                broker: self.clone(),
                scope,
                subscription,
                replay,
                live,
                last_cursor: stream_cursor,
            },
        ))
    }

    fn recover(
        &self,
        scope: &ScopeRef,
        subscription: &Subscription,
        after: EventCursor,
    ) -> Result<
        (
            VecDeque<PublishedEvent>,
            broadcast::Receiver<PublishedEvent>,
        ),
        FeedError,
    > {
        let state = self.inner.state.lock().expect("event broker lock");
        let live = self.inner.live.subscribe();
        let stream_kind = subscription.kind();
        let Some(position) = state.entries.iter().position(|entry| entry.cursor == after) else {
            if subscription.is_coalescible() {
                return Ok((
                    state
                        .entries
                        .iter()
                        .rev()
                        .find(|entry| entry.scope == *scope && entry.stream_kind == stream_kind)
                        .and_then(|entry| entry.event.clone())
                        .into_iter()
                        .collect(),
                    live,
                ));
            }
            return Err(FeedError::Backpressure);
        };
        Ok((
            state
                .entries
                .iter()
                .skip(position + 1)
                .filter(|entry| entry.scope == *scope && entry.stream_kind == stream_kind)
                .filter_map(|entry| entry.event.clone())
                .collect(),
            live,
        ))
    }
}

impl BrokerState {
    fn push(&mut self, entry: ReplayEntry) {
        self.encoded_bytes = self.encoded_bytes.saturating_add(entry.encoded_bytes);
        self.entries.push_back(entry);
        while self.entries.len() > MAX_REPLAY_EVENTS || self.encoded_bytes > MAX_REPLAY_BYTES {
            if let Some(evicted) = self.entries.pop_front() {
                self.encoded_bytes = self.encoded_bytes.saturating_sub(evicted.encoded_bytes);
            }
        }
    }
}

impl SubscriptionFeed {
    /// Receives the next replayed or live event.
    ///
    /// # Errors
    ///
    /// Returns [`FeedError::Backpressure`] when a discrete gap is no longer
    /// replayable, or [`FeedError::Closed`] when the broker shuts down.
    pub async fn recv(&mut self) -> Result<PublishedEvent, FeedError> {
        loop {
            if let Some(event) = self.replay.pop_front() {
                self.last_cursor = event.cursor;
                return Ok(event);
            }
            match self.live.recv().await {
                Ok(event)
                    if event.scope == self.scope
                        && event.event.subscription_kind() == self.subscription.kind() =>
                {
                    self.last_cursor = event.cursor;
                    return Ok(event);
                }
                Ok(_) => {}
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    let (replay, live) =
                        self.broker
                            .recover(&self.scope, &self.subscription, self.last_cursor)?;
                    self.live = live;
                    self.replay = replay;
                }
                Err(broadcast::error::RecvError::Closed) => return Err(FeedError::Closed),
            }
        }
    }

    #[must_use]
    pub const fn last_cursor(&self) -> EventCursor {
        self.last_cursor
    }
}

fn now() -> UnixMillis {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    UnixMillis(i64::try_from(millis).unwrap_or(i64::MAX))
}

fn event_scope(event: &Event) -> Option<&ScopeRef> {
    match event {
        Event::ConfigurationChanged(snapshot) => Some(&snapshot.scope),
        Event::RecordChanged(notice) => Some(&notice.scope),
        Event::BackgroundJobChanged(status) => Some(&status.scope),
        Event::NotificationRaised(notification) => Some(&notification.scope),
        Event::ErrorRaised(error) => Some(&error.scope),
        Event::AuthorizationPolicyChanged(notice) => Some(&notice.scope),
        Event::PermissionsChanged(_)
        | Event::UpdateStateChanged(_)
        | Event::SyncStatusChanged(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use eitmad_contracts::{
        config::ConfigSnapshot,
        events::{ConfigurationChanges, Notifications, SyncStatusChanges},
        identity::{ScopeId, ScopeKind},
        notifications::{Notification, NotificationId, NotificationSeverity},
        sync::SyncStatus,
        transport::CorrelationId,
    };

    use super::*;

    fn scope(id: u128) -> ScopeRef {
        ScopeRef {
            kind: ScopeKind::parse("organization").unwrap(),
            id: ScopeId::new(Uuid::from_u128(id)),
        }
    }

    fn config_event(scope: &ScopeRef, revision: u64) -> Event {
        Event::ConfigurationChanged(ConfigSnapshot {
            schema_version: 1,
            revision,
            scope: scope.clone(),
            entries: Vec::new(),
        })
    }

    #[tokio::test]
    async fn resumes_in_order_without_crossing_scopes() {
        let broker = EventBroker::new();
        let first_scope = scope(1);
        let second_scope = scope(2);
        let (_, mut initial) = broker
            .subscribe(
                first_scope.clone(),
                Subscription::Configuration(ConfigurationChanges {}),
                None,
            )
            .unwrap();
        let first = broker
            .publish(first_scope.clone(), config_event(&first_scope, 1))
            .unwrap();
        broker
            .publish(second_scope.clone(), config_event(&second_scope, 9))
            .unwrap();
        let second = broker
            .publish(first_scope.clone(), config_event(&first_scope, 2))
            .unwrap();

        assert_eq!(initial.recv().await.unwrap().cursor, first.cursor);
        let (_, mut resumed) = broker
            .subscribe(
                first_scope,
                Subscription::Configuration(ConfigurationChanges {}),
                Some(first.cursor),
            )
            .unwrap();
        assert_eq!(resumed.recv().await.unwrap().cursor, second.cursor);
        assert!(matches!(
            broker.subscribe(
                second_scope,
                Subscription::Configuration(ConfigurationChanges {}),
                Some(first.cursor),
            ),
            Err(SubscribeError::ResyncRequired)
        ));
    }

    #[tokio::test]
    async fn coalescible_stream_recovers_latest_after_eviction() {
        let broker = EventBroker::new();
        let scope = scope(1);
        let (_, mut feed) = broker
            .subscribe(
                scope.clone(),
                Subscription::SyncStatus(SyncStatusChanges::default()),
                None,
            )
            .unwrap();
        for records in 0..=MAX_REPLAY_EVENTS {
            broker
                .publish(
                    scope.clone(),
                    Event::SyncStatusChanged(SyncStatus::Queued {
                        records: records as u64,
                    }),
                )
                .unwrap();
        }
        let Event::SyncStatusChanged(SyncStatus::Queued { records }) =
            feed.recv().await.unwrap().event
        else {
            panic!("latest sync status expected");
        };
        assert_eq!(records, MAX_REPLAY_EVENTS as u64);

        broker
            .publish(
                scope,
                Event::SyncStatusChanged(SyncStatus::Queued {
                    records: MAX_REPLAY_EVENTS as u64 + 1,
                }),
            )
            .unwrap();
        let Event::SyncStatusChanged(SyncStatus::Queued { records }) =
            feed.recv().await.unwrap().event
        else {
            panic!("next live sync status expected");
        };
        assert_eq!(records, MAX_REPLAY_EVENTS as u64 + 1);
    }

    #[tokio::test]
    async fn discrete_stream_reports_backpressure_after_eviction() {
        let broker = EventBroker::new();
        let scope = scope(1);
        let (_, mut feed) = broker
            .subscribe(
                scope.clone(),
                Subscription::Notifications(Notifications {}),
                None,
            )
            .unwrap();
        for index in 0..=MAX_REPLAY_EVENTS {
            broker
                .publish(
                    scope.clone(),
                    Event::NotificationRaised(Notification {
                        notification_id: NotificationId::new(Uuid::from_u128(index as u128 + 1)),
                        scope: scope.clone(),
                        severity: NotificationSeverity::Information,
                        message_id: eitmad_contracts::errors::MessageId::parse(
                            "eitmad.message.contract-invalid.v1",
                        )
                        .unwrap(),
                        parameters: Vec::new(),
                        correlation_id: Some(CorrelationId::new(Uuid::nil())),
                    }),
                )
                .unwrap();
        }
        assert!(matches!(feed.recv().await, Err(FeedError::Backpressure)));
    }

    #[test]
    fn rejects_embedded_scope_mismatch() {
        let broker = EventBroker::new();
        let authorized = scope(1);
        let other = scope(2);
        assert!(matches!(
            broker.publish(authorized, config_event(&other, 1)),
            Err(PublishError::ScopeMismatch)
        ));
    }
}
