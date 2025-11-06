# libCANopen Simple - C# to Rust Migration Plan

## Overview
This document outlines the detailed steps to migrate the existing C# CANopen library (`libCanopenSimple`) to Rust. The original library provides a simple CANopen implementation with callbacks for different COB types (NMT/PDO/SDO), NMT controls, and SDO client functionality.

## Current C# Architecture Analysis

### Core Components
1. **Main Library** (`libCanopenSimple.cs`)
   - CANopen message handling and routing
   - Event-driven architecture with callbacks
   - SDO client implementation
   - NMT state management
   - PDO handling
   - CAN hardware abstraction

2. **SDO Module** (`SDO.cs`)
   - SDO transfer state machine
   - Expedited and segmented transfers
   - Timeout handling
   - Queue management

3. **NMT State** (`NMTState.cs`) 
   - Node state tracking
   - State transition callbacks
   - Heartbeat/node guarding

4. **Hardware Layer** (`can_hw/can.cs`)
   - PEAK CAN USB adapter support
   - NI-XNET support (currently unused in favor of PEAK)
   - Message reception threading
   - Bus statistics and error handling

## Migration Strategy

### Phase 1: Project Setup and Dependencies

#### Step 1.1: Create Rust Project Structure
```bash
# In the libCanOpenSimple_rs directory
cargo init --lib
```

#### Step 1.2: Configure Cargo.toml
Add the following dependencies to `Cargo.toml`:
```toml
[dependencies]
peak-can-sys = "0.1.2"          # PEAK CAN hardware interface
tokio = { version = "1.0", features = ["full"] }  # Async runtime
serde = { version = "1.0", features = ["derive"] } # Serialization
log = "0.4"                      # Logging
thiserror = "1.0"                # Error handling
crossbeam = "0.8"                # Concurrent data structures
futures = "0.3"                  # Async utilities
bitflags = "1.3"                 # Bit flag operations

[dev-dependencies]
tokio-test = "0.4"
```

#### Step 1.3: Directory Structure
```
src/
├── lib.rs                 # Main library entry point
├── canopen/
│   ├── mod.rs             # CANopen module
│   ├── message.rs         # CAN message types and COB definitions
│   ├── sdo.rs             # SDO client implementation  
│   ├── nmt.rs             # NMT state management
│   ├── pdo.rs             # PDO handling
│   └── events.rs          # Event system and callbacks
├── hardware/
│   ├── mod.rs             # Hardware abstraction layer
│   └── peak_can.rs        # PEAK CAN adapter implementation
├── errors.rs              # Error types
└── utils.rs               # Utility functions
```

### Phase 2: Core Data Types and Message Handling

#### Step 2.1: Define CAN Message Types (`src/canopen/message.rs`)
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanId(pub u16);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanMessage {
    pub id: CanId,
    pub data: Vec<u8>,
    pub timestamp: Option<u64>,
}

// COB-ID ranges for different message types
pub mod cob_ids {
    pub const NMT_COMMAND: u16 = 0x000;
    pub const SYNC: u16 = 0x080;
    pub const EMERGENCY_BASE: u16 = 0x080;
    pub const TIME_STAMP: u16 = 0x100;
    pub const PDO1_TX_BASE: u16 = 0x180;
    pub const PDO1_RX_BASE: u16 = 0x200;
    pub const PDO2_TX_BASE: u16 = 0x280;
    pub const PDO2_RX_BASE: u16 = 0x300;
    pub const PDO3_TX_BASE: u16 = 0x380;
    pub const PDO3_RX_BASE: u16 = 0x400;
    pub const PDO4_TX_BASE: u16 = 0x480;
    pub const PDO4_RX_BASE: u16 = 0x500;
    pub const SDO_TX_BASE: u16 = 0x580;
    pub const SDO_RX_BASE: u16 = 0x600;
    pub const NMT_ERROR_CONTROL_BASE: u16 = 0x700;
    pub const LSS_BASE: u16 = 0x7E4;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    Nmt,
    Sync,
    Emergency,
    TimeStamp,
    Pdo,
    Sdo,
    NmtErrorControl,
    Lss,
    Unknown,
}

impl CanMessage {
    pub fn message_type(&self) -> MessageType {
        match self.id.0 {
            cob_ids::NMT_COMMAND => MessageType::Nmt,
            cob_ids::SYNC => MessageType::Sync,
            cob_ids::TIME_STAMP => MessageType::TimeStamp,
            0x080..=0xFF => MessageType::Emergency,
            0x180..=0x57F => MessageType::Pdo,
            0x580..=0x67F => MessageType::Sdo,
            0x700..=0x77F => MessageType::NmtErrorControl,
            0x7E4..=0x7E5 => MessageType::Lss,
            _ => MessageType::Unknown,
        }
    }
}
```

#### Step 2.2: Error Types (`src/errors.rs`)
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CANopenError {
    #[error("Hardware error: {0}")]
    Hardware(String),
    
    #[error("SDO error: code {code:08X}")]
    Sdo { code: u32 },
    
    #[error("Timeout occurred")]
    Timeout,
    
    #[error("Invalid message format")]
    InvalidMessage,
    
    #[error("Node not found: {node_id}")]
    NodeNotFound { node_id: u8 },
    
    #[error("Connection error")]
    Connection,
    
    #[error("PEAK CAN error: {0}")]
    PeakCan(#[from] peak_can_sys::Error),
}

pub type Result<T> = std::result::Result<T, CANopenError>;
```

### Phase 3: Hardware Abstraction Layer

#### Step 3.1: Hardware Trait (`src/hardware/mod.rs`)
```rust
use async_trait::async_trait;
use tokio::sync::mpsc;
use crate::{CanMessage, Result};

#[async_trait]
pub trait CanHardware {
    async fn connect(&mut self) -> Result<()>;
    async fn disconnect(&mut self) -> Result<()>;
    async fn send_message(&self, message: &CanMessage) -> Result<()>;
    fn subscribe_messages(&self) -> mpsc::Receiver<CanMessage>;
    fn is_connected(&self) -> bool;
}

pub mod peak_can;
```

#### Step 3.2: PEAK CAN Implementation (`src/hardware/peak_can.rs`)
```rust
use async_trait::async_trait;
use peak_can_sys::*;
use tokio::sync::{mpsc, RwLock};
use std::sync::Arc;
use crate::{CanMessage, CanId, Result, CANopenError, hardware::CanHardware};

pub struct PeakCanAdapter {
    handle: Option<PcanHandle>,
    baudrate: PcanBaudrate,
    is_connected: Arc<RwLock<bool>>,
    message_sender: mpsc::Sender<CanMessage>,
    message_receiver: Option<mpsc::Receiver<CanMessage>>,
}

#[derive(Debug, Clone, Copy)]
pub enum BusSpeed {
    Baud10K,
    Baud20K,
    Baud50K,
    Baud100K,
    Baud125K,
    Baud250K,
    Baud500K,
    Baud800K,
    Baud1M,
}

impl From<BusSpeed> for PcanBaudrate {
    fn from(speed: BusSpeed) -> Self {
        match speed {
            BusSpeed::Baud1M => PcanBaudrate::Pcan1MBaud,
            BusSpeed::Baud800K => PcanBaudrate::Pcan800kBaud,
            BusSpeed::Baud500K => PcanBaudrate::Pcan500kBaud,
            BusSpeed::Baud250K => PcanBaudrate::Pcan250kBaud,
            BusSpeed::Baud125K => PcanBaudrate::Pcan125kBaud,
            BusSpeed::Baud100K => PcanBaudrate::Pcan100kBaud,
            BusSpeed::Baud50K => PcanBaudrate::Pcan50kBaud,
            BusSpeed::Baud20K => PcanBaudrate::Pcan20kBaud,
            BusSpeed::Baud10K => PcanBaudrate::Pcan10kBaud,
        }
    }
}

impl PeakCanAdapter {
    pub fn new(handle: PcanHandle, speed: BusSpeed) -> Self {
        let (tx, rx) = mpsc::channel(1000);
        Self {
            handle: Some(handle),
            baudrate: speed.into(),
            is_connected: Arc::new(RwLock::new(false)),
            message_sender: tx,
            message_receiver: Some(rx),
        }
    }

    async fn start_receive_thread(&self) -> Result<()> {
        let handle = self.handle.ok_or(CANopenError::Hardware("No handle".to_string()))?;
        let sender = self.message_sender.clone();
        let is_connected = self.is_connected.clone();
        
        tokio::spawn(async move {
            let mut msg = PcanMsg::default();
            let mut timestamp = PcanTimestamp::default();
            
            while *is_connected.read().await {
                match pcan_read(handle, &mut msg, &mut timestamp) {
                    Ok(_) if msg.msg_type == PcanMessageType::PcanMessageStandard => {
                        let can_msg = CanMessage {
                            id: CanId(msg.id as u16),
                            data: msg.data[..msg.len as usize].to_vec(),
                            timestamp: Some(timestamp.micros as u64),
                        };
                        
                        if sender.send(can_msg).await.is_err() {
                            break; // Channel closed
                        }
                    }
                    Ok(_) => {
                        // Handle other message types (error frames, etc.)
                        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
                    }
                    Err(_) => {
                        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
                    }
                }
            }
        });
        
        Ok(())
    }
}

#[async_trait]
impl CanHardware for PeakCanAdapter {
    async fn connect(&mut self) -> Result<()> {
        let handle = self.handle.ok_or(CANopenError::Hardware("No handle".to_string()))?;
        
        pcan_initialize(handle, self.baudrate, None, None, None)?;
        
        *self.is_connected.write().await = true;
        
        self.start_receive_thread().await?;
        
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        *self.is_connected.write().await = false;
        
        if let Some(handle) = self.handle {
            pcan_uninitialize(handle)?;
        }
        
        Ok(())
    }

    async fn send_message(&self, message: &CanMessage) -> Result<()> {
        let handle = self.handle.ok_or(CANopenError::Hardware("No handle".to_string()))?;
        
        let mut pcan_msg = PcanMsg {
            id: message.id.0 as u32,
            msg_type: PcanMessageType::PcanMessageStandard,
            len: message.data.len() as u8,
            data: [0; 8],
        };
        
        pcan_msg.data[..message.data.len()].copy_from_slice(&message.data);
        
        pcan_write(handle, &pcan_msg)?;
        
        Ok(())
    }

    fn subscribe_messages(&self) -> mpsc::Receiver<CanMessage> {
        let (tx, rx) = mpsc::channel(1000);
        // Implementation would need to clone messages to new subscriber
        // This is a simplified version
        rx
    }

    fn is_connected(&self) -> bool {
        // This should be implemented properly with async/await
        false // Placeholder
    }
}
```

### Phase 4: SDO Client Implementation

#### Step 4.1: SDO Types (`src/canopen/sdo.rs`)
```rust
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot};
use crate::{CanMessage, CanId, Result, CANopenError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SdoDirection {
    Read,
    Write,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SdoState {
    Init,
    Sent,
    Handshake,
    Finished,
    Error,
}

#[derive(Debug)]
pub struct SdoTransfer {
    pub node_id: u8,
    pub index: u16,
    pub subindex: u8,
    pub direction: SdoDirection,
    pub data: Vec<u8>,
    pub state: SdoState,
    pub expedited: bool,
    pub total_data: u32,
    pub transferred_data: u32,
    pub last_toggle: bool,
    pub timeout: Instant,
    pub completion_sender: Option<oneshot::Sender<Result<Vec<u8>>>>,
}

pub struct SdoClient {
    active_transfers: HashMap<u16, SdoTransfer>, // Key: SDO response COB-ID
    transfer_queue: mpsc::Receiver<SdoTransfer>,
    message_sender: mpsc::Sender<CanMessage>,
    default_timeout: Duration,
}

impl SdoClient {
    pub fn new(message_sender: mpsc::Sender<CanMessage>) -> (Self, mpsc::Sender<SdoTransfer>) {
        let (tx, rx) = mpsc::channel(100);
        let client = Self {
            active_transfers: HashMap::new(),
            transfer_queue: rx,
            message_sender,
            default_timeout: Duration::from_secs(5),
        };
        (client, tx)
    }

    pub async fn process_message(&mut self, message: &CanMessage) -> Result<()> {
        if message.id.0 < 0x580 || message.id.0 >= 0x600 {
            return Ok(()); // Not an SDO response
        }

        if let Some(transfer) = self.active_transfers.get_mut(&message.id.0) {
            self.handle_sdo_response(transfer, message).await?;
            
            if matches!(transfer.state, SdoState::Finished | SdoState::Error) {
                let completed = self.active_transfers.remove(&message.id.0).unwrap();
                self.complete_transfer(completed).await;
            }
        }

        Ok(())
    }

    async fn handle_sdo_response(&mut self, transfer: &mut SdoTransfer, message: &CanMessage) -> Result<()> {
        if message.data.len() != 8 {
            return Err(CANopenError::InvalidMessage);
        }

        let scs = (message.data[0] >> 5) & 0x07; // Server Command Specifier
        
        match scs {
            0x04 => {
                // Abort transfer
                let error_code = u32::from_le_bytes([
                    message.data[4], message.data[5], message.data[6], message.data[7]
                ]);
                transfer.state = SdoState::Error;
                return Err(CANopenError::Sdo { code: error_code });
            }
            0x02 => {
                // Initiate upload response (read)
                let e = (message.data[0] >> 1) & 0x01; // Expedited transfer
                let s = message.data[0] & 0x01; // Size indicator
                
                if e == 1 {
                    // Expedited transfer
                    let n = (message.data[0] >> 2) & 0x03;
                    let data_len = if s == 1 { 4 - n } else { 4 };
                    transfer.data = message.data[4..4 + data_len as usize].to_vec();
                    transfer.state = SdoState::Finished;
                } else {
                    // Segmented transfer
                    let data_size = u32::from_le_bytes([
                        message.data[4], message.data[5], message.data[6], message.data[7]
                    ]);
                    transfer.total_data = data_size;
                    transfer.data = vec![0; data_size as usize];
                    transfer.transferred_data = 0;
                    transfer.state = SdoState::Handshake;
                    self.request_next_segment(transfer, false).await?;
                }
            }
            0x00 => {
                // Upload segment response
                let t = (message.data[0] >> 4) & 0x01; // Toggle bit
                let n = (message.data[0] >> 1) & 0x07; // Number of bytes not containing data
                let c = message.data[0] & 0x01; // Last segment indicator
                
                let segment_size = 7 - n as usize;
                let start_idx = transfer.transferred_data as usize;
                let end_idx = start_idx + segment_size;
                
                if end_idx <= transfer.data.len() {
                    transfer.data[start_idx..end_idx]
                        .copy_from_slice(&message.data[1..1 + segment_size]);
                }
                
                transfer.transferred_data += segment_size as u32;
                
                if c == 1 || transfer.transferred_data >= transfer.total_data {
                    transfer.state = SdoState::Finished;
                } else {
                    transfer.last_toggle = !transfer.last_toggle;
                    self.request_next_segment(transfer, transfer.last_toggle).await?;
                }
            }
            0x03 => {
                // Initiate download response (write)
                if transfer.expedited {
                    transfer.state = SdoState::Finished;
                } else {
                    transfer.state = SdoState::Handshake;
                    self.send_next_segment(transfer, false).await?;
                }
            }
            0x01 => {
                // Download segment response  
                if transfer.transferred_data < transfer.total_data {
                    transfer.last_toggle = !transfer.last_toggle;
                    self.send_next_segment(transfer, transfer.last_toggle).await?;
                } else {
                    transfer.state = SdoState::Finished;
                }
            }
            _ => {
                return Err(CANopenError::InvalidMessage);
            }
        }

        Ok(())
    }

    async fn request_next_segment(&self, transfer: &SdoTransfer, toggle: bool) -> Result<()> {
        let mut cmd = 0x60u8;
        if toggle {
            cmd |= 0x10;
        }

        let message = CanMessage {
            id: CanId(0x600 + transfer.node_id as u16),
            data: vec![cmd, 0, 0, 0, 0, 0, 0, 0],
            timestamp: None,
        };

        self.message_sender.send(message).await
            .map_err(|_| CANopenError::Hardware("Failed to send message".to_string()))?;

        Ok(())
    }

    async fn send_next_segment(&self, transfer: &mut SdoTransfer, toggle: bool) -> Result<()> {
        let mut cmd = 0x00u8;
        if toggle {
            cmd |= 0x10;
        }

        let remaining = transfer.total_data - transfer.transferred_data;
        let segment_size = std::cmp::min(7, remaining) as usize;
        
        let mut data = vec![cmd];
        
        if segment_size > 0 {
            let start_idx = transfer.transferred_data as usize;
            data.extend_from_slice(&transfer.data[start_idx..start_idx + segment_size]);
        }
        
        // Pad to 8 bytes
        data.resize(8, 0);
        
        // Set 'c' bit if this is the last segment
        if remaining <= 7 {
            data[0] |= 0x01;
        }
        
        // Set 'n' field (number of bytes that don't contain data)
        if segment_size < 7 {
            let n = 7 - segment_size;
            data[0] |= ((n as u8) << 1);
        }

        let message = CanMessage {
            id: CanId(0x600 + transfer.node_id as u16),
            data,
            timestamp: None,
        };

        self.message_sender.send(message).await
            .map_err(|_| CANopenError::Hardware("Failed to send message".to_string()))?;

        transfer.transferred_data += segment_size as u32;
        
        Ok(())
    }

    async fn complete_transfer(&self, transfer: SdoTransfer) {
        if let Some(sender) = transfer.completion_sender {
            let result = match transfer.state {
                SdoState::Finished => Ok(transfer.data),
                SdoState::Error => Err(CANopenError::Sdo { code: 0 }), // Error code would be stored in transfer
                _ => Err(CANopenError::Timeout),
            };
            let _ = sender.send(result);
        }
    }

    pub async fn run(&mut self) {
        loop {
            tokio::select! {
                Some(mut transfer) = self.transfer_queue.recv() => {
                    transfer.timeout = Instant::now() + self.default_timeout;
                    transfer.state = SdoState::Init;
                    
                    let response_cob = 0x580 + transfer.node_id as u16;
                    
                    match self.initiate_transfer(&transfer).await {
                        Ok(_) => {
                            transfer.state = SdoState::Sent;
                            self.active_transfers.insert(response_cob, transfer);
                        }
                        Err(_) => {
                            // Complete transfer with error
                            self.complete_transfer(transfer).await;
                        }
                    }
                }
                _ = tokio::time::sleep(Duration::from_millis(100)) => {
                    // Check for timeouts
                    let now = Instant::now();
                    let mut timed_out = Vec::new();
                    
                    for (cob_id, transfer) in &self.active_transfers {
                        if now > transfer.timeout {
                            timed_out.push(*cob_id);
                        }
                    }
                    
                    for cob_id in timed_out {
                        if let Some(mut transfer) = self.active_transfers.remove(&cob_id) {
                            transfer.state = SdoState::Error;
                            self.complete_transfer(transfer).await;
                        }
                    }
                }
            }
        }
    }

    async fn initiate_transfer(&self, transfer: &SdoTransfer) -> Result<()> {
        let message = match transfer.direction {
            SdoDirection::Read => {
                CanMessage {
                    id: CanId(0x600 + transfer.node_id as u16),
                    data: vec![
                        0x40,
                        (transfer.index & 0xFF) as u8,
                        (transfer.index >> 8) as u8,
                        transfer.subindex,
                        0, 0, 0, 0
                    ],
                    timestamp: None,
                }
            }
            SdoDirection::Write => {
                self.create_write_message(transfer)?
            }
        };

        self.message_sender.send(message).await
            .map_err(|_| CANopenError::Hardware("Failed to send message".to_string()))?;

        Ok(())
    }

    fn create_write_message(&self, transfer: &SdoTransfer) -> Result<CanMessage> {
        let mut data = vec![0u8; 8];
        
        // Set index and subindex
        data[1] = (transfer.index & 0xFF) as u8;
        data[2] = (transfer.index >> 8) as u8;
        data[3] = transfer.subindex;
        
        match transfer.data.len() {
            1 => {
                data[0] = 0x2F; // Expedited, 1 byte
                data[4] = transfer.data[0];
            }
            2 => {
                data[0] = 0x2B; // Expedited, 2 bytes
                data[4..6].copy_from_slice(&transfer.data);
            }
            3 => {
                data[0] = 0x27; // Expedited, 3 bytes
                data[4..7].copy_from_slice(&transfer.data);
            }
            4 => {
                data[0] = 0x23; // Expedited, 4 bytes
                data[4..8].copy_from_slice(&transfer.data);
            }
            _ => {
                data[0] = 0x21; // Segmented transfer
                let size_bytes = (transfer.data.len() as u32).to_le_bytes();
                data[4..8].copy_from_slice(&size_bytes);
            }
        }

        Ok(CanMessage {
            id: CanId(0x600 + transfer.node_id as u16),
            data,
            timestamp: None,
        })
    }
}
```

### Phase 5: NMT State Management

#### Step 5.1: NMT Implementation (`src/canopen/nmt.rs`)
```rust
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use tokio::sync::mpsc;
use crate::{CanMessage, CanId, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NmtState {
    Invalid = 0xFF,
    Boot = 0x00,
    Stopped = 0x04,
    Operational = 0x05,
    PreOperational = 0x7F,
}

impl From<u8> for NmtState {
    fn from(value: u8) -> Self {
        match value {
            0x00 => NmtState::Boot,
            0x04 => NmtState::Stopped,
            0x05 => NmtState::Operational,
            0x7F => NmtState::PreOperational,
            _ => NmtState::Invalid,
        }
    }
}

#[derive(Debug, Clone)]
pub struct NodeState {
    pub state: NmtState,
    pub last_state: NmtState,
    pub last_heartbeat: SystemTime,
    pub node_id: u8,
}

impl NodeState {
    pub fn new(node_id: u8) -> Self {
        Self {
            state: NmtState::Invalid,
            last_state: NmtState::Invalid,
            last_heartbeat: SystemTime::now(),
            node_id,
        }
    }

    pub fn update_state(&mut self, new_state: NmtState) {
        self.last_state = self.state;
        self.state = new_state;
        self.last_heartbeat = SystemTime::now();
    }

    pub fn is_alive(&self, max_age: Duration) -> bool {
        self.last_heartbeat.elapsed().unwrap_or(Duration::MAX) < max_age
    }
}

pub type StateChangeCallback = Box<dyn Fn(u8, NmtState, NmtState) + Send + Sync>;

pub struct NmtManager {
    nodes: HashMap<u8, NodeState>,
    message_sender: mpsc::Sender<CanMessage>,
    state_change_callbacks: Vec<StateChangeCallback>,
}

impl NmtManager {
    pub fn new(message_sender: mpsc::Sender<CanMessage>) -> Self {
        let mut nodes = HashMap::new();
        
        // Pre-allocate all possible node states (1-127)
        for node_id in 1..=127u8 {
            nodes.insert(node_id, NodeState::new(node_id));
        }

        Self {
            nodes,
            message_sender,
            state_change_callbacks: Vec::new(),
        }
    }

    pub fn add_state_change_callback<F>(&mut self, callback: F)
    where
        F: Fn(u8, NmtState, NmtState) + Send + Sync + 'static,
    {
        self.state_change_callbacks.push(Box::new(callback));
    }

    pub async fn process_heartbeat(&mut self, message: &CanMessage) -> Result<()> {
        if message.id.0 < 0x700 || message.id.0 > 0x77F {
            return Ok(()); // Not a heartbeat message
        }

        if message.data.is_empty() {
            return Ok(());
        }

        let node_id = (message.id.0 & 0x7F) as u8;
        let new_state = NmtState::from(message.data[0]);

        if let Some(node) = self.nodes.get_mut(&node_id) {
            let old_state = node.state;
            node.update_state(new_state);

            // Trigger callbacks if state changed
            if old_state != new_state {
                for callback in &self.state_change_callbacks {
                    callback(node_id, old_state, new_state);
                }
            }
        }

        Ok(())
    }

    pub fn get_node_state(&self, node_id: u8) -> Option<&NodeState> {
        self.nodes.get(&node_id)
    }

    pub fn is_node_found(&self, node_id: u8) -> bool {
        self.nodes.get(&node_id)
            .map(|node| node.state != NmtState::Invalid)
            .unwrap_or(false)
    }

    pub fn check_node_guard(&self, node_id: u8, max_age: Duration) -> bool {
        self.nodes.get(&node_id)
            .map(|node| node.is_alive(max_age))
            .unwrap_or(false)
    }

    // NMT command helpers
    pub async fn start_node(&self, node_id: u8) -> Result<()> {
        self.send_nmt_command(0x01, node_id).await
    }

    pub async fn stop_node(&self, node_id: u8) -> Result<()> {
        self.send_nmt_command(0x02, node_id).await
    }

    pub async fn enter_preoperational(&self, node_id: u8) -> Result<()> {
        self.send_nmt_command(0x80, node_id).await
    }

    pub async fn reset_node(&self, node_id: u8) -> Result<()> {
        self.send_nmt_command(0x81, node_id).await
    }

    pub async fn reset_communication(&self, node_id: u8) -> Result<()> {
        self.send_nmt_command(0x82, node_id).await
    }

    async fn send_nmt_command(&self, command: u8, node_id: u8) -> Result<()> {
        let message = CanMessage {
            id: CanId(0x000),
            data: vec![command, node_id],
            timestamp: None,
        };

        self.message_sender.send(message).await
            .map_err(|_| crate::CANopenError::Hardware("Failed to send NMT command".to_string()))?;

        Ok(())
    }
}
```

### Phase 6: Main Library Implementation

#### Step 6.1: Event System (`src/canopen/events.rs`)
```rust
use tokio::sync::broadcast;
use crate::CanMessage;
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub struct MessageEvent {
    pub message: CanMessage,
    pub timestamp: SystemTime,
}

pub type MessageCallback = Box<dyn Fn(&MessageEvent) + Send + Sync>;

pub struct EventManager {
    // Different event channels for different message types
    pub packet_tx: broadcast::Sender<MessageEvent>,
    pub sdo_tx: broadcast::Sender<MessageEvent>,
    pub nmt_tx: broadcast::Sender<MessageEvent>,
    pub pdo_tx: broadcast::Sender<MessageEvent>,
    pub emergency_tx: broadcast::Sender<MessageEvent>,
    pub sync_tx: broadcast::Sender<MessageEvent>,
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
        }
    }

    pub fn emit_message(&self, message: CanMessage) {
        let event = MessageEvent {
            message: message.clone(),
            timestamp: SystemTime::now(),
        };

        // Always emit to general packet channel
        let _ = self.packet_tx.send(event.clone());

        // Emit to specific channels based on message type
        match message.message_type() {
            crate::canopen::message::MessageType::Sdo => {
                let _ = self.sdo_tx.send(event);
            }
            crate::canopen::message::MessageType::Nmt | 
            crate::canopen::message::MessageType::NmtErrorControl => {
                let _ = self.nmt_tx.send(event);
            }
            crate::canopen::message::MessageType::Pdo => {
                let _ = self.pdo_tx.send(event);
            }
            crate::canopen::message::MessageType::Emergency => {
                let _ = self.emergency_tx.send(event);
            }
            crate::canopen::message::MessageType::Sync => {
                let _ = self.sync_tx.send(event);
            }
            _ => {}
        }
    }
}
```

#### Step 6.2: Main Library (`src/lib.rs`)
```rust
mod canopen;
mod hardware;
mod errors;
mod utils;

pub use errors::{CANopenError, Result};
pub use canopen::message::{CanMessage, CanId, MessageType};
pub use canopen::sdo::{SdoClient, SdoDirection, SdoTransfer};
pub use canopen::nmt::{NmtManager, NmtState, NodeState};
pub use canopen::events::{EventManager, MessageEvent};
pub use hardware::{CanHardware, peak_can::PeakCanAdapter, peak_can::BusSpeed};

use tokio::sync::{mpsc, oneshot};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct CANopenSimple {
    hardware: Arc<RwLock<Box<dyn CanHardware + Send + Sync>>>,
    event_manager: Arc<EventManager>,
    nmt_manager: Arc<RwLock<NmtManager>>,
    message_tx: mpsc::Sender<CanMessage>,
    sdo_transfer_tx: mpsc::Sender<canopen::sdo::SdoTransfer>,
    is_running: Arc<RwLock<bool>>,
}

impl CANopenSimple {
    pub fn new(hardware: Box<dyn CanHardware + Send + Sync>) -> Self {
        let (message_tx, message_rx) = mpsc::channel(1000);
        let event_manager = Arc::new(EventManager::new());
        let nmt_manager = Arc::new(RwLock::new(NmtManager::new(message_tx.clone())));
        
        let (sdo_client, sdo_transfer_tx) = canopen::sdo::SdoClient::new(message_tx.clone());

        Self {
            hardware: Arc::new(RwLock::new(hardware)),
            event_manager,
            nmt_manager,
            message_tx,
            sdo_transfer_tx,
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    pub async fn connect(&mut self) -> Result<()> {
        self.hardware.write().await.connect().await?;
        *self.is_running.write().await = true;
        
        // Start message processing task
        self.start_message_processing().await;
        
        Ok(())
    }

    pub async fn disconnect(&mut self) -> Result<()> {
        *self.is_running.write().await = false;
        self.hardware.write().await.disconnect().await?;
        Ok(())
    }

    pub async fn send_message(&self, message: CanMessage) -> Result<()> {
        self.hardware.read().await.send_message(&message).await?;
        self.event_manager.emit_message(message);
        Ok(())
    }

    async fn start_message_processing(&self) {
        let hardware = Arc::clone(&self.hardware);
        let event_manager = Arc::clone(&self.event_manager);
        let nmt_manager = Arc::clone(&self.nmt_manager);
        let is_running = Arc::clone(&self.is_running);

        tokio::spawn(async move {
            // This would need to be implemented properly with the hardware subscription
            // let mut message_rx = hardware.read().await.subscribe_messages();
            
            while *is_running.read().await {
                // Process received messages
                // This is a placeholder - actual implementation would receive from hardware
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            }
        });
    }

    // SDO Helper Methods
    pub async fn sdo_read(&self, node_id: u8, index: u16, subindex: u8) -> Result<Vec<u8>> {
        let (completion_tx, completion_rx) = oneshot::channel();
        
        let transfer = canopen::sdo::SdoTransfer {
            node_id,
            index,
            subindex,
            direction: SdoDirection::Read,
            data: Vec::new(),
            state: canopen::sdo::SdoState::Init,
            expedited: false,
            total_data: 0,
            transferred_data: 0,
            last_toggle: false,
            timeout: std::time::Instant::now(),
            completion_sender: Some(completion_tx),
        };

        self.sdo_transfer_tx.send(transfer).await
            .map_err(|_| CANopenError::Hardware("SDO queue full".to_string()))?;

        completion_rx.await
            .map_err(|_| CANopenError::Hardware("SDO transfer cancelled".to_string()))?
    }

    pub async fn sdo_write(&self, node_id: u8, index: u16, subindex: u8, data: Vec<u8>) -> Result<()> {
        let (completion_tx, completion_rx) = oneshot::channel();
        
        let transfer = canopen::sdo::SdoTransfer {
            node_id,
            index,
            subindex,
            direction: SdoDirection::Write,
            data,
            state: canopen::sdo::SdoState::Init,
            expedited: false,
            total_data: 0,
            transferred_data: 0,
            last_toggle: false,
            timeout: std::time::Instant::now(),
            completion_sender: Some(completion_tx),
        };

        self.sdo_transfer_tx.send(transfer).await
            .map_err(|_| CANopenError::Hardware("SDO queue full".to_string()))?;

        completion_rx.await
            .map_err(|_| CANopenError::Hardware("SDO transfer cancelled".to_string()))?
            .map(|_| ())
    }

    // Convenience methods for different data types
    pub async fn sdo_write_u8(&self, node_id: u8, index: u16, subindex: u8, value: u8) -> Result<()> {
        self.sdo_write(node_id, index, subindex, vec![value]).await
    }

    pub async fn sdo_write_u16(&self, node_id: u8, index: u16, subindex: u8, value: u16) -> Result<()> {
        self.sdo_write(node_id, index, subindex, value.to_le_bytes().to_vec()).await
    }

    pub async fn sdo_write_u32(&self, node_id: u8, index: u16, subindex: u8, value: u32) -> Result<()> {
        self.sdo_write(node_id, index, subindex, value.to_le_bytes().to_vec()).await
    }

    pub async fn sdo_read_u32(&self, node_id: u8, index: u16, subindex: u8) -> Result<u32> {
        let data = self.sdo_read(node_id, index, subindex).await?;
        if data.len() >= 4 {
            Ok(u32::from_le_bytes([data[0], data[1], data[2], data[3]]))
        } else {
            Err(CANopenError::InvalidMessage)
        }
    }

    // NMT Helper Methods
    pub async fn nmt_start(&self, node_id: u8) -> Result<()> {
        self.nmt_manager.read().await.start_node(node_id).await
    }

    pub async fn nmt_stop(&self, node_id: u8) -> Result<()> {
        self.nmt_manager.read().await.stop_node(node_id).await
    }

    pub async fn nmt_preoperational(&self, node_id: u8) -> Result<()> {
        self.nmt_manager.read().await.enter_preoperational(node_id).await
    }

    pub async fn nmt_reset_node(&self, node_id: u8) -> Result<()> {
        self.nmt_manager.read().await.reset_node(node_id).await
    }

    pub async fn nmt_reset_communication(&self, node_id: u8) -> Result<()> {
        self.nmt_manager.read().await.reset_communication(node_id).await
    }

    pub async fn is_node_found(&self, node_id: u8) -> bool {
        self.nmt_manager.read().await.is_node_found(node_id)
    }

    // Event subscription methods
    pub fn subscribe_packets(&self) -> broadcast::Receiver<MessageEvent> {
        self.event_manager.packet_tx.subscribe()
    }

    pub fn subscribe_sdo(&self) -> broadcast::Receiver<MessageEvent> {
        self.event_manager.sdo_tx.subscribe()
    }

    pub fn subscribe_nmt(&self) -> broadcast::Receiver<MessageEvent> {
        self.event_manager.nmt_tx.subscribe()
    }

    pub fn subscribe_pdo(&self) -> broadcast::Receiver<MessageEvent> {
        self.event_manager.pdo_tx.subscribe()
    }

    // PDO Helper
    pub async fn write_pdo(&self, cob_id: u16, payload: Vec<u8>) -> Result<()> {
        let message = CanMessage {
            id: CanId(cob_id),
            data: payload,
            timestamp: None,
        };
        self.send_message(message).await
    }
}
```

### Phase 7: Testing and Examples

#### Step 7.1: Basic Usage Example (`examples/basic_usage.rs`)
```rust
use libcanopen_simple::*;
use peak_can_sys::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();

    // Create PEAK CAN adapter
    let peak_adapter = PeakCanAdapter::new(
        PcanHandle::PcanUsbbus1, 
        BusSpeed::Baud250K
    );

    // Create CANopen instance
    let mut canopen = CANopenSimple::new(Box::new(peak_adapter));

    // Connect to hardware
    canopen.connect().await?;

    // Subscribe to events
    let mut packet_rx = canopen.subscribe_packets();
    
    tokio::spawn(async move {
        while let Ok(event) = packet_rx.recv().await {
            println!("Received: {:?}", event);
        }
    });

    // Send NMT start to all nodes
    canopen.nmt_start(0).await?;

    // Read from a device
    match canopen.sdo_read_u32(1, 0x1000, 0).await {
        Ok(device_type) => println!("Device type: 0x{:08X}", device_type),
        Err(e) => println!("Failed to read device type: {}", e),
    }

    // Write to a device
    canopen.sdo_write_u16(1, 0x1017, 0, 1000).await?; // Set heartbeat to 1000ms

    // Keep running
    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

    // Disconnect
    canopen.disconnect().await?;

    Ok(())
}
```

### Phase 8: Testing Strategy

#### Step 8.1: Unit Tests (`src/canopen/message.rs` tests)
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_type_detection() {
        let nmt_msg = CanMessage {
            id: CanId(0x000),
            data: vec![0x01, 0x00],
            timestamp: None,
        };
        assert_eq!(nmt_msg.message_type(), MessageType::Nmt);

        let sdo_msg = CanMessage {
            id: CanId(0x601),
            data: vec![0x40, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00],
            timestamp: None,
        };
        assert_eq!(sdo_msg.message_type(), MessageType::Sdo);
    }
}
```

#### Step 8.2: Integration Tests (`tests/integration_test.rs`)
```rust
use libcanopen_simple::*;

#[tokio::test]
async fn test_sdo_transfer() {
    // Mock hardware implementation for testing
    // This would require implementing a mock CanHardware trait
}
```

### Phase 9: Documentation and Build

#### Step 9.1: Update Cargo.toml metadata
```toml
[package]
name = "libcanopen-simple"
version = "0.1.0"
edition = "2021"
authors = ["Your Name <your.email@example.com>"]
description = "A simple CANopen library for Rust using PEAK CAN hardware"
license = "GPL-3.0"
repository = "https://github.com/yourusername/libcanopen-simple-rs"
keywords = ["canopen", "can", "industrial", "automation"]
categories = ["embedded", "hardware-support"]

[lib]
name = "libcanopen_simple"
```

#### Step 9.2: Create comprehensive README.md
```markdown
# libCANopen Simple (Rust)

A Rust implementation of a simple CANopen library using PEAK CAN hardware adapters.

## Features

- Async/await based API
- SDO client with expedited and segmented transfers
- NMT state management and commands
- PDO handling
- Event-driven architecture
- PEAK CAN hardware support via peak-can-sys

## Quick Start

Add to your `Cargo.toml`:
```toml
[dependencies]
libcanopen-simple = "0.1"
peak-can-sys = "0.1.2"
tokio = { version = "1.0", features = ["full"] }
```

## Usage

[Include basic usage example here]
```

### Phase 10: Migration Validation

#### Step 10.1: Feature Parity Checklist
- [ ] CAN message sending/receiving ✓
- [ ] SDO client (expedited transfers) ✓
- [ ] SDO client (segmented transfers) ✓
- [ ] NMT commands ✓
- [ ] NMT state tracking ✓
- [ ] PDO handling ✓
- [ ] Event callbacks ✓
- [ ] PEAK CAN hardware support ✓
- [ ] Error handling ✓
- [ ] Timeout management ✓
- [ ] Queue management ✓
- [ ] Bus statistics ✓

#### Step 10.2: Performance Considerations
- Use `tokio` for async operations instead of threading
- Use `Arc` and `RwLock` for shared state
- Use `mpsc` channels for message passing
- Use `broadcast` channels for events
- Consider using `parking_lot` for potentially better lock performance

### Phase 11: Advanced Features (Future)

1. **Block SDO transfers** - Not implemented in C# version, could be added
2. **Object Dictionary support** - For implementing CANopen devices
3. **Additional hardware adapters** - SocketCAN for Linux
4. **Configuration file support** - EDS/DCF file parsing
5. **Network management** - Flying master, network scanning
6. **Diagnostics and logging** - Enhanced error reporting

## Conclusion

This migration plan provides a comprehensive roadmap for converting the C# CANopen library to Rust while maintaining feature parity and improving on the original design with modern Rust patterns. The async/await model provides better performance and resource utilization compared to the thread-based C# implementation.

Key improvements in the Rust version:
- Type safety and memory safety
- Better error handling with `Result` types
- Async/await for better concurrency
- More structured event handling
- Cleaner separation of concerns
- Better testability

The migration should be done incrementally, testing each phase thoroughly before proceeding to the next.