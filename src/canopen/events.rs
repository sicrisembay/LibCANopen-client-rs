// Event system for CANopen messages
use crate::canopen::message::{CanMessage, MessageType};
use std::time::SystemTime;
use tokio::sync::broadcast;

#[derive(Debug, Clone)]
pub struct MessageEvent {
    pub message: CanMessage,
    pub timestamp: SystemTime,
}

impl MessageEvent {
    pub fn new(message: CanMessage) -> Self {
        Self {
            message,
            timestamp: SystemTime::now(),
        }
    }
}

/// Event Manager - handles message distribution to subscribers
pub struct EventManager {
    // Different event channels for different message types
    pub packet_tx: broadcast::Sender<MessageEvent>,
    pub sdo_tx: broadcast::Sender<MessageEvent>,
    pub nmt_tx: broadcast::Sender<MessageEvent>,
    pub pdo_tx: broadcast::Sender<MessageEvent>,
    pub emergency_tx: broadcast::Sender<MessageEvent>,
    pub sync_tx: broadcast::Sender<MessageEvent>,
    pub lss_tx: broadcast::Sender<MessageEvent>,
    pub time_tx: broadcast::Sender<MessageEvent>,
}

impl EventManager {
    pub fn new() -> Self {
        Self {
            packet_tx: broadcast::channel(1000).0,
            sdo_tx: broadcast::channel(100).0,
            nmt_tx: broadcast::channel(100).0,
            pdo_tx: broadcast::channel(500).0,
            emergency_tx: broadcast::channel(100).0,
            sync_tx: broadcast::channel(100).0,
            lss_tx: broadcast::channel(100).0,
            time_tx: broadcast::channel(100).0,
        }
    }

    /// Emit a message to all relevant subscribers
    pub fn emit_message(&self, message: CanMessage) {
        let event = MessageEvent::new(message.clone());

        // Always emit to general packet channel
        let _ = self.packet_tx.send(event.clone());

        // Emit to specific channels based on message type
        match message.message_type() {
            MessageType::Sdo => {
                let _ = self.sdo_tx.send(event);
            }
            MessageType::Nmt | MessageType::NmtErrorControl => {
                let _ = self.nmt_tx.send(event);
            }
            MessageType::Pdo => {
                let _ = self.pdo_tx.send(event);
            }
            MessageType::Emergency => {
                let _ = self.emergency_tx.send(event);
            }
            MessageType::Sync => {
                let _ = self.sync_tx.send(event);
            }
            MessageType::Lss => {
                let _ = self.lss_tx.send(event);
            }
            MessageType::TimeStamp => {
                let _ = self.time_tx.send(event);
            }
            MessageType::Unknown => {
                // Only emit to general packet channel
            }
        }
    }

    /// Subscribe to all packet events
    pub fn subscribe_packets(&self) -> broadcast::Receiver<MessageEvent> {
        self.packet_tx.subscribe()
    }

    /// Subscribe to SDO events
    pub fn subscribe_sdo(&self) -> broadcast::Receiver<MessageEvent> {
        self.sdo_tx.subscribe()
    }

    /// Subscribe to NMT events
    pub fn subscribe_nmt(&self) -> broadcast::Receiver<MessageEvent> {
        self.nmt_tx.subscribe()
    }

    /// Subscribe to PDO events
    pub fn subscribe_pdo(&self) -> broadcast::Receiver<MessageEvent> {
        self.pdo_tx.subscribe()
    }

    /// Subscribe to Emergency events
    pub fn subscribe_emergency(&self) -> broadcast::Receiver<MessageEvent> {
        self.emergency_tx.subscribe()
    }

    /// Subscribe to SYNC events
    pub fn subscribe_sync(&self) -> broadcast::Receiver<MessageEvent> {
        self.sync_tx.subscribe()
    }

    /// Subscribe to LSS events
    pub fn subscribe_lss(&self) -> broadcast::Receiver<MessageEvent> {
        self.lss_tx.subscribe()
    }

    /// Subscribe to Time events
    pub fn subscribe_time(&self) -> broadcast::Receiver<MessageEvent> {
        self.time_tx.subscribe()
    }
}

impl Default for EventManager {
    fn default() -> Self {
        Self::new()
    }
}
