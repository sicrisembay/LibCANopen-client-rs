//! # libCANopen Simple
//!
//! A simple CANopen library for Rust using PEAK CAN hardware adapters.
//!
//! This library provides:
//! - Async/await based API
//! - SDO client with expedited and segmented transfers
//! - NMT state management and commands
//! - PDO handling
//! - Event-driven architecture
//! - PEAK CAN hardware support via peak-can-sys
//!
//! ## Quick Start
//!
//! ```no_run
//! use libcanopen_simple::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     // Create PEAK CAN adapter
//!     let peak_adapter = PeakCanAdapter::new(
//!         PcanHandle::PcanUsbbus1, 
//!         BusSpeed::Baud250K
//!     );
//!
//!     // Create CANopen instance
//!     let mut canopen = CANopenSimple::new(Box::new(peak_adapter));
//!
//!     // Connect to hardware
//!     canopen.connect().await?;
//!
//!     // Send NMT start to all nodes
//!     canopen.nmt_start(0).await?;
//!
//!     Ok(())
//! }
//! ```

// Module declarations
mod canopen;
mod hardware;
mod errors;
mod utils;

// Public exports
pub use errors::{CANopenError, Result};
pub use canopen::{
    CanMessage, CanId, MessageType,
    SdoClient, SdoDirection, SdoTransfer, SdoState,
    NmtManager, NmtState, NodeState,
    EventManager, MessageEvent,
    PdoManager,
};
pub use hardware::{
    CanHardware,
    peak_can::{PeakCanAdapter, BusSpeed, PcanHandle},
};

use tokio::sync::{mpsc, RwLock};
use std::sync::Arc;

/// Main CANopen library interface
pub struct CANopenSimple {
    hardware: Arc<RwLock<Box<dyn CanHardware + Send + Sync>>>,
    event_manager: Arc<EventManager>,
    nmt_manager: Arc<RwLock<NmtManager>>,
    pdo_manager: Arc<RwLock<PdoManager>>,
    message_tx: mpsc::Sender<CanMessage>,
    sdo_transfer_tx: mpsc::Sender<SdoTransfer>,
    is_running: Arc<RwLock<bool>>,
}

impl CANopenSimple {
    /// Create a new CANopen instance with the specified hardware
    pub fn new(hardware: Box<dyn CanHardware + Send + Sync>) -> Self {
        let (message_tx, _message_rx) = mpsc::channel(1000);
        let event_manager = Arc::new(EventManager::new());
        let nmt_manager = Arc::new(RwLock::new(NmtManager::new(message_tx.clone())));
        let pdo_manager = Arc::new(RwLock::new(PdoManager::new()));
        
        let (_sdo_client, sdo_transfer_tx) = SdoClient::new(message_tx.clone());

        Self {
            hardware: Arc::new(RwLock::new(hardware)),
            event_manager,
            nmt_manager,
            pdo_manager,
            message_tx,
            sdo_transfer_tx,
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// Connect to the CAN hardware
    pub async fn connect(&mut self) -> Result<()> {
        self.hardware.write().await.connect().await?;
        *self.is_running.write().await = true;
        
        // Start message processing task
        self.start_message_processing().await;
        
        log::info!("CANopen library connected and running");
        Ok(())
    }

    /// Disconnect from the CAN hardware
    pub async fn disconnect(&mut self) -> Result<()> {
        *self.is_running.write().await = false;
        self.hardware.write().await.disconnect().await?;
        
        log::info!("CANopen library disconnected");
        Ok(())
    }

    /// Send a CAN message
    pub async fn send_message(&self, message: CanMessage) -> Result<()> {
        self.hardware.read().await.send_message(&message).await?;
        self.event_manager.emit_message(message);
        Ok(())
    }

    /// Check if the hardware is connected
    pub async fn is_connected(&self) -> bool {
        self.hardware.read().await.is_connected()
    }

    async fn start_message_processing(&self) {
        let _hardware = Arc::clone(&self.hardware);
        let _event_manager = Arc::clone(&self.event_manager);
        let _nmt_manager = Arc::clone(&self.nmt_manager);
        let _is_running = Arc::clone(&self.is_running);

        tokio::spawn(async move {
            // TODO: Implement message processing loop in Phase 3
            log::debug!("Message processing task started");
        });
    }

    // Event subscription methods
    
    /// Subscribe to all packet events
    pub fn subscribe_packets(&self) -> tokio::sync::broadcast::Receiver<MessageEvent> {
        self.event_manager.subscribe_packets()
    }

    /// Subscribe to SDO events
    pub fn subscribe_sdo(&self) -> tokio::sync::broadcast::Receiver<MessageEvent> {
        self.event_manager.subscribe_sdo()
    }

    /// Subscribe to NMT events
    pub fn subscribe_nmt(&self) -> tokio::sync::broadcast::Receiver<MessageEvent> {
        self.event_manager.subscribe_nmt()
    }

    /// Subscribe to PDO events
    pub fn subscribe_pdo(&self) -> tokio::sync::broadcast::Receiver<MessageEvent> {
        self.event_manager.subscribe_pdo()
    }

    /// Subscribe to Emergency events
    pub fn subscribe_emergency(&self) -> tokio::sync::broadcast::Receiver<MessageEvent> {
        self.event_manager.subscribe_emergency()
    }

    /// Subscribe to SYNC events
    pub fn subscribe_sync(&self) -> tokio::sync::broadcast::Receiver<MessageEvent> {
        self.event_manager.subscribe_sync()
    }

    // Placeholder methods for future implementation
    
    /// Register a PDO handler (placeholder)
    pub async fn register_pdo_handler<F>(&self, cob_id: u16, handler: F) -> Result<()>
    where
        F: Fn(&[u8]) + Send + Sync + 'static,
    {
        self.pdo_manager.write().await.register_pdo_handler(cob_id, handler);
        Ok(())
    }

    /// Send an NMT start command (placeholder)
    pub async fn nmt_start(&self, _node_id: u8) -> Result<()> {
        // TODO: Implement in Phase 5
        log::debug!("NMT start command - placeholder");
        Ok(())
    }

    /// Send an NMT stop command (placeholder)
    pub async fn nmt_stop(&self, _node_id: u8) -> Result<()> {
        // TODO: Implement in Phase 5
        log::debug!("NMT stop command - placeholder");
        Ok(())
    }

    /// Write PDO data (placeholder)
    pub async fn write_pdo(&self, cob_id: u16, payload: Vec<u8>) -> Result<()> {
        let message = CanMessage::new(cob_id, payload);
        self.send_message(message).await
    }
}
