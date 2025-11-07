// Allow dead code warnings for library code that may not be used by all consumers
#![allow(dead_code)]

//! # libCANopen Client - Complete CANopen Master Implementation
//!
//! A comprehensive, async CANopen master/client library for Rust with full protocol support
//! and PEAK CAN hardware integration.
//!
//! ## Features
//!
//! ### Complete Protocol Support
//! - **SDO (Service Data Objects)**: Expedited and segmented transfers for object dictionary access
//! - **NMT (Network Management)**: Node control, state monitoring, heartbeat/node guard
//! - **PDO (Process Data Objects)**: High-speed real-time data exchange with callbacks
//! - **SYNC (Synchronization)**: Network synchronization with counter support (up to 240)
//! - **EMCY (Emergency)**: Emergency message handling with 40+ standard error codes
//! - **LSS (Layer Setting Services)**: Node commissioning and configuration (all 14 commands)
//!
//! ### Modern Architecture
//! - Async/await based API using Tokio runtime
//! - Type-safe message handling with compile-time checks
//! - Thread-safe concurrent access with Arc + RwLock
//! - Event-driven callbacks for real-time notifications
//! - Comprehensive error handling
//!
//! ## Quick Start
//!
//! ```no_run
//! use libcanopen_client::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     // Create PEAK CAN adapter
//!     let hardware = Box::new(PeakCanAdapter::new(
//!         PcanHandle::PcanUsbbus1,
//!         BusSpeed::Baud1M
//!     ));
//!     
//!     // Create CANopen instance
//!     let mut canopen = CANopenSimple::new(hardware);
//!     canopen.connect(BusSpeed::Baud1M).await?;
//!
//!     // Read device type from node 5 (timeout: 1000ms)
//!     let device_type = canopen.sdo_read_u32(5, 0x1000, 0, 1000).await?;
//!     println!("Device type: 0x{:08X}", device_type);
//!
//!     // Start node in operational mode
//!     canopen.nmt_start(5).await?;
//!
//!     // Register PDO handler
//!     canopen.register_pdo_handler(0x185, |data| {
//!         println!("PDO received: {:02X?}", data);
//!     }).await;
//!
//!     // Send SYNC
//!     canopen.send_sync().await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Protocol Examples
//!
//! ### SDO - Reading and Writing
//!
//! ```no_run
//! # use libcanopen_client::*;
//! # async fn example(canopen: &CANopenSimple) -> Result<()> {
//! // Read device information (timeout: 1000ms)
//! let vendor_id = canopen.sdo_read_u32(5, 0x1018, 1, 1000).await?;
//! let product_code = canopen.sdo_read_u32(5, 0x1018, 2, 1000).await?;
//!
//! // Configure heartbeat (1000ms)
//! canopen.sdo_write_u16(5, 0x1017, 0, 1000, 1000).await?;
//!
//! // Segmented transfers (>4 bytes) handled automatically
//! let large_data = vec![0u8; 100];
//! canopen.sdo_write_data(5, 0x1008, 0, large_data, 1000).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ### NMT - Node State Control
//!
//! ```no_run
//! # use libcanopen_client::*;
//! # async fn example(canopen: &CANopenSimple) -> Result<()> {
//! // Control node state
//! canopen.nmt_start(5).await?;              // Operational
//! canopen.nmt_stop(5).await?;               // Stopped
//! canopen.nmt_enter_pre_operational(5).await?;
//!
//! // Check node state
//! if let Some(state) = canopen.nmt_get_node_state(5).await {
//!     println!("Node 5 is in state: {:?}", state);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ### PDO - Real-time Data
//!
//! ```no_run
//! # use libcanopen_client::*;
//! # async fn example(canopen: &CANopenSimple) -> Result<()> {
//! // Send PDO
//! let message = CanMessage::new(0x185, vec![0x01, 0x02, 0x03, 0x04])?;
//! canopen.send_message(message).await?;
//!
//! // Receive PDO with handler
//! canopen.register_pdo_handler(0x285, |data| {
//!     // Process real-time data
//!     let value = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
//!     println!("Sensor value: {}", value);
//! }).await;
//! # Ok(())
//! # }
//! ```
//!
//! ### SYNC - Network Synchronization
//!
//! ```no_run
//! # use libcanopen_client::*;
//! # use tokio::time::Duration;
//! # async fn example(canopen: &CANopenSimple) -> Result<()> {
//! // Enable SYNC counter
//! canopen.set_sync_counter_enabled(true).await;
//!
//! // Send periodic SYNC (10ms = 100 Hz)
//! let mut interval = tokio::time::interval(Duration::from_millis(10));
//! loop {
//!     interval.tick().await;
//!     canopen.send_sync().await?;
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ### EMCY - Emergency Monitoring
//!
//! ```no_run
//! # use libcanopen_client::*;
//! # async fn example(canopen: &CANopenSimple) -> Result<()> {
//! canopen.register_emcy_handler(5, |emcy| {
//!     println!("Emergency: {} (0x{:04X})",
//!         emcy.error_code_description(),
//!         emcy.error_code
//!     );
//! }).await;
//! # Ok(())
//! # }
//! ```
//!
//! ### LSS - Node Configuration
//!
//! ```no_run
//! # use libcanopen_client::*;
//! # async fn example(canopen: &CANopenSimple) -> Result<()> {
//! // Enter configuration mode
//! canopen.lss_switch_state_global(LssMode::Configuration).await?;
//!
//! // Read LSS address
//! let address = canopen.lss_inquire_address(1000).await?;
//!
//! // Configure node-ID (if needed)
//! match canopen.lss_configure_node_id(10, 1000).await? {
//!     LssError::Success => println!("Configured!"),
//!     error => println!("Error: {}", error.description()),
//! }
//!
//! // Store to NV memory
//! canopen.lss_store_configuration(1000).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Performance
//!
//! Tested on PEAK CAN USB at 1 Mbps:
//! - PDO throughput: 50+ messages/second
//! - SYNC frequency: 100 Hz (10ms period)
//! - SDO segmented transfers: >4 KB successfully
//! - Message latency: Sub-millisecond in release mode
//!
//! ## Error Handling
//!
//! All operations return `Result<T, CANopenError>`:
//!
//! ```no_run
//! # use libcanopen_client::*;
//! # async fn example(canopen: &CANopenSimple) {
//! match canopen.sdo_read_u32(5, 0x1000, 0, 1000).await {
//!     Ok(device_type) => println!("Device: 0x{:08X}", device_type),
//!     Err(CANopenError::Timeout) => println!("Node not responding"),
//!     Err(CANopenError::Sdo { code }) => println!("SDO error: 0x{:08X}", code),
//!     Err(e) => println!("Error: {}", e),
//! }
//! # }
//! ```

// Module declarations
mod canopen;
mod errors;
pub mod hardware;
mod utils;

// Public exports
pub use canopen::{
    CanId, CanMessage, EmcyManager, EmergencyMessage, EventManager, LssAddress, LssCommand,
    LssError, LssManager, LssMode, LssResponse, MessageEvent, MessageType, NmtManager, NmtState,
    NodeState, PdoManager, SdoClient, SdoDirection, SdoRequest, SdoState, SdoTransfer, SyncManager,
};
pub use errors::{CANopenError, Result};
pub use hardware::{
    peak_can::{PcanHandle, PeakCanAdapter},
    BusSpeed, CanHardware,
};

use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

/// Main CANopen library interface
#[derive(Clone)]
pub struct CANopenSimple {
    hardware: Arc<RwLock<Box<dyn CanHardware + Send + Sync>>>,
    event_manager: Arc<EventManager>,
    nmt_manager: Arc<RwLock<Option<NmtManager>>>,
    pdo_manager: Arc<RwLock<PdoManager>>,
    sync_manager: Arc<RwLock<Option<SyncManager>>>,
    emcy_manager: Arc<RwLock<EmcyManager>>,
    lss_manager: Arc<RwLock<Option<LssManager>>>,
    message_tx: mpsc::Sender<CanMessage>,
    sdo_request_tx: Arc<RwLock<Option<mpsc::Sender<SdoRequest>>>>,
    is_running: Arc<RwLock<bool>>,
}

impl CANopenSimple {
    /// Create a new CANopen instance with the specified hardware
    pub fn new(hardware: Box<dyn CanHardware + Send + Sync>) -> Self {
        let (message_tx, _message_rx) = mpsc::channel(1000);
        let event_manager = Arc::new(EventManager::new());
        // NMT manager will be initialized later with proper outgoing channel
        let nmt_manager = Arc::new(RwLock::new(None));
        let pdo_manager = Arc::new(RwLock::new(PdoManager::new()));
        // SYNC manager will be initialized later with proper outgoing channel
        let sync_manager = Arc::new(RwLock::new(None));
        let emcy_manager = Arc::new(RwLock::new(EmcyManager::new()));
        // LSS manager will be initialized later with proper outgoing channel
        let lss_manager = Arc::new(RwLock::new(None));

        Self {
            hardware: Arc::new(RwLock::new(hardware)),
            event_manager,
            nmt_manager,
            pdo_manager,
            sync_manager,
            emcy_manager,
            lss_manager,
            message_tx,
            sdo_request_tx: Arc::new(RwLock::new(None)),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// Connect to the CAN hardware with specified bus speed
    pub async fn connect(&mut self, bus_speed: BusSpeed) -> Result<()> {
        self.hardware.write().await.connect(bus_speed).await?;
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
        let hardware = Arc::clone(&self.hardware);
        let event_manager = Arc::clone(&self.event_manager);
        let nmt_manager = Arc::clone(&self.nmt_manager);
        let pdo_manager = Arc::clone(&self.pdo_manager);
        let sync_manager = Arc::clone(&self.sync_manager);
        let emcy_manager = Arc::clone(&self.emcy_manager);
        let lss_manager = Arc::clone(&self.lss_manager);
        let is_running = Arc::clone(&self.is_running);
        let sdo_request_tx = Arc::clone(&self.sdo_request_tx);

        tokio::spawn(async move {
            // Create outgoing message channel for sending CAN messages to hardware
            let (outgoing_tx, mut outgoing_rx) = mpsc::channel::<CanMessage>(100);

            // Initialize NMT manager with outgoing message channel
            {
                let mut nmt_lock = nmt_manager.write().await;
                *nmt_lock = Some(NmtManager::new(outgoing_tx.clone()));
            }

            // Initialize SYNC manager (doesn't need outgoing channel for reception)
            {
                let mut sync_lock = sync_manager.write().await;
                *sync_lock = Some(SyncManager::new());
            }

            // Initialize LSS manager with outgoing message channel
            {
                let lss = LssManager::new(outgoing_tx.clone());
                lss.start_processing().await;
                let mut lss_lock = lss_manager.write().await;
                *lss_lock = Some(lss);
            }

            // Create SDO client with outgoing message channel
            let (mut sdo_client, sdo_req_tx, sdo_msg_tx) = SdoClient::new(outgoing_tx.clone());

            // Store the SDO request sender for API methods to use
            {
                let mut sdo_tx_lock = sdo_request_tx.write().await;
                *sdo_tx_lock = Some(sdo_req_tx);
            }

            // Subscribe to messages from hardware first
            let hw = hardware.read().await;
            let mut message_rx = hw.subscribe_messages();
            drop(hw); // Release the lock

            // Start SDO client processing in separate task
            let _sdo_client_handle = tokio::spawn(async move {
                sdo_client.run().await;
            });

            // Start outgoing message handler task to send messages to hardware
            let hw_clone = Arc::clone(&hardware);
            let is_running_clone = Arc::clone(&is_running);
            tokio::spawn(async move {
                while *is_running_clone.read().await {
                    match tokio::time::timeout(
                        std::time::Duration::from_millis(100),
                        outgoing_rx.recv(),
                    )
                    .await
                    {
                        Ok(Some(msg)) => {
                            log::debug!("Sending SDO request: ID={:03X}", msg.id.raw());
                            if let Err(e) = hw_clone.read().await.send_message(&msg).await {
                                log::error!("Failed to send CAN message to hardware: {:?}", e);
                            }
                        }
                        Ok(None) => {
                            log::warn!("Outgoing message channel closed");
                            break;
                        }
                        Err(_) => {
                            // Timeout - continue
                            continue;
                        }
                    }
                }
            });

            // Main message processing loop
            while *is_running.read().await {
                // Use a timeout to check if we should continue running
                match tokio::time::timeout(std::time::Duration::from_millis(100), message_rx.recv())
                    .await
                {
                    Ok(Some(can_msg)) => {
                        let msg_type = can_msg.message_type();
                        let cob_id = can_msg.id.raw();

                        // Process LSS responses (COB-ID 0x7E4 - slave to master, CiA-305)
                        if cob_id == 0x7E4 {
                            if let Some(lss) = lss_manager.read().await.as_ref() {
                                lss.process_response(&can_msg.data).await;
                            }
                        }
                        // Process SYNC messages (COB-ID 0x80)
                        else if cob_id == 0x80 {
                            if let Some(sync) = sync_manager.write().await.as_ref() {
                                sync.process_sync(&can_msg.data);
                            }
                        }
                        // Process Emergency messages (COB-ID 0x81-0xFF)
                        else if (0x81..=0xFF).contains(&cob_id) {
                            emcy_manager
                                .write()
                                .await
                                .process_emcy(cob_id, &can_msg.data);
                        }
                        // Process heartbeat messages for NMT state tracking
                        else if msg_type == MessageType::NmtErrorControl {
                            if let Some(nmt) = nmt_manager.write().await.as_mut() {
                                if let Err(e) = nmt.process_heartbeat(&can_msg).await {
                                    log::warn!("Failed to process heartbeat: {:?}", e);
                                }
                            }
                        }
                        // Process PDO messages (0x180-0x57F)
                        else if (0x180..=0x57F).contains(&cob_id) {
                            if let Err(e) = pdo_manager.write().await.process_pdo(&can_msg) {
                                log::warn!("Failed to process PDO: {:?}", e);
                            }
                        }

                        // Forward CAN messages to SDO client for processing
                        let _ = sdo_msg_tx.send(can_msg.clone()).await;

                        // Forward to event manager
                        event_manager.emit_message(can_msg);
                    }
                    Ok(None) => {
                        // Channel closed
                        log::warn!("Hardware message channel closed");
                        break;
                    }
                    Err(_) => {
                        // Timeout - continue loop to check if we should still be running
                        continue;
                    }
                }
            }
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

    // NMT Methods

    /// Start a remote node (transition to Operational state)
    ///
    /// # Arguments
    /// * `node_id` - Target node ID (0 = all nodes)
    pub async fn nmt_start(&self, node_id: u8) -> Result<()> {
        if let Some(nmt) = self.nmt_manager.read().await.as_ref() {
            nmt.start_node(node_id).await
        } else {
            Err(CANopenError::Connection)
        }
    }

    /// Stop a remote node (transition to Stopped state)
    ///
    /// # Arguments
    /// * `node_id` - Target node ID (0 = all nodes)
    pub async fn nmt_stop(&self, node_id: u8) -> Result<()> {
        if let Some(nmt) = self.nmt_manager.read().await.as_ref() {
            nmt.stop_node(node_id).await
        } else {
            Err(CANopenError::Connection)
        }
    }

    /// Put a node into Pre-Operational state
    ///
    /// # Arguments
    /// * `node_id` - Target node ID (0 = all nodes)
    pub async fn nmt_enter_pre_operational(&self, node_id: u8) -> Result<()> {
        if let Some(nmt) = self.nmt_manager.read().await.as_ref() {
            nmt.enter_pre_operational(node_id).await
        } else {
            Err(CANopenError::Connection)
        }
    }

    /// Reset a node (full device reset)
    ///
    /// # Arguments
    /// * `node_id` - Target node ID (0 = all nodes)
    pub async fn nmt_reset_node(&self, node_id: u8) -> Result<()> {
        if let Some(nmt) = self.nmt_manager.read().await.as_ref() {
            nmt.reset_node(node_id).await
        } else {
            Err(CANopenError::Connection)
        }
    }

    /// Reset communication parameters of a node
    ///
    /// # Arguments
    /// * `node_id` - Target node ID (0 = all nodes)
    pub async fn nmt_reset_communication(&self, node_id: u8) -> Result<()> {
        if let Some(nmt) = self.nmt_manager.read().await.as_ref() {
            nmt.reset_communication(node_id).await
        } else {
            Err(CANopenError::Connection)
        }
    }

    /// Check if a node has been discovered (received heartbeat)
    ///
    /// # Arguments
    /// * `node_id` - Node ID to check
    pub async fn nmt_is_node_found(&self, node_id: u8) -> bool {
        if let Some(nmt) = self.nmt_manager.read().await.as_ref() {
            nmt.is_node_found(node_id)
        } else {
            false
        }
    }

    /// Get the current state of a node
    ///
    /// # Arguments
    /// * `node_id` - Node ID to query
    pub async fn nmt_get_node_state(&self, node_id: u8) -> Option<canopen::nmt::NmtState> {
        if let Some(nmt) = self.nmt_manager.read().await.as_ref() {
            nmt.get_node_state(node_id).map(|state| state.state)
        } else {
            None
        }
    }

    /// Check if a node's heartbeat is within the timeout period
    ///
    /// # Arguments
    /// * `node_id` - Node ID to check
    /// * `timeout` - Maximum duration since last heartbeat
    pub async fn nmt_check_heartbeat(&self, node_id: u8, timeout: std::time::Duration) -> bool {
        if let Some(nmt) = self.nmt_manager.read().await.as_ref() {
            nmt.check_heartbeat(node_id, timeout)
        } else {
            false
        }
    }

    /// Get a list of all discovered nodes
    pub async fn nmt_get_discovered_nodes(&self) -> Vec<u8> {
        if let Some(nmt) = self.nmt_manager.read().await.as_ref() {
            nmt.get_discovered_nodes()
        } else {
            Vec::new()
        }
    }

    // PDO Methods

    /// Send a PDO message with the specified COB-ID and payload
    ///
    /// # Arguments
    /// * `cob_id` - COB-ID for the PDO (e.g., 0x200 for RPDO1 to node 0)
    /// * `payload` - Data bytes to send (up to 8 bytes for standard CAN)
    ///
    /// # Example
    /// ```ignore
    /// // Send RPDO1 to node 1 (COB-ID = 0x201)
    /// canopen.write_pdo(0x201, vec![0x01, 0x02, 0x03, 0x04]).await?;
    /// ```
    pub async fn write_pdo(&self, cob_id: u16, payload: Vec<u8>) -> Result<()> {
        let message = CanMessage::new(cob_id, payload)?;
        self.send_message(message).await
    }

    /// Register a callback handler for incoming PDO messages
    ///
    /// When a PDO is received with the specified COB-ID, the callback will be invoked
    /// with the PDO data payload.
    ///
    /// # Arguments
    /// * `cob_id` - COB-ID to listen for (e.g., 0x181 for TPDO1 from node 1)
    /// * `handler` - Callback function that receives PDO data bytes
    ///
    /// # Example
    /// ```ignore
    /// // Listen for TPDO1 from node 1
    /// canopen.register_pdo_handler(0x181, |data| {
    ///     println!("Received sensor data: {:?}", data);
    /// }).await;
    /// ```
    pub async fn register_pdo_handler<F>(&self, cob_id: u16, handler: F)
    where
        F: Fn(&[u8]) + Send + Sync + 'static,
    {
        self.pdo_manager
            .write()
            .await
            .register_pdo_handler(cob_id, handler);
    }

    /// Unregister a PDO callback handler
    ///
    /// # Arguments
    /// * `cob_id` - COB-ID to stop listening for
    pub async fn unregister_pdo_handler(&self, cob_id: u16) {
        self.pdo_manager
            .write()
            .await
            .unregister_pdo_handler(cob_id);
    }

    /// Get the most recently received PDO data for a specific COB-ID
    ///
    /// Returns None if no PDO has been received for this COB-ID.
    /// This is useful when not using callbacks.
    ///
    /// # Arguments
    /// * `cob_id` - COB-ID to query
    pub async fn get_recent_pdo(&self, cob_id: u16) -> Option<Vec<u8>> {
        self.pdo_manager.read().await.get_recent_pdo(cob_id)
    }

    /// Clear all stored recent PDO data
    pub async fn clear_recent_pdos(&self) {
        self.pdo_manager.write().await.clear_recent_pdos();
    }

    // SDO Methods

    /// Read an 8-bit unsigned integer from the object dictionary
    pub async fn sdo_read_u8(
        &self,
        node_id: u8,
        index: u16,
        subindex: u8,
        timeout_ms: u32,
    ) -> Result<u8> {
        let data = self
            .sdo_read_data(node_id, index, subindex, timeout_ms)
            .await?;
        if data.len() != 1 {
            return Err(CANopenError::InvalidData(format!(
                "Expected 1 byte for u8, got {}",
                data.len()
            )));
        }
        Ok(data[0])
    }

    /// Read a 16-bit unsigned integer from the object dictionary
    pub async fn sdo_read_u16(
        &self,
        node_id: u8,
        index: u16,
        subindex: u8,
        timeout_ms: u32,
    ) -> Result<u16> {
        let data = self
            .sdo_read_data(node_id, index, subindex, timeout_ms)
            .await?;
        if data.len() != 2 {
            return Err(CANopenError::InvalidData(format!(
                "Expected 2 bytes for u16, got {}",
                data.len()
            )));
        }
        Ok(u16::from_le_bytes([data[0], data[1]]))
    }

    /// Read a 32-bit unsigned integer from the object dictionary
    pub async fn sdo_read_u32(
        &self,
        node_id: u8,
        index: u16,
        subindex: u8,
        timeout_ms: u32,
    ) -> Result<u32> {
        let data = self
            .sdo_read_data(node_id, index, subindex, timeout_ms)
            .await?;
        if data.len() != 4 {
            return Err(CANopenError::InvalidData(format!(
                "Expected 4 bytes for u32, got {}",
                data.len()
            )));
        }
        Ok(u32::from_le_bytes([data[0], data[1], data[2], data[3]]))
    }

    /// Read raw data from the object dictionary
    pub async fn sdo_read_data(
        &self,
        node_id: u8,
        index: u16,
        subindex: u8,
        timeout_ms: u32,
    ) -> Result<Vec<u8>> {
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();

        let request = SdoRequest::new(
            node_id,
            SdoDirection::Upload,
            index,
            subindex,
            Vec::new(),
            timeout_ms,
            response_tx,
        );

        // Get the SDO request sender
        let sdo_tx = {
            let sdo_tx_lock = self.sdo_request_tx.read().await;
            sdo_tx_lock
                .as_ref()
                .ok_or(CANopenError::Connection)?
                .clone()
        };

        sdo_tx
            .send(request)
            .await
            .map_err(|_| CANopenError::ChannelClosed)?;

        match tokio::time::timeout(
            tokio::time::Duration::from_millis(timeout_ms as u64 + 1000),
            response_rx,
        )
        .await
        {
            Ok(Ok(data)) => data,
            Ok(Err(_)) => Err(CANopenError::ChannelClosed),
            Err(_) => Err(CANopenError::Timeout),
        }
    }

    /// Write an 8-bit unsigned integer to the object dictionary
    pub async fn sdo_write_u8(
        &self,
        node_id: u8,
        index: u16,
        subindex: u8,
        value: u8,
        timeout_ms: u32,
    ) -> Result<()> {
        self.sdo_write_data(node_id, index, subindex, vec![value], timeout_ms)
            .await
    }

    /// Write a 16-bit unsigned integer to the object dictionary
    pub async fn sdo_write_u16(
        &self,
        node_id: u8,
        index: u16,
        subindex: u8,
        value: u16,
        timeout_ms: u32,
    ) -> Result<()> {
        let bytes = value.to_le_bytes();
        self.sdo_write_data(node_id, index, subindex, bytes.to_vec(), timeout_ms)
            .await
    }

    /// Write a 32-bit unsigned integer to the object dictionary
    pub async fn sdo_write_u32(
        &self,
        node_id: u8,
        index: u16,
        subindex: u8,
        value: u32,
        timeout_ms: u32,
    ) -> Result<()> {
        let bytes = value.to_le_bytes();
        self.sdo_write_data(node_id, index, subindex, bytes.to_vec(), timeout_ms)
            .await
    }

    /// Write raw data to the object dictionary
    pub async fn sdo_write_data(
        &self,
        node_id: u8,
        index: u16,
        subindex: u8,
        data: Vec<u8>,
        timeout_ms: u32,
    ) -> Result<()> {
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();

        let request = SdoRequest::new(
            node_id,
            SdoDirection::Download,
            index,
            subindex,
            data,
            timeout_ms,
            response_tx,
        );

        // Get the SDO request sender
        let sdo_tx = {
            let sdo_tx_lock = self.sdo_request_tx.read().await;
            sdo_tx_lock
                .as_ref()
                .ok_or(CANopenError::Connection)?
                .clone()
        };

        sdo_tx
            .send(request)
            .await
            .map_err(|_| CANopenError::ChannelClosed)?;

        match tokio::time::timeout(
            tokio::time::Duration::from_millis(timeout_ms as u64 + 1000),
            response_rx,
        )
        .await
        {
            Ok(result) => {
                let _response = result.map_err(|_| CANopenError::ChannelClosed)?;
                Ok(())
            }
            Err(_) => Err(CANopenError::Timeout),
        }
    }

    // ===== SYNC Methods =====

    /// Send a SYNC message
    ///
    /// Broadcasts a SYNC message to all nodes on the network.
    /// If counter is enabled, the counter will be incremented automatically.
    ///
    /// # Example
    /// ```no_run
    /// # use libcanopen_client::*;
    /// # async fn example(canopen: &CANopenSimple) -> Result<()> {
    /// canopen.send_sync().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn send_sync(&self) -> Result<()> {
        let sync_lock = self.sync_manager.read().await;
        if let Some(sync) = sync_lock.as_ref() {
            let sync_msg = sync.create_sync_message();
            self.send_message(sync_msg).await?;
            Ok(())
        } else {
            Err(CANopenError::NotInitialized)
        }
    }

    /// Enable or disable SYNC counter
    ///
    /// When enabled, SYNC messages include a 1-byte counter (1-240, wrapping to 1).
    /// When disabled, SYNC messages have no data bytes.
    pub async fn set_sync_counter_enabled(&self, enabled: bool) -> Result<()> {
        let sync_lock = self.sync_manager.read().await;
        if let Some(sync) = sync_lock.as_ref() {
            sync.set_counter_enabled(enabled);
            Ok(())
        } else {
            Err(CANopenError::NotInitialized)
        }
    }

    /// Get current SYNC counter value
    pub async fn get_sync_counter(&self) -> Result<u8> {
        let sync_lock = self.sync_manager.read().await;
        if let Some(sync) = sync_lock.as_ref() {
            Ok(sync.get_counter())
        } else {
            Err(CANopenError::NotInitialized)
        }
    }

    /// Register a callback for SYNC reception
    ///
    /// The callback receives the SYNC counter value (0 if no counter present).
    ///
    /// # Example
    /// ```no_run
    /// # use libcanopen_client::*;
    /// # async fn example(canopen: &CANopenSimple) -> Result<()> {
    /// canopen.register_sync_callback(|counter| {
    ///     println!("SYNC received: counter={}", counter);
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn register_sync_callback<F>(&self, callback: F) -> Result<()>
    where
        F: Fn(u8) + Send + Sync + 'static,
    {
        let sync_lock = self.sync_manager.read().await;
        if let Some(sync) = sync_lock.as_ref() {
            sync.register_sync_callback(callback);
            Ok(())
        } else {
            Err(CANopenError::NotInitialized)
        }
    }

    /// Unregister SYNC callback
    pub async fn unregister_sync_callback(&self) -> Result<()> {
        let sync_lock = self.sync_manager.read().await;
        if let Some(sync) = sync_lock.as_ref() {
            sync.unregister_sync_callback();
            Ok(())
        } else {
            Err(CANopenError::NotInitialized)
        }
    }

    // ===== Emergency (EMCY) Methods =====

    /// Register an emergency message handler for a specific node
    ///
    /// # Arguments
    /// * `node_id` - The node ID to monitor (1-127)
    /// * `handler` - Callback function invoked when emergency is received
    ///
    /// # Example
    /// ```no_run
    /// # use libcanopen_client::*;
    /// # async fn example(canopen: &CANopenSimple) -> Result<()> {
    /// canopen.register_emcy_handler(5, |emcy| {
    ///     println!("Emergency from node {}: Error 0x{:04X} - {}",
    ///         emcy.node_id,
    ///         emcy.error_code,
    ///         emcy.error_code_description());
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn register_emcy_handler<F>(&self, node_id: u8, handler: F) -> Result<()>
    where
        F: Fn(&EmergencyMessage) + Send + Sync + 'static,
    {
        let emcy_lock = self.emcy_manager.read().await;
        emcy_lock.register_emcy_handler(node_id, handler);
        Ok(())
    }

    /// Unregister emergency handler for a node
    pub async fn unregister_emcy_handler(&self, node_id: u8) -> Result<()> {
        let emcy_lock = self.emcy_manager.read().await;
        emcy_lock.unregister_emcy_handler(node_id);
        Ok(())
    }

    /// Get the most recent emergency message from a node
    ///
    /// Returns `None` if no emergency has been received from this node.
    pub async fn get_recent_emcy(&self, node_id: u8) -> Option<EmergencyMessage> {
        let emcy_lock = self.emcy_manager.read().await;
        emcy_lock.get_recent_emcy(node_id)
    }

    /// Clear all stored emergency messages
    pub async fn clear_recent_emcy(&self) -> Result<()> {
        let emcy_lock = self.emcy_manager.read().await;
        emcy_lock.clear_recent_emcy();
        Ok(())
    }

    // ===== LSS (Layer Setting Services) Methods =====

    /// Switch LSS to global state (affects all unconfigured slaves)
    ///
    /// # Arguments
    /// * `mode` - LssMode::Waiting or LssMode::Configuration
    ///
    /// # Example
    /// ```no_run
    /// # use libcanopen_client::*;
    /// # async fn example(canopen: &CANopenSimple) -> Result<()> {
    /// // Switch all unconfigured slaves to configuration mode
    /// canopen.lss_switch_state_global(LssMode::Configuration).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn lss_switch_state_global(&self, mode: LssMode) -> Result<()> {
        let lss_lock = self.lss_manager.read().await;
        if let Some(lss) = lss_lock.as_ref() {
            lss.switch_state_global(mode).await
        } else {
            Err(CANopenError::NotInitialized)
        }
    }

    /// Switch LSS to selective state (select specific slave by LSS address)
    ///
    /// # Arguments
    /// * `address` - LSS address (Vendor-ID, Product-Code, Revision, Serial Number)
    ///
    /// # Example
    /// ```no_run
    /// # use libcanopen_client::*;
    /// # async fn example(canopen: &CANopenSimple) -> Result<()> {
    /// let address = LssAddress {
    ///     vendor_id: 0x00000000,
    ///     product_code: 0x12345678,
    ///     revision_number: 0x00010000,
    ///     serial_number: 0xABCDEF00,
    /// };
    /// canopen.lss_switch_state_selective(&address).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn lss_switch_state_selective(&self, address: &LssAddress) -> Result<()> {
        let lss_lock = self.lss_manager.read().await;
        if let Some(lss) = lss_lock.as_ref() {
            lss.switch_state_selective(address).await
        } else {
            Err(CANopenError::NotInitialized)
        }
    }

    /// Configure node-ID for selected LSS slave
    ///
    /// # Arguments
    /// * `node_id` - New node ID (1-127)
    /// * `timeout_ms` - Timeout in milliseconds
    ///
    /// # Returns
    /// * `Ok(LssError)` - LSS error code (Success if OK)
    ///
    /// # Example
    /// ```no_run
    /// # use libcanopen_client::*;
    /// # async fn example(canopen: &CANopenSimple) -> Result<()> {
    /// let error = canopen.lss_configure_node_id(10, 1000).await?;
    /// if error == LssError::Success {
    ///     println!("Node ID configured successfully");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn lss_configure_node_id(&self, node_id: u8, timeout_ms: u32) -> Result<LssError> {
        let lss_lock = self.lss_manager.read().await;
        if let Some(lss) = lss_lock.as_ref() {
            lss.configure_node_id(node_id, timeout_ms).await
        } else {
            Err(CANopenError::NotInitialized)
        }
    }

    /// Configure bit-rate for selected LSS slave
    ///
    /// # Arguments
    /// * `table_selector` - Bit timing table selector (0 = CiA 301)
    /// * `table_index` - Index in table (0=1Mbps, 1=800kbps, 2=500kbps, etc.)
    /// * `timeout_ms` - Timeout in milliseconds
    ///
    /// Common table_index values (CiA 301):
    /// - 0: 1000 kbit/s
    /// - 1: 800 kbit/s
    /// - 2: 500 kbit/s
    /// - 3: 250 kbit/s
    /// - 4: 125 kbit/s
    /// - 5: 50 kbit/s
    /// - 6: 20 kbit/s
    /// - 7: 10 kbit/s
    pub async fn lss_configure_bit_rate(
        &self,
        table_selector: u8,
        table_index: u8,
        timeout_ms: u32,
    ) -> Result<LssError> {
        let lss_lock = self.lss_manager.read().await;
        if let Some(lss) = lss_lock.as_ref() {
            lss.configure_bit_rate(table_selector, table_index, timeout_ms)
                .await
        } else {
            Err(CANopenError::NotInitialized)
        }
    }

    /// Activate configured bit-rate
    ///
    /// # Arguments
    /// * `switch_delay_ms` - Delay before switching (in milliseconds)
    ///
    /// Note: After calling this, you must also switch your CAN hardware
    /// to the new bit-rate after the delay!
    pub async fn lss_activate_bit_rate(&self, switch_delay_ms: u16) -> Result<()> {
        let lss_lock = self.lss_manager.read().await;
        if let Some(lss) = lss_lock.as_ref() {
            lss.activate_bit_rate(switch_delay_ms).await
        } else {
            Err(CANopenError::NotInitialized)
        }
    }

    /// Store configuration to non-volatile memory
    ///
    /// Saves the configured node-ID and bit-rate to the slave's persistent storage.
    pub async fn lss_store_configuration(&self, timeout_ms: u32) -> Result<LssError> {
        let lss_lock = self.lss_manager.read().await;
        if let Some(lss) = lss_lock.as_ref() {
            lss.store_configuration(timeout_ms).await
        } else {
            Err(CANopenError::NotInitialized)
        }
    }

    /// Inquire LSS address from selected slave
    ///
    /// Returns the LSS address (Vendor-ID, Product-Code, Revision, Serial Number)
    ///
    /// # Example
    /// ```no_run
    /// # use libcanopen_client::*;
    /// # async fn example(canopen: &CANopenSimple) -> Result<()> {
    /// let address = canopen.lss_inquire_address(1000).await?;
    /// println!("Vendor ID: 0x{:08X}", address.vendor_id);
    /// println!("Product Code: 0x{:08X}", address.product_code);
    /// println!("Revision: 0x{:08X}", address.revision_number);
    /// println!("Serial: 0x{:08X}", address.serial_number);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn lss_inquire_address(&self, timeout_ms: u32) -> Result<LssAddress> {
        let lss_lock = self.lss_manager.read().await;
        if let Some(lss) = lss_lock.as_ref() {
            lss.inquire_lss_address(timeout_ms).await
        } else {
            Err(CANopenError::NotInitialized)
        }
    }

    /// Inquire current node-ID from selected slave
    pub async fn lss_inquire_node_id(&self, timeout_ms: u32) -> Result<u8> {
        let lss_lock = self.lss_manager.read().await;
        if let Some(lss) = lss_lock.as_ref() {
            lss.inquire_node_id(timeout_ms).await
        } else {
            Err(CANopenError::NotInitialized)
        }
    }

    /// Identify remote slave
    ///
    /// Check if a slave with the previously selected LSS address responds.
    ///
    /// # Returns
    /// * `Ok(true)` - Slave responded
    /// * `Ok(false)` - No response (timeout)
    pub async fn lss_identify_remote_slave(&self, timeout_ms: u32) -> Result<bool> {
        let lss_lock = self.lss_manager.read().await;
        if let Some(lss) = lss_lock.as_ref() {
            lss.identify_remote_slave(timeout_ms).await
        } else {
            Err(CANopenError::NotInitialized)
        }
    }
}
