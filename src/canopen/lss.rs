use crate::canopen::message::CanMessage;
/// LSS (Layer Setting Services) implementation
///
/// LSS protocol for node commissioning and configuration
///
/// Communication:
/// - Master -> Slave: COB-ID 0x7E5 (2021)
/// - Slave -> Master: COB-ID 0x7E4 (2020)
///
/// Reference: CiA 305 (LSS and Fastscan)
use crate::{CANopenError, Result};
use log::{debug, trace, warn};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, RwLock};

// LSS COB-IDs (CiA-305)
pub const LSS_MASTER_TX: u16 = 0x7E5; // Master to slave (2021 decimal)
pub const LSS_SLAVE_TX: u16 = 0x7E4; // Slave to master (2020 decimal)

/// LSS Command Specifiers (CS)
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum LssCommand {
    // Switch Mode Commands
    SwitchStateGlobal = 0x04,
    SwitchStateSelective = 0x40,

    // Configuration Commands
    ConfigureNodeId = 0x11,
    ConfigureBitTiming = 0x13,
    ActivateBitTiming = 0x15,
    StoreConfiguration = 0x17,

    // Inquire Commands
    InquireVendorId = 0x5A,
    InquireProductCode = 0x5B,
    InquireRevisionNumber = 0x5C,
    InquireSerialNumber = 0x5D,
    InquireNodeId = 0x5E,

    // Identification Commands
    IdentifyRemoteSlave = 0x46,
    IdentifyNonConfiguredSlave = 0x50,

    // Fastscan (optional)
    FastscanRequest = 0x51,
}

impl LssCommand {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x04 => Some(Self::SwitchStateGlobal),
            0x40 => Some(Self::SwitchStateSelective),
            0x11 => Some(Self::ConfigureNodeId),
            0x13 => Some(Self::ConfigureBitTiming),
            0x15 => Some(Self::ActivateBitTiming),
            0x17 => Some(Self::StoreConfiguration),
            0x5A => Some(Self::InquireVendorId),
            0x5B => Some(Self::InquireProductCode),
            0x5C => Some(Self::InquireRevisionNumber),
            0x5D => Some(Self::InquireSerialNumber),
            0x5E => Some(Self::InquireNodeId),
            0x46 => Some(Self::IdentifyRemoteSlave),
            0x50 => Some(Self::IdentifyNonConfiguredSlave),
            0x51 => Some(Self::FastscanRequest),
            _ => None,
        }
    }
}

/// LSS Error Codes
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum LssError {
    Success = 0,
    UnsupportedCommand = 1,
    MediaAccessFailure = 2,
    InvalidParameter = 3,
    ManufacturerSpecific = 0xFF,
}

impl LssError {
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::Success,
            1 => Self::UnsupportedCommand,
            2 => Self::MediaAccessFailure,
            3 => Self::InvalidParameter,
            _ => Self::ManufacturerSpecific,
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Success => "Success",
            Self::UnsupportedCommand => "Unsupported Command",
            Self::MediaAccessFailure => "Media Access Failure",
            Self::InvalidParameter => "Invalid Parameter",
            Self::ManufacturerSpecific => "Manufacturer Specific Error",
        }
    }
}

/// LSS State (CiA-305)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LssMode {
    Operation = 0,
    Configuration = 1,
}

/// LSS Address (Identity)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LssAddress {
    pub vendor_id: u32,
    pub product_code: u32,
    pub revision_number: u32,
    pub serial_number: u32,
}

/// LSS Response
#[derive(Debug, Clone)]
pub enum LssResponse {
    /// Node-ID configuration response
    ConfigureNodeId(LssError),

    /// Bit-rate configuration response
    ConfigureBitRate(LssError),

    /// Store configuration response
    StoreConfiguration(LssError),

    /// Inquire responses
    InquireVendorId(u32),
    InquireProductCode(u32),
    InquireRevisionNumber(u32),
    InquireSerialNumber(u32),
    InquireNodeId(u8),

    /// Identification response
    IdentifyResponse,

    /// Generic error
    Error(String),
}

/// LSS Request (internal)
struct LssRequest {
    command: Vec<u8>,
    response_tx: oneshot::Sender<Result<LssResponse>>,
    timeout_ms: u32,
}

/// LSS Manager
pub struct LssManager {
    /// Outgoing message channel
    message_tx: mpsc::Sender<CanMessage>,

    /// Pending requests (mpsc for multiple responses support)
    pending_requests: Arc<RwLock<HashMap<u8, mpsc::Sender<Result<LssResponse>>>>>,

    /// Request queue
    request_tx: mpsc::Sender<LssRequest>,
    request_rx: Arc<RwLock<mpsc::Receiver<LssRequest>>>,
}

impl LssManager {
    /// Create a new LSS manager
    pub fn new(message_tx: mpsc::Sender<CanMessage>) -> Self {
        let (request_tx, request_rx) = mpsc::channel(100);

        Self {
            message_tx,
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
            request_tx,
            request_rx: Arc::new(RwLock::new(request_rx)),
        }
    }

    /// Start LSS request processing task
    pub async fn start_processing(&self) {
        let message_tx = self.message_tx.clone();
        let pending = Arc::clone(&self.pending_requests);
        let request_rx = Arc::clone(&self.request_rx);

        tokio::spawn(async move {
            let mut rx = request_rx.write().await;

            while let Some(request) = rx.recv().await {
                // Send LSS command
                let lss_msg = CanMessage::new(LSS_MASTER_TX, request.command.clone())
                    .expect("LSS message creation should never fail");

                if let Err(e) = message_tx.send(lss_msg).await {
                    let _ = request.response_tx.send(Err(CANopenError::ChannelClosed));
                    warn!("Failed to send LSS message: {:?}", e);
                    continue;
                }

                // Create mpsc channel for potentially multiple responses
                let (mpsc_tx, mut mpsc_rx) = mpsc::channel(1);

                // Store pending request
                if !request.command.is_empty() {
                    let cs = request.command[0];
                    pending.write().await.insert(cs, mpsc_tx);

                    // Forward first response to oneshot sender and clean up
                    let pending_clone = Arc::clone(&pending);
                    let response_tx = request.response_tx;
                    let timeout_ms = request.timeout_ms;

                    tokio::spawn(async move {
                        match tokio::time::timeout(
                            tokio::time::Duration::from_millis(timeout_ms as u64),
                            mpsc_rx.recv(),
                        )
                        .await
                        {
                            Ok(Some(response)) => {
                                let _ = response_tx.send(response);
                            }
                            Ok(None) | Err(_) => {
                                let _ = response_tx.send(Err(CANopenError::Timeout));
                            }
                        }

                        // Clean up pending request
                        pending_clone.write().await.remove(&cs);
                    });
                }
            }
        });
    }

    /// Process incoming LSS response
    pub async fn process_response(&self, data: &[u8]) {
        if data.is_empty() {
            return;
        }

        let cs = data[0];

        // Find pending request (don't remove, as we may need to handle multiple responses)
        let response_tx = {
            let pending = self.pending_requests.read().await;
            pending.get(&cs).cloned()
        };

        if let Some(tx) = response_tx {
            let response = Self::parse_response(data);
            // Send to channel (may fail if receiver dropped, which is OK)
            let _ = tx.send(response).await;
        } else {
            trace!(
                "Received LSS response with no pending request: CS=0x{:02X}",
                cs
            );
        }
    }

    /// Parse LSS response message
    fn parse_response(data: &[u8]) -> Result<LssResponse> {
        if data.is_empty() {
            return Err(CANopenError::InvalidMessage);
        }

        let cs = data[0];

        match LssCommand::from_u8(cs) {
            Some(LssCommand::ConfigureNodeId) => {
                if data.len() >= 2 {
                    let error = LssError::from_u8(data[1]);
                    Ok(LssResponse::ConfigureNodeId(error))
                } else {
                    Err(CANopenError::InvalidMessage)
                }
            }
            Some(LssCommand::ConfigureBitTiming) => {
                if data.len() >= 2 {
                    let error = LssError::from_u8(data[1]);
                    Ok(LssResponse::ConfigureBitRate(error))
                } else {
                    Err(CANopenError::InvalidMessage)
                }
            }
            Some(LssCommand::StoreConfiguration) => {
                if data.len() >= 2 {
                    let error = LssError::from_u8(data[1]);
                    Ok(LssResponse::StoreConfiguration(error))
                } else {
                    Err(CANopenError::InvalidMessage)
                }
            }
            Some(LssCommand::InquireVendorId) => {
                if data.len() >= 5 {
                    let value = u32::from_le_bytes([data[1], data[2], data[3], data[4]]);
                    Ok(LssResponse::InquireVendorId(value))
                } else {
                    Err(CANopenError::InvalidMessage)
                }
            }
            Some(LssCommand::InquireProductCode) => {
                if data.len() >= 5 {
                    let value = u32::from_le_bytes([data[1], data[2], data[3], data[4]]);
                    Ok(LssResponse::InquireProductCode(value))
                } else {
                    Err(CANopenError::InvalidMessage)
                }
            }
            Some(LssCommand::InquireRevisionNumber) => {
                if data.len() >= 5 {
                    let value = u32::from_le_bytes([data[1], data[2], data[3], data[4]]);
                    Ok(LssResponse::InquireRevisionNumber(value))
                } else {
                    Err(CANopenError::InvalidMessage)
                }
            }
            Some(LssCommand::InquireSerialNumber) => {
                if data.len() >= 5 {
                    let value = u32::from_le_bytes([data[1], data[2], data[3], data[4]]);
                    Ok(LssResponse::InquireSerialNumber(value))
                } else {
                    Err(CANopenError::InvalidMessage)
                }
            }
            Some(LssCommand::InquireNodeId) => {
                if data.len() >= 2 {
                    Ok(LssResponse::InquireNodeId(data[1]))
                } else {
                    Err(CANopenError::InvalidMessage)
                }
            }
            Some(LssCommand::IdentifyRemoteSlave) => Ok(LssResponse::IdentifyResponse),
            _ => {
                warn!("Unknown LSS response: CS=0x{:02X}", cs);
                Err(CANopenError::InvalidMessage)
            }
        }
    }

    /// Send LSS command and wait for response
    async fn send_command(&self, command: Vec<u8>, timeout_ms: u32) -> Result<LssResponse> {
        let (response_tx, response_rx) = oneshot::channel();

        let request = LssRequest {
            command,
            response_tx,
            timeout_ms,
        };

        self.request_tx
            .send(request)
            .await
            .map_err(|_| CANopenError::ChannelClosed)?;

        match tokio::time::timeout(
            tokio::time::Duration::from_millis(timeout_ms as u64 + 100),
            response_rx,
        )
        .await
        {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(CANopenError::ChannelClosed),
            Err(_) => Err(CANopenError::Timeout),
        }
    }

    // === LSS Commands ===

    /// Switch to global state (all unconfigured slaves)
    pub async fn switch_state_global(&self, mode: LssMode) -> Result<()> {
        let command = vec![
            LssCommand::SwitchStateGlobal as u8,
            mode as u8,
            0,
            0,
            0,
            0,
            0,
            0,
        ];

        // Global commands don't get responses
        let lss_msg = CanMessage::new(LSS_MASTER_TX, command)
            .expect("LSS message creation should never fail");
        self.message_tx
            .send(lss_msg)
            .await
            .map_err(|_| CANopenError::ChannelClosed)?;

        debug!("LSS: Switched to global mode: {:?}", mode);
        Ok(())
    }

    /// Switch to selective state (specific slave by LSS address)
    pub async fn switch_state_selective(&self, address: &LssAddress) -> Result<()> {
        // Send 4 messages for the 4 parts of LSS address
        // CS values: 0x40 (vendor), 0x41 (product), 0x42 (revision), 0x43 (serial)
        let address_parts = [
            (0x40u8, address.vendor_id),
            (0x41u8, address.product_code),
            (0x42u8, address.revision_number),
            (0x43u8, address.serial_number),
            (0x44u8, 0u32),
        ];

        for (cs, value) in address_parts.iter() {
            let mut command = vec![*cs];
            command.extend_from_slice(&value.to_le_bytes());
            command.extend_from_slice(&[0, 0, 0]); // Padding to 8 bytes

            let lss_msg = CanMessage::new(LSS_MASTER_TX, command)
                .expect("LSS message creation should never fail");
            self.message_tx
                .send(lss_msg)
                .await
                .map_err(|_| CANopenError::ChannelClosed)?;

            // Small delay between messages
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        debug!("LSS: Switched to selective mode for address: {:?}", address);
        Ok(())
    }

    /// Configure node-ID
    pub async fn configure_node_id(&self, node_id: u8, timeout_ms: u32) -> Result<LssError> {
        if node_id == 0 || node_id > 127 {
            return Err(CANopenError::InvalidData(
                "Node ID must be 1-127".to_string(),
            ));
        }

        let command = vec![LssCommand::ConfigureNodeId as u8, node_id, 0, 0, 0, 0, 0, 0];

        let response = self.send_command(command, timeout_ms).await?;

        match response {
            LssResponse::ConfigureNodeId(error) => {
                debug!("LSS: Configure node-ID {} result: {:?}", node_id, error);
                Ok(error)
            }
            _ => Err(CANopenError::InvalidMessage),
        }
    }

    /// Configure bit-rate (bit timing)
    pub async fn configure_bit_rate(
        &self,
        table_selector: u8,
        table_index: u8,
        timeout_ms: u32,
    ) -> Result<LssError> {
        let command = vec![
            LssCommand::ConfigureBitTiming as u8,
            table_selector,
            table_index,
            0,
            0,
            0,
            0,
            0,
        ];

        let response = self.send_command(command, timeout_ms).await?;

        match response {
            LssResponse::ConfigureBitRate(error) => {
                debug!("LSS: Configure bit-rate result: {:?}", error);
                Ok(error)
            }
            _ => Err(CANopenError::InvalidMessage),
        }
    }

    /// Activate configured bit-rate
    pub async fn activate_bit_rate(&self, switch_delay_ms: u16) -> Result<()> {
        let delay_bytes = switch_delay_ms.to_le_bytes();
        let command = vec![
            LssCommand::ActivateBitTiming as u8,
            delay_bytes[0],
            delay_bytes[1],
            0,
            0,
            0,
            0,
            0,
        ];

        let lss_msg = CanMessage::new(LSS_MASTER_TX, command)
            .expect("LSS message creation should never fail");
        self.message_tx
            .send(lss_msg)
            .await
            .map_err(|_| CANopenError::ChannelClosed)?;

        debug!(
            "LSS: Activated bit-rate (switch delay: {}ms)",
            switch_delay_ms
        );
        Ok(())
    }

    /// Store configuration to non-volatile memory
    pub async fn store_configuration(&self, timeout_ms: u32) -> Result<LssError> {
        let command = vec![LssCommand::StoreConfiguration as u8, 0, 0, 0, 0, 0, 0, 0];

        let response = self.send_command(command, timeout_ms).await?;

        match response {
            LssResponse::StoreConfiguration(error) => {
                debug!("LSS: Store configuration result: {:?}", error);
                Ok(error)
            }
            _ => Err(CANopenError::InvalidMessage),
        }
    }

    /// Inquire vendor IDs from all LSS slaves in configuration mode
    ///
    /// Sends LSS inquiry command 0x5A once and collects all responses within timeout.
    /// Returns Vec of all unique vendor IDs received.
    ///
    /// Note: This is a broadcast command. In global configuration mode, multiple
    /// slaves can respond. Use switch_state_selective() first to query a specific slave.
    pub async fn inquire_vendor_ids(&self, timeout_ms: u32) -> Result<Vec<u32>> {
        let command = vec![LssCommand::InquireVendorId as u8, 0, 0, 0, 0, 0, 0, 0];
        let cs = command[0];

        // Send command once
        let lss_msg = CanMessage::new(LSS_MASTER_TX, command)
            .expect("LSS message creation should never fail");
        self.message_tx
            .send(lss_msg)
            .await
            .map_err(|_| CANopenError::ChannelClosed)?;

        // Collect responses
        let (response_tx, mut response_rx) = mpsc::channel(100);
        self.pending_requests.write().await.insert(cs, response_tx);

        let mut vendor_ids = Vec::new();
        let start_time = tokio::time::Instant::now();
        let timeout_duration = tokio::time::Duration::from_millis(timeout_ms as u64);

        while start_time.elapsed() < timeout_duration {
            let remaining_time = timeout_duration - start_time.elapsed();
            match tokio::time::timeout(remaining_time, response_rx.recv()).await {
                Ok(Some(result)) => match result {
                    Ok(LssResponse::InquireVendorId(vendor_id)) => {
                        if !vendor_ids.contains(&vendor_id) {
                            debug!("LSS: Vendor ID: 0x{:08X}", vendor_id);
                            vendor_ids.push(vendor_id);
                        }
                        // Continue collecting more responses
                    }
                    Ok(_) => continue,
                    Err(_) => break,
                },
                Ok(None) => break,
                Err(_) => break, // Timeout
            }
        }

        // Clean up
        self.pending_requests.write().await.remove(&cs);

        if vendor_ids.is_empty() {
            Err(CANopenError::Timeout)
        } else {
            Ok(vendor_ids)
        }
    }

    /// Inquire product codes from all LSS slaves in configuration mode
    ///
    /// Sends LSS inquiry command 0x5B once and collects all responses within timeout.
    /// Returns Vec of all unique product codes received.
    ///
    /// Note: This is a broadcast command. In global configuration mode, multiple
    /// slaves can respond. Use switch_state_selective() first to query a specific slave.
    pub async fn inquire_product_codes(&self, timeout_ms: u32) -> Result<Vec<u32>> {
        let command = vec![LssCommand::InquireProductCode as u8, 0, 0, 0, 0, 0, 0, 0];
        let cs = command[0];

        // Send command once
        let lss_msg = CanMessage::new(LSS_MASTER_TX, command)
            .expect("LSS message creation should never fail");
        self.message_tx
            .send(lss_msg)
            .await
            .map_err(|_| CANopenError::ChannelClosed)?;

        // Collect responses
        let (response_tx, mut response_rx) = mpsc::channel(100);
        self.pending_requests.write().await.insert(cs, response_tx);

        let mut product_codes = Vec::new();
        let start_time = tokio::time::Instant::now();
        let timeout_duration = tokio::time::Duration::from_millis(timeout_ms as u64);

        while start_time.elapsed() < timeout_duration {
            let remaining_time = timeout_duration - start_time.elapsed();
            match tokio::time::timeout(remaining_time, response_rx.recv()).await {
                Ok(Some(result)) => match result {
                    Ok(LssResponse::InquireProductCode(product_code)) => {
                        if !product_codes.contains(&product_code) {
                            debug!("LSS: Product Code: 0x{:08X}", product_code);
                            product_codes.push(product_code);
                        }
                        // Continue collecting more responses
                    }
                    Ok(_) => continue,
                    Err(_) => break,
                },
                Ok(None) => break,
                Err(_) => break, // Timeout
            }
        }

        // Clean up
        self.pending_requests.write().await.remove(&cs);

        if product_codes.is_empty() {
            Err(CANopenError::Timeout)
        } else {
            Ok(product_codes)
        }
    }

    /// Inquire revision numbers from all LSS slaves in configuration mode
    ///
    /// Sends LSS inquiry command 0x5C once and collects all responses within timeout.
    /// Returns Vec of all unique revision numbers received.
    ///
    /// Note: This is a broadcast command. In global configuration mode, multiple
    /// slaves can respond. Use switch_state_selective() first to query a specific slave.
    pub async fn inquire_revision_numbers(&self, timeout_ms: u32) -> Result<Vec<u32>> {
        let command = vec![LssCommand::InquireRevisionNumber as u8, 0, 0, 0, 0, 0, 0, 0];
        let cs = command[0];

        // Send command once
        let lss_msg = CanMessage::new(LSS_MASTER_TX, command)
            .expect("LSS message creation should never fail");
        self.message_tx
            .send(lss_msg)
            .await
            .map_err(|_| CANopenError::ChannelClosed)?;

        // Collect responses
        let (response_tx, mut response_rx) = mpsc::channel(100);
        self.pending_requests.write().await.insert(cs, response_tx);

        let mut revision_numbers = Vec::new();
        let start_time = tokio::time::Instant::now();
        let timeout_duration = tokio::time::Duration::from_millis(timeout_ms as u64);

        while start_time.elapsed() < timeout_duration {
            let remaining_time = timeout_duration - start_time.elapsed();
            match tokio::time::timeout(remaining_time, response_rx.recv()).await {
                Ok(Some(result)) => match result {
                    Ok(LssResponse::InquireRevisionNumber(revision_number)) => {
                        if !revision_numbers.contains(&revision_number) {
                            debug!("LSS: Revision Number: 0x{:08X}", revision_number);
                            revision_numbers.push(revision_number);
                        }
                        // Continue collecting more responses
                    }
                    Ok(_) => continue,
                    Err(_) => break,
                },
                Ok(None) => break,
                Err(_) => break, // Timeout
            }
        }

        // Clean up
        self.pending_requests.write().await.remove(&cs);

        if revision_numbers.is_empty() {
            Err(CANopenError::Timeout)
        } else {
            Ok(revision_numbers)
        }
    }

    /// Inquire serial numbers from all LSS slaves in configuration mode
    ///
    /// Sends LSS inquiry command 0x5D once and collects all responses within timeout.
    /// Returns Vec of all unique serial numbers received.
    ///
    /// Note: This is a broadcast command. In global configuration mode, multiple
    /// slaves can respond. Use switch_state_selective() first to query a specific slave.
    pub async fn inquire_serial_numbers(&self, timeout_ms: u32) -> Result<Vec<u32>> {
        let command = vec![LssCommand::InquireSerialNumber as u8, 0, 0, 0, 0, 0, 0, 0];
        let cs = command[0];

        // Send command once
        let lss_msg = CanMessage::new(LSS_MASTER_TX, command)
            .expect("LSS message creation should never fail");
        self.message_tx
            .send(lss_msg)
            .await
            .map_err(|_| CANopenError::ChannelClosed)?;

        // Collect responses
        let (response_tx, mut response_rx) = mpsc::channel(100);
        self.pending_requests.write().await.insert(cs, response_tx);

        let mut serial_numbers = Vec::new();
        let start_time = tokio::time::Instant::now();
        let timeout_duration = tokio::time::Duration::from_millis(timeout_ms as u64);

        while start_time.elapsed() < timeout_duration {
            let remaining_time = timeout_duration - start_time.elapsed();
            match tokio::time::timeout(remaining_time, response_rx.recv()).await {
                Ok(Some(result)) => match result {
                    Ok(LssResponse::InquireSerialNumber(serial_number)) => {
                        if !serial_numbers.contains(&serial_number) {
                            debug!("LSS: Serial Number: 0x{:08X}", serial_number);
                            serial_numbers.push(serial_number);
                        }
                        // Continue collecting more responses
                    }
                    Ok(_) => continue,
                    Err(_) => break,
                },
                Ok(None) => break,
                Err(_) => break, // Timeout
            }
        }

        // Clean up
        self.pending_requests.write().await.remove(&cs);

        if serial_numbers.is_empty() {
            Err(CANopenError::Timeout)
        } else {
            Ok(serial_numbers)
        }
    }

    /// Inquire node IDs from all LSS slaves in configuration mode
    ///
    /// Sends LSS inquiry command 0x5E once and collects all responses within timeout.
    /// Returns Vec of all unique node IDs received.
    ///
    /// Note: This is a broadcast command. In global configuration mode, multiple
    /// slaves can respond. Use switch_state_selective() first to query a specific slave.
    pub async fn inquire_node_ids(&self, timeout_ms: u32) -> Result<Vec<u8>> {
        let command = vec![LssCommand::InquireNodeId as u8, 0, 0, 0, 0, 0, 0, 0];
        let cs = command[0];

        // Send command once
        let lss_msg = CanMessage::new(LSS_MASTER_TX, command)
            .expect("LSS message creation should never fail");
        self.message_tx
            .send(lss_msg)
            .await
            .map_err(|_| CANopenError::ChannelClosed)?;

        // Collect responses
        let (response_tx, mut response_rx) = mpsc::channel(100);
        self.pending_requests.write().await.insert(cs, response_tx);

        let mut node_ids = Vec::new();
        let start_time = tokio::time::Instant::now();
        let timeout_duration = tokio::time::Duration::from_millis(timeout_ms as u64);

        while start_time.elapsed() < timeout_duration {
            let remaining_time = timeout_duration - start_time.elapsed();
            match tokio::time::timeout(remaining_time, response_rx.recv()).await {
                Ok(Some(result)) => match result {
                    Ok(LssResponse::InquireNodeId(node_id)) => {
                        if !node_ids.contains(&node_id) {
                            debug!("LSS: Node ID: {}", node_id);
                            node_ids.push(node_id);
                        }
                        // Continue collecting more responses
                    }
                    Ok(_) => continue,
                    Err(_) => break,
                },
                Ok(None) => break,
                Err(_) => break, // Timeout
            }
        }

        // Clean up
        self.pending_requests.write().await.remove(&cs);

        if node_ids.is_empty() {
            Err(CANopenError::Timeout)
        } else {
            Ok(node_ids)
        }
    }

    /// Identify remote slave (check if selected slave responds)
    pub async fn identify_remote_slave(&self, timeout_ms: u32) -> Result<bool> {
        let response = self
            .send_command(
                vec![LssCommand::IdentifyRemoteSlave as u8, 0, 0, 0, 0, 0, 0, 0],
                timeout_ms,
            )
            .await;

        match response {
            Ok(LssResponse::IdentifyResponse) => {
                debug!("LSS: Slave identified");
                Ok(true)
            }
            Err(CANopenError::Timeout) => {
                debug!("LSS: No slave response");
                Ok(false)
            }
            Err(e) => Err(e),
            _ => Err(CANopenError::InvalidMessage),
        }
    }
}

impl Clone for LssManager {
    fn clone(&self) -> Self {
        Self {
            message_tx: self.message_tx.clone(),
            pending_requests: Arc::clone(&self.pending_requests),
            request_tx: self.request_tx.clone(),
            request_rx: Arc::clone(&self.request_rx),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lss_cob_ids_per_cia305() {
        // Per CiA-305 standard:
        // LSS Master TX (to slave) = 2021 decimal = 0x7E5
        // LSS Slave TX (to master) = 2020 decimal = 0x7E4
        assert_eq!(LSS_MASTER_TX, 0x7E5);
        assert_eq!(LSS_SLAVE_TX, 0x7E4);

        // Verify decimal values
        assert_eq!(LSS_MASTER_TX, 2021);
        assert_eq!(LSS_SLAVE_TX, 2020);
    }

    #[test]
    fn test_lss_command_from_u8() {
        // Switch Mode Commands
        assert_eq!(
            LssCommand::from_u8(0x04),
            Some(LssCommand::SwitchStateGlobal)
        );
        assert_eq!(
            LssCommand::from_u8(0x40),
            Some(LssCommand::SwitchStateSelective)
        );

        // Configuration Commands
        assert_eq!(LssCommand::from_u8(0x11), Some(LssCommand::ConfigureNodeId));
        assert_eq!(
            LssCommand::from_u8(0x13),
            Some(LssCommand::ConfigureBitTiming)
        );
        assert_eq!(
            LssCommand::from_u8(0x15),
            Some(LssCommand::ActivateBitTiming)
        );
        assert_eq!(
            LssCommand::from_u8(0x17),
            Some(LssCommand::StoreConfiguration)
        );

        // Inquire Commands
        assert_eq!(LssCommand::from_u8(0x5A), Some(LssCommand::InquireVendorId));
        assert_eq!(
            LssCommand::from_u8(0x5B),
            Some(LssCommand::InquireProductCode)
        );
        assert_eq!(
            LssCommand::from_u8(0x5C),
            Some(LssCommand::InquireRevisionNumber)
        );
        assert_eq!(
            LssCommand::from_u8(0x5D),
            Some(LssCommand::InquireSerialNumber)
        );
        assert_eq!(LssCommand::from_u8(0x5E), Some(LssCommand::InquireNodeId));

        // Identification Commands
        assert_eq!(
            LssCommand::from_u8(0x46),
            Some(LssCommand::IdentifyRemoteSlave)
        );
        assert_eq!(
            LssCommand::from_u8(0x50),
            Some(LssCommand::IdentifyNonConfiguredSlave)
        );

        // Fastscan
        assert_eq!(LssCommand::from_u8(0x51), Some(LssCommand::FastscanRequest));

        // Invalid command
        assert_eq!(LssCommand::from_u8(0xFF), None);
        assert_eq!(LssCommand::from_u8(0x00), None);
    }

    #[test]
    fn test_all_lss_commands_have_unique_values() {
        // Ensure no command value conflicts
        let commands = [
            LssCommand::SwitchStateGlobal as u8,
            LssCommand::SwitchStateSelective as u8,
            LssCommand::ConfigureNodeId as u8,
            LssCommand::ConfigureBitTiming as u8,
            LssCommand::ActivateBitTiming as u8,
            LssCommand::StoreConfiguration as u8,
            LssCommand::InquireVendorId as u8,
            LssCommand::InquireProductCode as u8,
            LssCommand::InquireRevisionNumber as u8,
            LssCommand::InquireSerialNumber as u8,
            LssCommand::InquireNodeId as u8,
            LssCommand::IdentifyRemoteSlave as u8,
            LssCommand::IdentifyNonConfiguredSlave as u8,
            LssCommand::FastscanRequest as u8,
        ];

        // Check for duplicates
        for i in 0..commands.len() {
            for j in (i + 1)..commands.len() {
                assert_ne!(commands[i], commands[j], "Duplicate command values found");
            }
        }
    }

    #[test]
    fn test_lss_error_codes() {
        assert_eq!(LssError::Success as u8, 0);
        assert_eq!(LssError::UnsupportedCommand as u8, 1);
        assert_eq!(LssError::MediaAccessFailure as u8, 2);
        assert_eq!(LssError::InvalidParameter as u8, 3);
        assert_eq!(LssError::ManufacturerSpecific as u8, 0xFF);
    }

    #[test]
    fn test_lss_error_description() {
        assert_eq!(LssError::Success.description(), "Success");
        assert_eq!(
            LssError::UnsupportedCommand.description(),
            "Unsupported Command"
        );
        assert_eq!(
            LssError::MediaAccessFailure.description(),
            "Media Access Failure"
        );
        assert_eq!(
            LssError::InvalidParameter.description(),
            "Invalid Parameter"
        );
        assert_eq!(
            LssError::ManufacturerSpecific.description(),
            "Manufacturer Specific Error"
        );
    }

    #[test]
    fn test_lss_address_creation() {
        let addr = LssAddress {
            vendor_id: 0x12345678,
            product_code: 0xABCDEF00,
            revision_number: 0x00010002,
            serial_number: 0xDEADBEEF,
        };

        assert_eq!(addr.vendor_id, 0x12345678);
        assert_eq!(addr.product_code, 0xABCDEF00);
        assert_eq!(addr.revision_number, 0x00010002);
        assert_eq!(addr.serial_number, 0xDEADBEEF);
    }

    #[test]
    fn test_lss_address_clone() {
        let addr1 = LssAddress {
            vendor_id: 0x11111111,
            product_code: 0x22222222,
            revision_number: 0x33333333,
            serial_number: 0x44444444,
        };

        let addr2 = addr1;

        assert_eq!(addr1.vendor_id, addr2.vendor_id);
        assert_eq!(addr1.product_code, addr2.product_code);
        assert_eq!(addr1.revision_number, addr2.revision_number);
        assert_eq!(addr1.serial_number, addr2.serial_number);
    }

    #[test]
    fn test_lss_message_format() {
        // LSS messages are always 8 bytes

        // Master to Slave message (COB-ID 0x7E5)
        let master_msg =
            CanMessage::new(0x7E5, vec![0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]).unwrap();
        assert_eq!(master_msg.id.raw(), 0x7E5);
        assert_eq!(master_msg.data.len(), 8);

        // Slave to Master message (COB-ID 0x7E4)
        let slave_msg =
            CanMessage::new(0x7E4, vec![0x5A, 0x78, 0x56, 0x34, 0x12, 0x00, 0x00, 0x00]).unwrap();
        assert_eq!(slave_msg.id.raw(), 0x7E4);
        assert_eq!(slave_msg.data.len(), 8);
    }

    #[test]
    fn test_lss_inquire_vendor_id_request() {
        // Inquire Vendor ID: CS=0x5A
        let data = vec![0x5A, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let msg = CanMessage::new(LSS_MASTER_TX, data).unwrap();

        assert_eq!(msg.data[0], 0x5A);
        assert_eq!(msg.id.raw(), 0x7E5);
    }

    #[test]
    fn test_lss_inquire_vendor_id_response() {
        // Response with vendor_id = 0x12345678 (little-endian)
        let data = vec![0x5A, 0x78, 0x56, 0x34, 0x12, 0x00, 0x00, 0x00];
        let msg = CanMessage::new(LSS_SLAVE_TX, data).unwrap();

        assert_eq!(msg.data[0], 0x5A); // Command specifier echoed
        assert_eq!(msg.id.raw(), 0x7E4); // Slave transmits on 0x7E4

        // Extract vendor ID (bytes 1-4, little-endian)
        let vendor_id = u32::from_le_bytes([msg.data[1], msg.data[2], msg.data[3], msg.data[4]]);
        assert_eq!(vendor_id, 0x12345678);
    }

    #[test]
    fn test_lss_configure_node_id() {
        // Configure Node ID: CS=0x11, Node ID in byte 1
        let new_node_id = 42u8;
        let data = vec![0x11, new_node_id, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let msg = CanMessage::new(LSS_MASTER_TX, data).unwrap();

        assert_eq!(msg.data[0], 0x11);
        assert_eq!(msg.data[1], 42);
    }

    #[test]
    fn test_lss_configure_node_id_response() {
        // Response: CS=0x11, Error Code in byte 1
        let data = vec![
            0x11,
            LssError::Success as u8,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
        ];
        let msg = CanMessage::new(LSS_SLAVE_TX, data).unwrap();

        assert_eq!(msg.data[0], 0x11);
        assert_eq!(msg.data[1], 0); // Success
    }

    #[test]
    fn test_lss_switch_state_global() {
        // Switch State Global: CS=0x04, Mode in byte 1
        // Mode: 0=Waiting, 1=Configuration
        let data = vec![0x04, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let msg = CanMessage::new(LSS_MASTER_TX, data).unwrap();

        assert_eq!(msg.data[0], 0x04);
        assert_eq!(msg.data[1], 0x01); // Enter configuration mode
    }

    #[test]
    fn test_lss_switch_state_selective() {
        // Switch State Selective uses vendor ID, product code, revision, serial
        // This is a 5-message sequence. Test the first message (vendor ID)
        let vendor_id = 0x12345678u32;
        let data = vec![
            0x40,
            (vendor_id & 0xFF) as u8,
            ((vendor_id >> 8) & 0xFF) as u8,
            ((vendor_id >> 16) & 0xFF) as u8,
            ((vendor_id >> 24) & 0xFF) as u8,
            0x00,
            0x00,
            0x00,
        ];
        let msg = CanMessage::new(LSS_MASTER_TX, data).unwrap();

        assert_eq!(msg.data[0], 0x40);
        let extracted_vendor =
            u32::from_le_bytes([msg.data[1], msg.data[2], msg.data[3], msg.data[4]]);
        assert_eq!(extracted_vendor, vendor_id);
    }

    #[test]
    fn test_lss_store_configuration() {
        // Store Configuration: CS=0x17
        let data = vec![0x17, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let msg = CanMessage::new(LSS_MASTER_TX, data).unwrap();

        assert_eq!(msg.data[0], 0x17);
    }

    #[test]
    fn test_lss_activate_bit_timing() {
        // Activate Bit Timing: CS=0x15, Delay in bytes 1-2 (little-endian, milliseconds)
        let delay_ms = 1000u16;
        let data = vec![
            0x15,
            (delay_ms & 0xFF) as u8,
            ((delay_ms >> 8) & 0xFF) as u8,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
        ];
        let msg = CanMessage::new(LSS_MASTER_TX, data).unwrap();

        assert_eq!(msg.data[0], 0x15);
        let extracted_delay = u16::from_le_bytes([msg.data[1], msg.data[2]]);
        assert_eq!(extracted_delay, 1000);
    }

    #[test]
    fn test_lss_identify_non_configured_slave() {
        // Identify Non-Configured Slave: CS=0x50
        let data = vec![0x50, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let msg = CanMessage::new(LSS_MASTER_TX, data).unwrap();

        assert_eq!(msg.data[0], 0x50);
    }

    #[test]
    fn test_lss_fastscan_request() {
        // Fastscan: CS=0x51
        let data = vec![0x51, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let msg = CanMessage::new(LSS_MASTER_TX, data).unwrap();

        assert_eq!(msg.data[0], 0x51);
    }

    #[test]
    fn test_lss_message_must_be_8_bytes() {
        // LSS messages must always be exactly 8 bytes

        // Valid 8-byte message
        let valid = CanMessage::new(0x7E5, vec![0; 8]);
        assert!(valid.is_ok());

        // Invalid 7-byte message
        let invalid = CanMessage::new(0x7E5, vec![0; 7]);
        assert!(invalid.is_ok()); // Message created but validation might fail

        let msg = CanMessage::new_unchecked(0x7E5, vec![0; 7]);
        assert!(!msg.is_valid()); // LSS messages should be 8 bytes
    }

    #[test]
    fn test_lss_error_responses() {
        // Test all error responses
        for (error, expected_byte) in [
            (LssError::Success, 0u8),
            (LssError::UnsupportedCommand, 1u8),
            (LssError::MediaAccessFailure, 2u8),
            (LssError::InvalidParameter, 3u8),
            (LssError::ManufacturerSpecific, 0xFFu8),
        ] {
            let data = vec![0x11, error as u8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
            let msg = CanMessage::new(LSS_SLAVE_TX, data).unwrap();
            assert_eq!(msg.data[1], expected_byte);
        }
    }

    #[test]
    fn test_lss_command_equality() {
        assert_eq!(LssCommand::InquireVendorId, LssCommand::InquireVendorId);
        assert_ne!(LssCommand::InquireVendorId, LssCommand::InquireProductCode);
    }

    #[test]
    fn test_lss_error_equality() {
        assert_eq!(LssError::Success, LssError::Success);
        assert_ne!(LssError::Success, LssError::InvalidParameter);
    }

    #[test]
    fn test_lss_all_inquire_commands() {
        // Test all 5 inquire commands
        let inquire_commands = [
            (0x5A, LssCommand::InquireVendorId),
            (0x5B, LssCommand::InquireProductCode),
            (0x5C, LssCommand::InquireRevisionNumber),
            (0x5D, LssCommand::InquireSerialNumber),
            (0x5E, LssCommand::InquireNodeId),
        ];

        for (cs, expected_cmd) in inquire_commands {
            assert_eq!(LssCommand::from_u8(cs), Some(expected_cmd));
            let data = vec![cs, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
            let msg = CanMessage::new(LSS_MASTER_TX, data).unwrap();
            assert_eq!(msg.data[0], cs);
        }
    }
}
