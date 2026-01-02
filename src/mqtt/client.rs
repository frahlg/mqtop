#![allow(dead_code)]

use anyhow::Result;
use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS, Transport};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

use crate::config::MqttConfig;
use crate::mqtt::message::MqttMessage;
use crate::mqtt::resilience::{BackoffStrategy, ConnectionHealth};

/// Connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
}

/// Events from the MQTT client
#[derive(Debug)]
pub enum MqttEvent {
    Message(MqttMessage),
    StateChange(ConnectionState),
    Error(String),
}

pub struct MqttClient {
    client: AsyncClient,
    config: Arc<MqttConfig>,
    health: Arc<RwLock<ConnectionHealth>>,
}

impl MqttClient {
    /// Create a new MQTT client and start the event loop
    pub async fn connect(
        config: MqttConfig,
        event_tx: mpsc::UnboundedSender<MqttEvent>,
    ) -> Result<Self> {
        Self::connect_with_backoff(config, event_tx, BackoffStrategy::default()).await
    }

    /// Create a new MQTT client with custom backoff strategy
    pub async fn connect_with_backoff(
        config: MqttConfig,
        event_tx: mpsc::UnboundedSender<MqttEvent>,
        backoff: BackoffStrategy,
    ) -> Result<Self> {
        let config = Arc::new(config);
        let health = Arc::new(RwLock::new(ConnectionHealth::new(backoff)));

        // Build MQTT options
        let mut mqttoptions = MqttOptions::new(&config.client_id, &config.host, config.port);

        // Set authentication: username (defaults to client_id), password = token
        mqttoptions.set_credentials(config.get_username(), config.get_token());
        mqttoptions.set_keep_alive(Duration::from_secs(config.keep_alive_secs));

        // Set clean session to false for persistent sessions
        mqttoptions.set_clean_session(true);

        // Configure TLS if enabled
        if config.use_tls {
            // Use native TLS with system certificates
            let transport = Transport::tls_with_default_config();
            mqttoptions.set_transport(transport);
        }

        // Increase capacity for high-throughput scenarios
        mqttoptions.set_inflight(100);

        // Increase max packet size for large payloads (default is 10KB, set to 1MB)
        mqttoptions.set_max_packet_size(1024 * 1024, 1024 * 1024);

        let (client, mut eventloop) = AsyncClient::new(mqttoptions, 1000);

        let health_clone = Arc::clone(&health);
        let event_tx_clone = event_tx.clone();
        let subscribe_topic = config.subscribe_topic.clone();
        let client_clone = client.clone();

        // Spawn the event loop handler
        tokio::spawn(async move {
            loop {
                match eventloop.poll().await {
                    Ok(notification) => {
                        match notification {
                            Event::Incoming(Packet::Publish(publish)) => {
                                let msg = MqttMessage::new(
                                    publish.topic.to_string(),
                                    publish.payload.to_vec(),
                                    publish.qos as u8,
                                    publish.retain,
                                );
                                let _ = event_tx_clone.send(MqttEvent::Message(msg));
                            }
                            Event::Incoming(Packet::ConnAck(connack)) => {
                                info!("Connected to MQTT broker: {:?}", connack);
                                health_clone.write().await.record_success();
                                let _ = event_tx_clone
                                    .send(MqttEvent::StateChange(ConnectionState::Connected));

                                // Subscribe after connection is established
                                info!("Subscribing to: {}", subscribe_topic);
                                if let Err(e) = client_clone
                                    .subscribe(&subscribe_topic, QoS::AtLeastOnce)
                                    .await
                                {
                                    error!("Failed to subscribe: {:?}", e);
                                    let _ = event_tx_clone.send(MqttEvent::Error(format!(
                                        "Subscribe failed: {:?}",
                                        e
                                    )));
                                }
                            }
                            Event::Incoming(Packet::SubAck(suback)) => {
                                info!("Subscription acknowledged: {:?}", suback);
                            }
                            Event::Incoming(Packet::PingResp) => {
                                debug!("Ping response received");
                            }
                            Event::Outgoing(_) => {
                                // Outgoing events, usually not interesting
                            }
                            other => {
                                debug!("MQTT event: {:?}", other);
                            }
                        }
                    }
                    Err(e) => {
                        let error_str = format!("{:?}", e);
                        error!("MQTT connection error: {}", error_str);

                        let mut health = health_clone.write().await;
                        health.record_failure(error_str.clone());

                        let _ = event_tx_clone
                            .send(MqttEvent::StateChange(ConnectionState::Reconnecting));
                        let _ = event_tx_clone.send(MqttEvent::Error(error_str));

                        // Check if we should continue reconnecting
                        if !health.should_reconnect() {
                            error!("Max reconnection attempts reached, giving up");
                            let _ = event_tx_clone
                                .send(MqttEvent::StateChange(ConnectionState::Disconnected));
                            break;
                        }

                        // Get backoff delay
                        if let Some(delay) = health.next_reconnect_delay() {
                            warn!(
                                "Reconnecting in {:?} (attempt {}, total reconnects: {})",
                                delay,
                                health.failure_count(),
                                health.total_reconnects()
                            );
                            drop(health); // Release lock before sleeping
                            tokio::time::sleep(delay).await;
                        }
                    }
                }
            }
        });

        let mqtt_client = Self {
            client,
            config,
            health,
        };

        let _ = event_tx.send(MqttEvent::StateChange(ConnectionState::Connecting));

        Ok(mqtt_client)
    }

    /// Subscribe to the configured topic pattern
    pub async fn subscribe(&self) -> Result<()> {
        info!("Subscribing to: {}", self.config.subscribe_topic);
        self.client
            .subscribe(&self.config.subscribe_topic, QoS::AtLeastOnce)
            .await?;
        Ok(())
    }

    /// Subscribe to a specific topic
    pub async fn subscribe_topic(&self, topic: &str) -> Result<()> {
        info!("Subscribing to: {}", topic);
        self.client.subscribe(topic, QoS::AtLeastOnce).await?;
        Ok(())
    }

    /// Unsubscribe from a topic
    pub async fn unsubscribe(&self, topic: &str) -> Result<()> {
        info!("Unsubscribing from: {}", topic);
        self.client.unsubscribe(topic).await?;
        Ok(())
    }

    /// Publish a message (useful for testing)
    pub async fn publish(&self, topic: &str, payload: &[u8], qos: QoS) -> Result<()> {
        self.client.publish(topic, qos, false, payload).await?;
        Ok(())
    }

    /// Disconnect from the broker
    pub async fn disconnect(&self) -> Result<()> {
        self.client.disconnect().await?;
        Ok(())
    }

    /// Check if the connection is healthy
    pub async fn is_healthy(&self) -> bool {
        self.health.read().await.is_healthy()
    }

    /// Get connection health statistics
    pub async fn health_stats(&self) -> (u64, u64, u32) {
        let health = self.health.read().await;
        (
            health.total_connections(),
            health.total_reconnects(),
            health.failure_count(),
        )
    }

    /// Get the last error message if any
    pub async fn last_error(&self) -> Option<String> {
        self.health.read().await.last_error().map(|s| s.to_string())
    }
}
