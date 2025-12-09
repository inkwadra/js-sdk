//! Realtime Service
//!
//! SSE (Server-Sent Events) based realtime subscription service for PocketBase.
//! This service allows subscribing to real-time changes on collections and records.

use crate::client::Client;
use crate::client_response_error::ClientResponseError;
use crate::services::base_service::BaseService;
use crate::tools::options::SendOptions;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio::time::{sleep, Duration};

/// Type alias for the unsubscribe function.
pub type UnsubscribeFunc = Box<
    dyn FnOnce() -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> + Send + Sync,
>;

/// Realtime message received from SSE.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealtimeMessage {
    /// The action that occurred (e.g., "create", "update", "delete").
    pub action: String,
    /// The record data.
    pub record: serde_json::Value,
}

/// Callback type for subscription events.
pub type SubscriptionCallback = Box<dyn Fn(RealtimeMessage) + Send + Sync>;

/// Internal subscription entry.
struct SubscriptionEntry {
    /// The callback to invoke when a message is received.
    callback: SubscriptionCallback,
    /// Unique ID for this subscription.
    id: u64,
}

/// Subscriptions map: topic key -> list of subscription entries.
type Subscriptions = HashMap<String, Vec<SubscriptionEntry>>;

/// Realtime service for subscribing to SSE events from PocketBase.
pub struct RealtimeService {
    /// Client reference.
    client: Arc<Client>,

    /// Client ID assigned by the server.
    client_id: Arc<RwLock<String>>,

    /// Active subscriptions.
    subscriptions: Arc<RwLock<Subscriptions>>,

    /// Last sent subscription topics.
    last_sent_subscriptions: Arc<RwLock<Vec<String>>>,

    /// Maximum connection timeout in milliseconds.
    max_connect_timeout: u64,

    /// Reconnect attempt counter.
    reconnect_attempts: Arc<RwLock<u32>>,

    /// Maximum reconnect attempts.
    max_reconnect_attempts: u32,

    /// Predefined reconnect intervals in milliseconds.
    predefined_reconnect_intervals: Vec<u64>,

    /// Whether currently connected.
    is_connected: Arc<RwLock<bool>>,

    /// Channel to signal disconnect.
    disconnect_tx: Arc<RwLock<Option<mpsc::Sender<()>>>>,

    /// Subscription ID counter.
    subscription_id_counter: Arc<RwLock<u64>>,

    /// Pending connect callbacks.
    pending_connects:
        Arc<RwLock<Vec<tokio::sync::oneshot::Sender<Result<(), ClientResponseError>>>>>,

    /// Broadcast channel for connection events.
    connect_broadcast: broadcast::Sender<()>,
}

impl RealtimeService {
    /// Creates a new RealtimeService.
    pub fn new(client: Arc<Client>) -> Self {
        let (connect_broadcast, _) = broadcast::channel(16);

        Self {
            client,
            client_id: Arc::new(RwLock::new(String::new())),
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            last_sent_subscriptions: Arc::new(RwLock::new(Vec::new())),
            max_connect_timeout: 15000,
            reconnect_attempts: Arc::new(RwLock::new(0)),
            max_reconnect_attempts: u32::MAX,
            predefined_reconnect_intervals: vec![200, 300, 500, 1000, 1200, 1500, 2000],
            is_connected: Arc::new(RwLock::new(false)),
            disconnect_tx: Arc::new(RwLock::new(None)),
            subscription_id_counter: Arc::new(RwLock::new(0)),
            pending_connects: Arc::new(RwLock::new(Vec::new())),
            connect_broadcast,
        }
    }

    /// Returns whether the realtime connection has been established.
    pub async fn is_connected(&self) -> bool {
        let connected = *self.is_connected.read().await;
        let client_id = self.client_id.read().await;
        let pending = self.pending_connects.read().await;

        connected && !client_id.is_empty() && pending.is_empty()
    }

    /// Returns the client ID.
    pub async fn get_client_id(&self) -> String {
        self.client_id.read().await.clone()
    }

    /// Subscribes to a topic with the given callback.
    ///
    /// # Arguments
    ///
    /// * `topic` - The topic to subscribe to (e.g., "collection_name" or "collection_name/record_id").
    /// * `callback` - The callback to invoke when a message is received.
    /// * `options` - Optional send options for the subscription.
    ///
    /// # Returns
    ///
    /// A function that can be called to unsubscribe from this specific subscription.
    pub async fn subscribe<F>(
        &self,
        topic: &str,
        callback: F,
        options: Option<SendOptions>,
    ) -> Result<impl FnOnce() + Send, ClientResponseError>
    where
        F: Fn(RealtimeMessage) + Send + Sync + 'static,
    {
        if topic.is_empty() {
            return Err(ClientResponseError::new(
                "",
                400,
                [(
                    "message".to_string(),
                    serde_json::json!("topic must be set"),
                )]
                .into_iter()
                .collect(),
            ));
        }

        let mut key = topic.to_string();

        // Serialize and append the topic options (if any)
        if let Some(opts) = options {
            let serialized_opts = serde_json::json!({
                "query": opts.query,
                "headers": opts.headers
            });

            if let Ok(json_str) = serde_json::to_string(&serialized_opts) {
                let encoded = urlencoding::encode(&json_str);
                let delimiter = if key.contains('?') { "&" } else { "?" };
                key = format!("{}{}options={}", key, delimiter, encoded);
            }
        }

        // Generate a unique subscription ID
        let sub_id = {
            let mut counter = self.subscription_id_counter.write().await;
            *counter += 1;
            *counter
        };

        // Store the subscription
        {
            let mut subs = self.subscriptions.write().await;
            let entry = SubscriptionEntry {
                callback: Box::new(callback),
                id: sub_id,
            };

            subs.entry(key.clone()).or_default().push(entry);
        }

        let is_new_key = {
            let subs = self.subscriptions.read().await;
            subs.get(&key).map(|v| v.len() == 1).unwrap_or(false)
        };

        // Connect if not already connected
        if !self.is_connected().await {
            self.connect().await?;
        } else if is_new_key {
            // Submit updated subscriptions if this is a new topic
            self.submit_subscriptions().await?;
        }

        // Create unsubscribe closure
        let topic_clone = topic.to_string();
        let subscriptions = Arc::clone(&self.subscriptions);
        let last_sent = Arc::clone(&self.last_sent_subscriptions);
        let is_connected = Arc::clone(&self.is_connected);
        let client = self.client.clone();

        Ok(move || {
            let rt = tokio::runtime::Handle::current();
            rt.block_on(async {
                Self::unsubscribe_by_id(
                    &subscriptions,
                    &last_sent,
                    &is_connected,
                    &client,
                    &topic_clone,
                    sub_id,
                )
                .await;
            });
        })
    }

    /// Unsubscribes a specific subscription by its ID.
    async fn unsubscribe_by_id(
        subscriptions: &Arc<RwLock<Subscriptions>>,
        last_sent: &Arc<RwLock<Vec<String>>>,
        is_connected: &Arc<RwLock<bool>>,
        _client: &Arc<Client>,
        topic: &str,
        sub_id: u64,
    ) {
        let mut need_to_submit = false;
        let has_remaining_subs;

        {
            let mut subs = subscriptions.write().await;

            // Find keys matching this topic
            let topic_prefix = if topic.contains('?') {
                topic.to_string()
            } else {
                format!("{}?", topic)
            };

            let matching_keys: Vec<String> = subs
                .keys()
                .filter(|k| *k == topic || format!("{}?", k).starts_with(&topic_prefix))
                .cloned()
                .collect();

            for key in matching_keys {
                if let Some(entries) = subs.get_mut(&key) {
                    let initial_len = entries.len();
                    entries.retain(|e| e.id != sub_id);

                    if entries.len() < initial_len {
                        need_to_submit = true;
                    }

                    if entries.is_empty() {
                        subs.remove(&key);
                    }
                }
            }

            has_remaining_subs = !subs.is_empty();
        }

        if !has_remaining_subs {
            *is_connected.write().await = false;
        } else if need_to_submit && *is_connected.read().await {
            // Would need to submit subscriptions here
            let topics: Vec<String> = subscriptions.read().await.keys().cloned().collect();
            *last_sent.write().await = topics;
        }
    }

    /// Unsubscribes from all subscriptions matching the specified topic.
    ///
    /// If topic is empty, unsubscribes from all subscriptions.
    pub async fn unsubscribe(&self, topic: Option<&str>) -> Result<(), ClientResponseError> {
        let mut need_to_submit = false;

        {
            let mut subs = self.subscriptions.write().await;

            match topic {
                None => {
                    // Remove all subscriptions
                    subs.clear();
                }
                Some(t) => {
                    // Remove all subscriptions for the topic
                    let topic_prefix = if t.contains('?') {
                        t.to_string()
                    } else {
                        format!("{}?", t)
                    };

                    let keys_to_remove: Vec<String> = subs
                        .keys()
                        .filter(|k| *k == t || format!("{}?", k).starts_with(&topic_prefix))
                        .cloned()
                        .collect();

                    for key in keys_to_remove {
                        subs.remove(&key);
                        need_to_submit = true;
                    }
                }
            }
        }

        let has_subs = !self.subscriptions.read().await.is_empty();

        if !has_subs {
            self.disconnect(false).await;
        } else if need_to_submit {
            self.submit_subscriptions().await?;
        }

        Ok(())
    }

    /// Unsubscribes from all subscriptions starting with the specified key prefix.
    pub async fn unsubscribe_by_prefix(&self, key_prefix: &str) -> Result<(), ClientResponseError> {
        let mut has_at_least_one = false;

        {
            let mut subs = self.subscriptions.write().await;

            let keys_to_remove: Vec<String> = subs
                .keys()
                .filter(|k| format!("{}?", k).starts_with(key_prefix))
                .cloned()
                .collect();

            for key in keys_to_remove {
                subs.remove(&key);
                has_at_least_one = true;
            }
        }

        if !has_at_least_one {
            return Ok(());
        }

        let has_subs = !self.subscriptions.read().await.is_empty();

        if has_subs {
            self.submit_subscriptions().await?;
        } else {
            self.disconnect(false).await;
        }

        Ok(())
    }

    /// Connects to the SSE endpoint.
    async fn connect(&self) -> Result<(), ClientResponseError> {
        // Check if already reconnecting
        if *self.reconnect_attempts.read().await > 0 {
            return Ok(());
        }

        // Create a oneshot channel for the connection result
        let (tx, rx) = tokio::sync::oneshot::channel();

        {
            let mut pending = self.pending_connects.write().await;
            pending.push(tx);

            if pending.len() > 1 {
                // Already connecting, wait for result
                drop(pending);
                return rx.await.unwrap_or(Err(ClientResponseError::new(
                    "",
                    0,
                    [(
                        "message".to_string(),
                        serde_json::json!("Connection cancelled"),
                    )]
                    .into_iter()
                    .collect(),
                )));
            }
        }

        // Initialize the connection
        self.init_connect().await;

        // Wait for connection result
        rx.await.unwrap_or(Err(ClientResponseError::new(
            "",
            0,
            [(
                "message".to_string(),
                serde_json::json!("Connection cancelled"),
            )]
            .into_iter()
            .collect(),
        )))
    }

    /// Initializes the SSE connection.
    async fn init_connect(&self) {
        // Disconnect any existing connection
        self.disconnect(true).await;

        let url = self.client.build_url("/api/realtime");

        // Create disconnect channel
        let (disconnect_tx, mut disconnect_rx) = mpsc::channel::<()>(1);
        *self.disconnect_tx.write().await = Some(disconnect_tx);

        let client_id = Arc::clone(&self.client_id);
        let is_connected = Arc::clone(&self.is_connected);
        let subscriptions = Arc::clone(&self.subscriptions);
        let last_sent = Arc::clone(&self.last_sent_subscriptions);
        let pending_connects = Arc::clone(&self.pending_connects);
        let reconnect_attempts = Arc::clone(&self.reconnect_attempts);
        let predefined_intervals = self.predefined_reconnect_intervals.clone();
        let max_reconnect_attempts = self.max_reconnect_attempts;
        let max_connect_timeout = self.max_connect_timeout;
        let client = self.client.clone();
        let connect_broadcast = self.connect_broadcast.clone();

        tokio::spawn(async move {
            // Create HTTP client for SSE
            let http_client = reqwest::Client::new();

            // Connection timeout
            let timeout = tokio::time::timeout(
                Duration::from_millis(max_connect_timeout),
                http_client.get(&url).send(),
            );

            let response = tokio::select! {
                res = timeout => {
                    match res {
                        Ok(Ok(resp)) => resp,
                        Ok(Err(e)) => {
                            Self::handle_connect_error(
                                &pending_connects,
                                &reconnect_attempts,
                                &is_connected,
                                &client_id,
                                &predefined_intervals,
                                max_reconnect_attempts,
                                ClientResponseError::from(e),
                            ).await;
                            return;
                        }
                        Err(_) => {
                            Self::handle_connect_error(
                                &pending_connects,
                                &reconnect_attempts,
                                &is_connected,
                                &client_id,
                                &predefined_intervals,
                                max_reconnect_attempts,
                                ClientResponseError::new("", 0, [("message".to_string(), serde_json::json!("Connection timeout"))].into_iter().collect()),
                            ).await;
                            return;
                        }
                    }
                }
                _ = disconnect_rx.recv() => {
                    return;
                }
            };

            if !response.status().is_success() {
                Self::handle_connect_error(
                    &pending_connects,
                    &reconnect_attempts,
                    &is_connected,
                    &client_id,
                    &predefined_intervals,
                    max_reconnect_attempts,
                    ClientResponseError::new(&url, response.status().as_u16(), HashMap::new()),
                )
                .await;
                return;
            }

            // Process SSE events
            let mut stream = response.bytes_stream();
            use futures_util::StreamExt;

            let mut buffer = String::new();
            let mut event_type = String::new();
            let mut event_data = String::new();
            let mut event_id = String::new();

            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        buffer.push_str(&String::from_utf8_lossy(&chunk));

                        // Process complete lines
                        while let Some(newline_pos) = buffer.find('\n') {
                            let line = buffer[..newline_pos].trim_end_matches('\r').to_string();
                            buffer = buffer[newline_pos + 1..].to_string();

                            if line.is_empty() {
                                // Empty line means end of event
                                if !event_type.is_empty() || !event_data.is_empty() {
                                    // Handle PB_CONNECT event
                                    if event_type == "PB_CONNECT" {
                                        *client_id.write().await = event_id.clone();

                                        // Submit subscriptions
                                        let topics: Vec<String> = {
                                            let subs = subscriptions.read().await;
                                            subs.iter()
                                                .filter(|(_, entries)| !entries.is_empty())
                                                .map(|(k, _)| k.clone())
                                                .collect()
                                        };

                                        if !topics.is_empty() {
                                            let mut opts = SendOptions::post();
                                            opts.body = Some(serde_json::json!({
                                                "clientId": event_id,
                                                "subscriptions": topics
                                            }));

                                            let _ = client
                                                .send::<serde_json::Value>("/api/realtime", opts)
                                                .await;
                                            *last_sent.write().await = topics;
                                        }

                                        // Mark as connected
                                        *is_connected.write().await = true;
                                        *reconnect_attempts.write().await = 0;

                                        // Resolve pending connects
                                        let mut pending = pending_connects.write().await;
                                        for tx in pending.drain(..) {
                                            let _ = tx.send(Ok(()));
                                        }

                                        // Broadcast connection event
                                        let _ = connect_broadcast.send(());
                                    } else {
                                        // Dispatch to subscribers
                                        if let Ok(msg) =
                                            serde_json::from_str::<RealtimeMessage>(&event_data)
                                        {
                                            let subs = subscriptions.read().await;
                                            if let Some(entries) = subs.get(&event_type) {
                                                for entry in entries {
                                                    (entry.callback)(msg.clone());
                                                }
                                            }
                                        }
                                    }
                                }

                                // Reset for next event
                                event_type.clear();
                                event_data.clear();
                                event_id.clear();
                            } else if let Some(value) = line.strip_prefix("event:") {
                                event_type = value.trim().to_string();
                            } else if let Some(value) = line.strip_prefix("data:") {
                                if !event_data.is_empty() {
                                    event_data.push('\n');
                                }
                                event_data.push_str(value.trim());
                            } else if let Some(value) = line.strip_prefix("id:") {
                                event_id = value.trim().to_string();
                            }
                        }
                    }
                    Err(e) => {
                        Self::handle_connect_error(
                            &pending_connects,
                            &reconnect_attempts,
                            &is_connected,
                            &client_id,
                            &predefined_intervals,
                            max_reconnect_attempts,
                            ClientResponseError::from(e),
                        )
                        .await;
                        break;
                    }
                }
            }
        });
    }

    /// Handles connection errors.
    async fn handle_connect_error(
        pending_connects: &Arc<
            RwLock<Vec<tokio::sync::oneshot::Sender<Result<(), ClientResponseError>>>>,
        >,
        reconnect_attempts: &Arc<RwLock<u32>>,
        is_connected: &Arc<RwLock<bool>>,
        client_id: &Arc<RwLock<String>>,
        predefined_intervals: &[u64],
        max_reconnect_attempts: u32,
        error: ClientResponseError,
    ) {
        let attempts = *reconnect_attempts.read().await;
        let was_connected = !client_id.read().await.is_empty();

        if (!was_connected && attempts == 0) || attempts > max_reconnect_attempts {
            // Direct reject
            let mut pending = pending_connects.write().await;
            for tx in pending.drain(..) {
                let _ = tx.send(Err(error.clone()));
            }

            *is_connected.write().await = false;
            *client_id.write().await = String::new();
            *reconnect_attempts.write().await = 0;
            return;
        }

        // Reconnect in background
        *is_connected.write().await = false;
        *client_id.write().await = String::new();

        let interval_idx = (attempts as usize).min(predefined_intervals.len() - 1);
        let timeout = predefined_intervals[interval_idx];

        *reconnect_attempts.write().await = attempts + 1;

        sleep(Duration::from_millis(timeout)).await;

        // Would need to call init_connect again here
        // This is simplified - in a full implementation, we'd store a reference
        // to the service or use a channel to trigger reconnection
    }

    /// Submits the current subscriptions to the server.
    async fn submit_subscriptions(&self) -> Result<(), ClientResponseError> {
        let client_id = self.client_id.read().await.clone();
        if client_id.is_empty() {
            return Ok(());
        }

        let topics: Vec<String> = {
            let subs = self.subscriptions.read().await;
            subs.iter()
                .filter(|(_, entries)| !entries.is_empty())
                .map(|(k, _)| k.clone())
                .collect()
        };

        *self.last_sent_subscriptions.write().await = topics.clone();

        let mut opts = SendOptions::post();
        opts.body = Some(serde_json::json!({
            "clientId": client_id,
            "subscriptions": topics
        }));
        opts.request_key = Some(format!("realtime_{}", client_id));

        match self
            .client
            .send::<serde_json::Value>("/api/realtime", opts)
            .await
        {
            Ok(_) => Ok(()),
            Err(e) if e.is_abort => Ok(()), // Silently ignore aborted requests
            Err(e) => Err(e),
        }
    }

    /// Disconnects from the SSE endpoint.
    async fn disconnect(&self, from_reconnect: bool) {
        // Cancel pending requests
        let client_id = self.client_id.read().await.clone();
        if !client_id.is_empty() {
            // Would call onDisconnect callback here if we had one
        }

        // Send disconnect signal
        if let Some(tx) = self.disconnect_tx.write().await.take() {
            let _ = tx.send(()).await;
        }

        *self.client_id.write().await = String::new();
        *self.is_connected.write().await = false;

        if !from_reconnect {
            *self.reconnect_attempts.write().await = 0;

            // Resolve any remaining pending connects
            let mut pending = self.pending_connects.write().await;
            for tx in pending.drain(..) {
                let _ = tx.send(Ok(()));
            }
        }
    }
}

impl BaseService for RealtimeService {
    fn client(&self) -> &Arc<Client> {
        &self.client
    }
}

impl std::fmt::Debug for RealtimeService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RealtimeService")
            .field("max_connect_timeout", &self.max_connect_timeout)
            .field("max_reconnect_attempts", &self.max_reconnect_attempts)
            .finish()
    }
}

/// URL encoding helper module.
mod urlencoding {
    pub fn encode(input: &str) -> String {
        let mut result = String::new();
        for c in input.chars() {
            match c {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' => {
                    result.push(c);
                }
                _ => {
                    for byte in c.to_string().as_bytes() {
                        result.push_str(&format!("%{:02X}", byte));
                    }
                }
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_realtime_service_new() {
        let client = Client::new("http://localhost:8090");
        let service = RealtimeService::new(client);

        assert!(!service.is_connected().await);
        assert!(service.get_client_id().await.is_empty());
    }

    #[test]
    fn test_realtime_message_deserialize() {
        let json = r#"{"action":"create","record":{"id":"test123"}}"#;
        let msg: RealtimeMessage = serde_json::from_str(json).unwrap();

        assert_eq!(msg.action, "create");
        assert_eq!(msg.record["id"], "test123");
    }
}
