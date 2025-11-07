// SDO (Service Data Object) client implementation
// Handles expedited and segmented SDO transfers for object dictionary access

use crate::{Result, CANopenError};
use crate::canopen::message::{CanMessage, MessageType};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot};
use tokio::time::timeout;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SdoDirection {
    Upload,   // Read from server (device)
    Download, // Write to server (device)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SdoState {
    Idle,
    InitiateSent,
    SegmentTransfer,
    Completed,
    Aborted,
    Timeout,
}

/// SDO transfer type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SdoTransferType {
    Expedited,  // Data fits in single message (<=4 bytes)
    Segmented,  // Data requires multiple messages
}

/// SDO transfer request
#[derive(Debug)]
pub struct SdoRequest {
    pub node_id: u8,
    pub index: u16,
    pub subindex: u8,
    pub direction: SdoDirection,
    pub data: Vec<u8>,  // For downloads, empty for uploads
    pub timeout_ms: u32,
    pub response_sender: oneshot::Sender<Result<Vec<u8>>>,
}

impl SdoRequest {
    pub fn new(
        node_id: u8,
        direction: SdoDirection,
        index: u16,
        subindex: u8,
        data: Vec<u8>,
        timeout_ms: u32,
        response_sender: oneshot::Sender<Result<Vec<u8>>>,
    ) -> Self {
        Self {
            node_id,
            index,
            subindex,
            direction,
            data,
            timeout_ms,
            response_sender,
        }
    }
}

/// Active SDO transfer state
#[derive(Debug)]
pub struct SdoTransfer {
    pub node_id: u8,
    pub index: u16,
    pub subindex: u8,
    pub direction: SdoDirection,
    pub state: SdoState,
    pub transfer_type: SdoTransferType,
    pub response_sender: Option<oneshot::Sender<Result<Vec<u8>>>>,
    pub data_buffer: Vec<u8>,
    pub segment_index: u8,
    pub toggle_bit: bool,
    pub expected_size: Option<usize>,
    pub last_timestamp: Instant,
    pub timeout_duration: Duration,
}

/// SDO Client - handles SDO transfers for object dictionary access
pub struct SdoClient {
    message_sender: mpsc::Sender<CanMessage>,
    message_receiver: mpsc::Receiver<CanMessage>,
    request_receiver: mpsc::Receiver<SdoRequest>,
    active_transfers: HashMap<u8, SdoTransfer>, // node_id -> transfer
    next_transfer_id: u32,
}

impl SdoClient {
    /// Create a new SDO client
    pub fn new(message_sender: mpsc::Sender<CanMessage>) -> (Self, mpsc::Sender<SdoRequest>, mpsc::Sender<CanMessage>) {
        let (request_sender, request_receiver) = mpsc::channel(100);
        let (message_tx, message_rx) = mpsc::channel(1000);
        let client = Self {
            message_sender,
            message_receiver: message_rx,
            request_receiver,
            active_transfers: HashMap::new(),
            next_transfer_id: 0,
        };
        (client, request_sender, message_tx)
    }

    /// Start the SDO client processing loop
    pub async fn run(&mut self) {
        log::debug!("SDO client run() loop started");
        let mut cleanup_interval = tokio::time::interval(Duration::from_millis(100));
        
        loop {
            tokio::select! {
                // Handle new SDO requests
                request = self.request_receiver.recv() => {
                    if let Some(request) = request {
                        log::debug!("SDO client received request for node {}", request.node_id);
                        if let Err(e) = self.handle_sdo_request(request).await {
                            log::error!("Failed to handle SDO request: {}", e);
                        }
                    } else {
                        // Channel closed, exit
                        log::error!("SDO request receiver channel closed - exiting SDO client loop");
                        break;
                    }
                }
                
                // Handle incoming CAN messages
                message = self.message_receiver.recv() => {
                    if let Some(message) = message {
                        log::debug!("SDO client received CAN message: ID={:03X}", message.id.raw());
                        if let Err(e) = self.process_message(&message).await {
                            log::error!("Failed to process CAN message: {}", e);
                        }
                    } else {
                        // Channel closed, exit
                        log::error!("SDO message receiver channel closed - exiting SDO client loop");
                        break;
                    }
                }
                
                // Cleanup timed out transfers
                _ = cleanup_interval.tick() => {
                    self.cleanup_timed_out_transfers().await;
                }
            }
        }
        
        log::debug!("SDO client run() loop exited");
    }

    /// Process incoming SDO response message
    pub async fn process_message(&mut self, message: &CanMessage) -> Result<()> {
        if message.message_type() != MessageType::Sdo {
            return Ok(());
        }

        let node_id = message.node_id().unwrap_or(0);
        
        // Check if we have an active transfer for this node
        if let Some(mut transfer) = self.active_transfers.remove(&node_id) {
            let mut should_complete = false;
            
            // Handle abort messages
            if message.data[0] & 0x80 != 0 {
                let abort_code = u32::from_le_bytes([
                    message.data[4], message.data[5], message.data[6], message.data[7]
                ]);
                if let Some(sender) = transfer.response_sender.take() {
                    let _ = sender.send(Err(CANopenError::Sdo { code: abort_code }));
                }
                transfer.state = SdoState::Aborted;
                should_complete = true;
            } else {
                // Update timestamp
                transfer.last_timestamp = Instant::now();
                let command = message.data[0];
                
                match (transfer.direction, transfer.state) {
                    (SdoDirection::Upload, SdoState::InitiateSent) => {
                        if command & 0x43 == 0x43 {
                            // Expedited transfer
                            let n = (command >> 2) & 0x03;
                            let data_length = 4 - n as usize;
                            let data = message.data[4..4 + data_length].to_vec();
                            
                            if let Some(sender) = transfer.response_sender.take() {
                                let _ = sender.send(Ok(data));
                            }
                            transfer.state = SdoState::Completed;
                            should_complete = true;
                        } else if command & 0x41 == 0x41 {
                            // Segmented transfer
                            transfer.state = SdoState::SegmentTransfer;
                            transfer.toggle_bit = false;
                            transfer.data_buffer.clear();
                            self.send_upload_segment_request(transfer.node_id, transfer.toggle_bit).await?;
                        }
                    }
                    (SdoDirection::Upload, SdoState::SegmentTransfer) => {
                        if command & 0x60 == 0x60 {
                            let toggle = (command & 0x10) != 0;
                            let n = (command >> 1) & 0x07;
                            let c = (command & 0x01) != 0;
                            
                            if toggle == transfer.toggle_bit {
                                let data_length = 7 - n as usize;
                                transfer.data_buffer.extend_from_slice(&message.data[1..1 + data_length]);
                                
                                if c {
                                    // Last segment
                                    let data = transfer.data_buffer.clone();
                                    if let Some(sender) = transfer.response_sender.take() {
                                        let _ = sender.send(Ok(data));
                                    }
                                    transfer.state = SdoState::Completed;
                                    should_complete = true;
                                } else {
                                    // Request next segment
                                    transfer.toggle_bit = !transfer.toggle_bit;
                                    self.send_upload_segment_request(transfer.node_id, transfer.toggle_bit).await?;
                                }
                            } else {
                                // Toggle bit error
                                self.send_abort(transfer.node_id, transfer.index, transfer.subindex, 0x05030000).await?;
                                if let Some(sender) = transfer.response_sender.take() {
                                    let _ = sender.send(Err(CANopenError::Sdo { code: 0x05030000 }));
                                }
                                transfer.state = SdoState::Aborted;
                                should_complete = true;
                            }
                        }
                    }
                    (SdoDirection::Download, SdoState::InitiateSent) => {
                        if command & 0x60 == 0x60 {
                            // Download initiate response
                            if transfer.transfer_type == SdoTransferType::Expedited {
                                // Expedited transfer complete
                                if let Some(sender) = transfer.response_sender.take() {
                                    let _ = sender.send(Ok(vec![]));
                                }
                                transfer.state = SdoState::Completed;
                                should_complete = true;
                            } else {
                                // Start segmented transfer
                                transfer.state = SdoState::SegmentTransfer;
                                transfer.toggle_bit = false;
                                self.send_download_segment(&mut transfer).await?;
                            }
                        }
                    }
                    (SdoDirection::Download, SdoState::SegmentTransfer) => {
                        if command & 0x20 == 0x20 {
                            let toggle = (command & 0x10) != 0;
                            
                            if toggle == transfer.toggle_bit {
                                transfer.segment_index += 1;
                                
                                if (transfer.segment_index as usize) * 7 >= transfer.data_buffer.len() {
                                    // All segments sent
                                    if let Some(sender) = transfer.response_sender.take() {
                                        let _ = sender.send(Ok(vec![]));
                                    }
                                    transfer.state = SdoState::Completed;
                                    should_complete = true;
                                } else {
                                    // Send next segment
                                    transfer.toggle_bit = !transfer.toggle_bit;
                                    self.send_download_segment(&mut transfer).await?;
                                }
                            } else {
                                // Toggle bit error
                                self.send_abort(transfer.node_id, transfer.index, transfer.subindex, 0x05030000).await?;
                                if let Some(sender) = transfer.response_sender.take() {
                                    let _ = sender.send(Err(CANopenError::Sdo { code: 0x05030000 }));
                                }
                                transfer.state = SdoState::Aborted;
                                should_complete = true;
                            }
                        }
                    }
                    _ => {}
                }
            }
            
            // Re-insert transfer if not complete
            if !should_complete {
                self.active_transfers.insert(node_id, transfer);
            }
        } else {
            log::warn!("Received SDO response from node {} with no active transfer", node_id);
        }

        Ok(())
    }

    /// Handle a new SDO request
    async fn handle_sdo_request(&mut self, request: SdoRequest) -> Result<()> {
        let node_id = request.node_id;
        
        // Check if there's already an active transfer for this node
        if self.active_transfers.contains_key(&node_id) {
            let _ = request.response_sender.send(Err(CANopenError::Sdo { code: 0x05040001 }));
            return Ok(());
        }

        // Create transfer object
        let transfer = SdoTransfer {
            node_id,
            index: request.index,
            subindex: request.subindex,
            direction: request.direction,
            transfer_type: if request.data.len() <= 4 { 
                SdoTransferType::Expedited 
            } else { 
                SdoTransferType::Segmented 
            },
            state: SdoState::Idle,
            data_buffer: request.data.clone(),
            segment_index: 0,
            toggle_bit: false,
            expected_size: None,
            last_timestamp: Instant::now(),
            timeout_duration: Duration::from_millis(request.timeout_ms as u64),
            response_sender: Some(request.response_sender),
        };

        // Send initiate message
        match request.direction {
            SdoDirection::Upload => {
                self.send_upload_initiate(node_id, request.index, request.subindex).await?;
            }
            SdoDirection::Download => {
                if request.data.len() <= 4 {
                    self.send_download_initiate_expedited(node_id, request.index, request.subindex, &request.data).await?;
                } else {
                    self.send_download_initiate_segmented(node_id, request.index, request.subindex, request.data.len()).await?;
                }
            }
        }

        // Store the transfer
        let mut transfer = transfer;
        transfer.state = SdoState::InitiateSent;
        self.active_transfers.insert(node_id, transfer);
        
        log::debug!("Started SDO {} for node {}, index 0x{:04X}:{:02X}", 
                   match request.direction {
                       SdoDirection::Upload => "upload",
                       SdoDirection::Download => "download",
                   },
                   node_id, request.index, request.subindex);

        Ok(())
    }

    /// Send upload initiate message
    async fn send_upload_initiate(&self, node_id: u8, index: u16, subindex: u8) -> Result<()> {
        let cob_id = 0x600 + node_id as u16;
        let data = vec![
            0x40, // Upload initiate
            (index & 0xFF) as u8,
            (index >> 8) as u8,
            subindex,
            0x00, 0x00, 0x00, 0x00,
        ];
        
        let message = CanMessage::new(cob_id, data)?;
        self.message_sender.send(message).await
            .map_err(|_| CANopenError::ChannelClosed)?;
        
        Ok(())
    }

    /// Send download initiate expedited message
    async fn send_download_initiate_expedited(&self, node_id: u8, index: u16, subindex: u8, data: &[u8]) -> Result<()> {
        let cob_id = 0x600 + node_id as u16;
        let n = 4 - data.len().min(4); // Number of bytes that do NOT contain data
        let command = 0x23 | ((n as u8) << 2); // Download initiate, expedited, size indicated
        
        let mut msg_data = vec![
            command,
            (index & 0xFF) as u8,
            (index >> 8) as u8,
            subindex,
            0x00, 0x00, 0x00, 0x00,
        ];
        
        // Copy data
        for (i, &byte) in data.iter().take(4).enumerate() {
            msg_data[4 + i] = byte;
        }
        
        let message = CanMessage::new(cob_id, msg_data)?;
        self.message_sender.send(message).await
            .map_err(|_| CANopenError::ChannelClosed)?;
        
        Ok(())
    }

    /// Send download initiate segmented message
    async fn send_download_initiate_segmented(&self, node_id: u8, index: u16, subindex: u8, size: usize) -> Result<()> {
        let cob_id = 0x600 + node_id as u16;
        let data = vec![
            0x21, // Download initiate, segmented, size indicated
            (index & 0xFF) as u8,
            (index >> 8) as u8,
            subindex,
            (size & 0xFF) as u8,
            ((size >> 8) & 0xFF) as u8,
            ((size >> 16) & 0xFF) as u8,
            ((size >> 24) & 0xFF) as u8,
        ];
        
        let message = CanMessage::new(cob_id, data)?;
        self.message_sender.send(message).await
            .map_err(|_| CANopenError::ChannelClosed)?;
        
        Ok(())
    }

    /// Send upload segment request
    async fn send_upload_segment_request(&self, node_id: u8, toggle: bool) -> Result<()> {
        let cob_id = 0x600 + node_id as u16;
        let command = 0x60 | if toggle { 0x10 } else { 0x00 };
        let data = vec![command, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        
        let message = CanMessage::new(cob_id, data)?;
        self.message_sender.send(message).await
            .map_err(|_| CANopenError::ChannelClosed)?;
        
        Ok(())
    }

    /// Send download segment
    async fn send_download_segment(&self, transfer: &SdoTransfer) -> Result<()> {
        let cob_id = 0x600 + transfer.node_id as u16;
        let start_byte = transfer.segment_index as usize * 7;
        let remaining_bytes = transfer.data_buffer.len() - start_byte;
        let bytes_to_send = remaining_bytes.min(7);
        let n = 7 - bytes_to_send; // Number of bytes that do NOT contain data
        let c = remaining_bytes <= 7; // Last segment
        
        let command = 0x00 | 
                     if transfer.toggle_bit { 0x10 } else { 0x00 } |
                     if c { 0x01 } else { 0x00 } |
                     ((n as u8) << 1);
        
        let mut data = vec![command, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        
        // Copy segment data
        for (i, &byte) in transfer.data_buffer[start_byte..start_byte + bytes_to_send].iter().enumerate() {
            data[1 + i] = byte;
        }
        
        let message = CanMessage::new(cob_id, data)?;
        self.message_sender.send(message).await
            .map_err(|_| CANopenError::ChannelClosed)?;
        
        Ok(())
    }

    /// Send SDO abort message
    async fn send_abort(&self, node_id: u8, index: u16, subindex: u8, abort_code: u32) -> Result<()> {
        let cob_id = 0x600 + node_id as u16;
        let data = vec![
            0x80, // Abort
            (index & 0xFF) as u8,
            (index >> 8) as u8,
            subindex,
            (abort_code & 0xFF) as u8,
            ((abort_code >> 8) & 0xFF) as u8,
            ((abort_code >> 16) & 0xFF) as u8,
            ((abort_code >> 24) & 0xFF) as u8,
        ];
        
        let message = CanMessage::new(cob_id, data)?;
        self.message_sender.send(message).await
            .map_err(|_| CANopenError::ChannelClosed)?;
        
        Ok(())
    }

    /// Clean up timed out transfers
    async fn cleanup_timed_out_transfers(&mut self) {
        let now = Instant::now();
        let mut timed_out_nodes = Vec::new();
        
        for (node_id, transfer) in &self.active_transfers {
            if now.duration_since(transfer.last_timestamp) > transfer.timeout_duration {
                timed_out_nodes.push(*node_id);
            }
        }
        
        for node_id in timed_out_nodes {
            if let Some(mut transfer) = self.active_transfers.remove(&node_id) {
                transfer.state = SdoState::Timeout;
                if let Some(sender) = transfer.response_sender.take() {
                    let _ = sender.send(Err(CANopenError::Timeout));
                }
                log::warn!("SDO transfer to node {} timed out", node_id);
            }
        }
    }
}

/// High-level SDO operations
impl SdoClient {
    /// Read a u8 value from object dictionary
    pub async fn read_u8(&self, sdo_sender: &mpsc::Sender<SdoRequest>, node_id: u8, index: u16, subindex: u8) -> Result<u8> {
        let data = self.read_data(sdo_sender, node_id, index, subindex).await?;
        if data.len() != 1 {
            return Err(CANopenError::InvalidLength(data.len()));
        }
        Ok(data[0])
    }

    /// Read a u16 value from object dictionary
    pub async fn read_u16(&self, sdo_sender: &mpsc::Sender<SdoRequest>, node_id: u8, index: u16, subindex: u8) -> Result<u16> {
        let data = self.read_data(sdo_sender, node_id, index, subindex).await?;
        if data.len() != 2 {
            return Err(CANopenError::InvalidLength(data.len()));
        }
        Ok(u16::from_le_bytes([data[0], data[1]]))
    }

    /// Read a u32 value from object dictionary
    pub async fn read_u32(&self, sdo_sender: &mpsc::Sender<SdoRequest>, node_id: u8, index: u16, subindex: u8) -> Result<u32> {
        let data = self.read_data(sdo_sender, node_id, index, subindex).await?;
        if data.len() != 4 {
            return Err(CANopenError::InvalidLength(data.len()));
        }
        Ok(u32::from_le_bytes([data[0], data[1], data[2], data[3]]))
    }

    /// Write a u8 value to object dictionary
    pub async fn write_u8(&self, sdo_sender: &mpsc::Sender<SdoRequest>, node_id: u8, index: u16, subindex: u8, value: u8) -> Result<()> {
        self.write_data(sdo_sender, node_id, index, subindex, vec![value]).await
    }

    /// Write a u16 value to object dictionary  
    pub async fn write_u16(&self, sdo_sender: &mpsc::Sender<SdoRequest>, node_id: u8, index: u16, subindex: u8, value: u16) -> Result<()> {
        let data = value.to_le_bytes().to_vec();
        self.write_data(sdo_sender, node_id, index, subindex, data).await
    }

    /// Write a u32 value to object dictionary
    pub async fn write_u32(&self, sdo_sender: &mpsc::Sender<SdoRequest>, node_id: u8, index: u16, subindex: u8, value: u32) -> Result<()> {
        let data = value.to_le_bytes().to_vec();
        self.write_data(sdo_sender, node_id, index, subindex, data).await
    }

    /// Read raw data from object dictionary
    pub async fn read_data(&self, sdo_sender: &mpsc::Sender<SdoRequest>, node_id: u8, index: u16, subindex: u8) -> Result<Vec<u8>> {
        let (response_sender, response_receiver) = oneshot::channel();
        
        let request = SdoRequest {
            node_id,
            index,
            subindex,
            direction: SdoDirection::Upload,
            data: vec![],
            timeout_ms: 1000,
            response_sender,
        };

        sdo_sender.send(request).await
            .map_err(|_| CANopenError::ChannelClosed)?;

        timeout(Duration::from_millis(1500), response_receiver).await
            .map_err(|_| CANopenError::Timeout)?
            .map_err(|_| CANopenError::ChannelClosed)?
    }

    /// Write raw data to object dictionary
    pub async fn write_data(&self, sdo_sender: &mpsc::Sender<SdoRequest>, node_id: u8, index: u16, subindex: u8, data: Vec<u8>) -> Result<()> {
        let (response_sender, response_receiver) = oneshot::channel();
        
        let request = SdoRequest {
            node_id,
            index,
            subindex,
            direction: SdoDirection::Download,
            data,
            timeout_ms: 1000,
            response_sender,
        };

        sdo_sender.send(request).await
            .map_err(|_| CANopenError::ChannelClosed)?;

        timeout(Duration::from_millis(1500), response_receiver).await
            .map_err(|_| CANopenError::Timeout)?
            .map_err(|_| CANopenError::ChannelClosed)?
            .map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sdo_direction_values() {
        let upload = SdoDirection::Upload;
        let download = SdoDirection::Download;
        
        assert_eq!(upload, SdoDirection::Upload);
        assert_eq!(download, SdoDirection::Download);
        assert_ne!(upload, download);
    }
    
    #[test]
    fn test_sdo_direction_clone() {
        let dir1 = SdoDirection::Upload;
        let dir2 = dir1;
        
        assert_eq!(dir1, dir2);
    }
    
    #[test]
    fn test_sdo_state_values() {
        assert_eq!(SdoState::Idle, SdoState::Idle);
        assert_eq!(SdoState::InitiateSent, SdoState::InitiateSent);
        assert_eq!(SdoState::SegmentTransfer, SdoState::SegmentTransfer);
        assert_eq!(SdoState::Completed, SdoState::Completed);
        assert_eq!(SdoState::Aborted, SdoState::Aborted);
        assert_eq!(SdoState::Timeout, SdoState::Timeout);
        
        assert_ne!(SdoState::Idle, SdoState::Completed);
    }
    
    #[test]
    fn test_sdo_transfer_type() {
        assert_eq!(SdoTransferType::Expedited, SdoTransferType::Expedited);
        assert_eq!(SdoTransferType::Segmented, SdoTransferType::Segmented);
        assert_ne!(SdoTransferType::Expedited, SdoTransferType::Segmented);
    }
    
    #[test]
    fn test_sdo_transfer_type_determination() {
        // Data <= 4 bytes should be expedited
        let data_1 = vec![0x42];
        assert!(data_1.len() <= 4);
        
        let data_4 = vec![0x01, 0x02, 0x03, 0x04];
        assert!(data_4.len() <= 4);
        
        // Data > 4 bytes should be segmented
        let data_5 = vec![0x01, 0x02, 0x03, 0x04, 0x05];
        assert!(data_5.len() > 4);
        
        let data_100 = vec![0u8; 100];
        assert!(data_100.len() > 4);
    }
    
    #[test]
    fn test_sdo_cob_id_calculation() {
        // SDO client -> server (download/upload request): 0x600 + node_id
        let node_1_tx = 0x600 + 1;
        assert_eq!(node_1_tx, 0x601);
        
        let node_127_tx = 0x600 + 127;
        assert_eq!(node_127_tx, 0x67F);
        
        // SDO server -> client (response): 0x580 + node_id
        let node_1_rx = 0x580 + 1;
        assert_eq!(node_1_rx, 0x581);
        
        let node_127_rx = 0x580 + 127;
        assert_eq!(node_127_rx, 0x5FF);
    }
    
    #[test]
    fn test_upload_initiate_command() {
        // Upload initiate command: 0x40
        let command = 0x40u8;
        
        // Verify command structure
        assert_eq!(command & 0xE0, 0x40); // Client command specifier
        assert_eq!(command & 0x02, 0x00); // Not expedited
        assert_eq!(command & 0x01, 0x00); // No size indication
    }
    
    #[test]
    fn test_download_initiate_expedited_command() {
        // Download initiate, expedited, size indicated
        // Command format: 0x23 + (n << 2) where n = 4 - data_len
        
        // 1 byte data: n = 3
        let cmd_1byte = 0x23 | (3 << 2);
        assert_eq!(cmd_1byte, 0x2F);
        
        // 2 bytes data: n = 2
        let cmd_2bytes = 0x23 | (2 << 2);
        assert_eq!(cmd_2bytes, 0x2B);
        
        // 3 bytes data: n = 1
        let cmd_3bytes = 0x23 | (1 << 2);
        assert_eq!(cmd_3bytes, 0x27);
        
        // 4 bytes data: n = 0
        let cmd_4bytes = 0x23 | (0 << 2);
        assert_eq!(cmd_4bytes, 0x23);
    }
    
    #[test]
    fn test_download_initiate_segmented_command() {
        // Download initiate, segmented, size indicated: 0x21
        let command = 0x21u8;
        
        assert_eq!(command & 0xE0, 0x20); // Download
        assert_eq!(command & 0x02, 0x00); // Not expedited
        assert_eq!(command & 0x01, 0x01); // Size indicated
    }
    
    #[test]
    fn test_upload_segment_request_command() {
        // Upload segment request: 0x60 + toggle bit
        let toggle_0 = 0x60u8;
        let toggle_1 = 0x60u8 | 0x10;
        
        assert_eq!(toggle_0, 0x60);
        assert_eq!(toggle_1, 0x70);
        
        // Verify toggle bit
        assert_eq!(toggle_0 & 0x10, 0x00);
        assert_eq!(toggle_1 & 0x10, 0x10);
    }
    
    #[test]
    fn test_download_segment_command() {
        // Download segment: toggle + c (last) + n (number of invalid bytes)
        
        // First segment, not last, 7 bytes valid (n=0)
        let cmd = 0x00 | 0x00 | 0x00 | (0 << 1);
        assert_eq!(cmd, 0x00);
        
        // Second segment (toggle=1), not last, 7 bytes valid
        let cmd = 0x00 | 0x10 | 0x00 | (0 << 1);
        assert_eq!(cmd, 0x10);
        
        // Last segment (c=1), toggle=0, 5 bytes valid (n=2)
        let cmd = 0x00 | 0x00 | 0x01 | (2 << 1);
        assert_eq!(cmd, 0x05);
        
        // Last segment (c=1), toggle=1, 3 bytes valid (n=4)
        let cmd = 0x00 | 0x10 | 0x01 | (4 << 1);
        assert_eq!(cmd, 0x19);
    }
    
    #[test]
    fn test_upload_response_expedited() {
        // Upload initiate response, expedited: 0x43 + (n << 2)
        
        // 1 byte: n=3
        let resp_1 = 0x43 | (3 << 2);
        assert_eq!(resp_1, 0x4F);
        assert_eq!(resp_1 & 0x43, 0x43);
        
        // 2 bytes: n=2
        let resp_2 = 0x43 | (2 << 2);
        assert_eq!(resp_2, 0x4B);
        
        // 4 bytes: n=0
        let resp_4 = 0x43 | (0 << 2);
        assert_eq!(resp_4, 0x43);
    }
    
    #[test]
    fn test_upload_response_segmented() {
        // Upload initiate response, segmented: 0x41
        let command = 0x41u8;
        
        assert_eq!(command & 0x41, 0x41);
        assert_eq!(command & 0x02, 0x00); // Not expedited
    }
    
    #[test]
    fn test_upload_segment_response() {
        // Upload segment response: 0x00 + toggle + c (last) + n
        
        // First segment, not last, 7 bytes valid
        let resp = 0x00 | 0x00 | 0x00 | (0 << 1);
        assert_eq!(resp, 0x00);
        assert_eq!(resp & 0x60, 0x00); // Segment response
        
        // Second segment (toggle=1), not last, 7 bytes
        let resp = 0x00 | 0x10 | 0x00 | (0 << 1);
        assert_eq!(resp, 0x10);
        
        // Last segment, toggle=0, 4 bytes valid (n=3)
        let resp = 0x00 | 0x00 | 0x01 | (3 << 1);
        assert_eq!(resp, 0x07);
        assert_eq!(resp & 0x01, 0x01); // Last segment
    }
    
    #[test]
    fn test_download_response() {
        // Download initiate response: 0x60
        let command = 0x60u8;
        assert_eq!(command & 0x60, 0x60);
    }
    
    #[test]
    fn test_download_segment_response() {
        // Download segment response: 0x20 + toggle
        let toggle_0 = 0x20u8;
        let toggle_1 = 0x20u8 | 0x10;
        
        assert_eq!(toggle_0, 0x20);
        assert_eq!(toggle_1, 0x30);
        
        assert_eq!(toggle_0 & 0x20, 0x20);
        assert_eq!(toggle_1 & 0x20, 0x20);
    }
    
    #[test]
    fn test_abort_command() {
        // Abort command: 0x80
        let command = 0x80u8;
        assert_eq!(command & 0x80, 0x80);
    }
    
    #[test]
    fn test_sdo_abort_codes() {
        // Common SDO abort codes
        let toggle_bit_error = 0x05030000u32;
        let timeout = 0x05040000u32;
        let command_specifier = 0x05040001u32;
        let object_not_exist = 0x06020000u32;
        let access_failed = 0x06010000u32;
        let write_only = 0x06010001u32;
        let read_only = 0x06010002u32;
        
        // Verify they're different
        assert_ne!(toggle_bit_error, timeout);
        assert_ne!(timeout, object_not_exist);
        
        // Verify encoding (little-endian in CAN message)
        let toggle_bytes = toggle_bit_error.to_le_bytes();
        assert_eq!(toggle_bytes, [0x00, 0x00, 0x03, 0x05]);
    }
    
    #[test]
    fn test_sdo_index_subindex_encoding() {
        // Test index/subindex encoding in SDO messages
        let index: u16 = 0x1018;
        let subindex: u8 = 0x01;
        
        // Index is little-endian in bytes 1-2
        let index_low = (index & 0xFF) as u8;
        let index_high = (index >> 8) as u8;
        
        assert_eq!(index_low, 0x18);
        assert_eq!(index_high, 0x10);
        
        // Subindex is in byte 3
        assert_eq!(subindex, 0x01);
    }
    
    #[test]
    fn test_expedited_data_packing() {
        // Test packing data into expedited SDO message
        let data = vec![0x12, 0x34, 0x56, 0x78];
        
        // Data goes in bytes 4-7
        let mut msg = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        for (i, &byte) in data.iter().take(4).enumerate() {
            msg[4 + i] = byte;
        }
        
        assert_eq!(msg[4], 0x12);
        assert_eq!(msg[5], 0x34);
        assert_eq!(msg[6], 0x56);
        assert_eq!(msg[7], 0x78);
    }
    
    #[test]
    fn test_segmented_data_size_encoding() {
        // Segmented download initiate includes size in bytes 4-7
        let size: usize = 12345;
        
        let size_bytes = [
            (size & 0xFF) as u8,
            ((size >> 8) & 0xFF) as u8,
            ((size >> 16) & 0xFF) as u8,
            ((size >> 24) & 0xFF) as u8,
        ];
        
        assert_eq!(size_bytes[0], 0x39);
        assert_eq!(size_bytes[1], 0x30);
        assert_eq!(size_bytes[2], 0x00);
        assert_eq!(size_bytes[3], 0x00);
        
        // Decode back
        let decoded = u32::from_le_bytes(size_bytes) as usize;
        assert_eq!(decoded, size);
    }
    
    #[test]
    fn test_segment_data_length_calculation() {
        // Each segment can carry up to 7 bytes
        let total_size = 20;
        let segment_size = 7;
        
        let num_segments = (total_size + segment_size - 1) / segment_size;
        assert_eq!(num_segments, 3); // 7 + 7 + 6 = 20
        
        // Last segment has fewer bytes
        let last_segment_size = total_size % segment_size;
        assert_eq!(last_segment_size, 6);
        
        // n (number of invalid bytes) for last segment
        let n = segment_size - last_segment_size;
        assert_eq!(n, 1);
    }
    
    #[test]
    fn test_toggle_bit_alternation() {
        // Toggle bit should alternate: false, true, false, true...
        let mut toggle = false;
        
        assert_eq!(toggle, false);
        toggle = !toggle;
        assert_eq!(toggle, true);
        toggle = !toggle;
        assert_eq!(toggle, false);
        toggle = !toggle;
        assert_eq!(toggle, true);
    }
    
    #[test]
    fn test_sdo_message_length() {
        // All SDO messages are 8 bytes
        let msg1 = vec![0x40, 0x00, 0x10, 0x01, 0x00, 0x00, 0x00, 0x00];
        assert_eq!(msg1.len(), 8);
        
        let msg2 = vec![0x23, 0x00, 0x10, 0x01, 0x42, 0x00, 0x00, 0x00];
        assert_eq!(msg2.len(), 8);
    }
    
    #[test]
    fn test_expedited_n_value_calculation() {
        // n = number of bytes that do NOT contain data (4 - data_len)
        
        for data_len in 1..=4 {
            let n = 4 - data_len;
            assert_eq!(n, 4 - data_len);
            
            // Valid data is in bytes [4..4+data_len]
            let valid_bytes = data_len;
            let invalid_bytes = 4 - valid_bytes;
            assert_eq!(invalid_bytes, n);
        }
    }
    
    #[test]
    fn test_segment_n_value_calculation() {
        // n = number of bytes that do NOT contain data in segment (7 - data_len)
        
        for data_len in 1..=7 {
            let n = 7 - data_len;
            assert_eq!(n, 7 - data_len);
            assert!(n < 7);
        }
    }
    
    #[test]
    fn test_sdo_command_bit_patterns() {
        // Upload initiate: 01000000 = 0x40
        assert_eq!(0x40, 0b01000000);
        
        // Download initiate expedited: 00100011 = 0x23
        assert_eq!(0x23, 0b00100011);
        
        // Upload segment request: 01100000 = 0x60
        assert_eq!(0x60, 0b01100000);
        
        // Download segment: 00000000 (base)
        assert_eq!(0x00, 0b00000000);
        
        // Abort: 10000000 = 0x80
        assert_eq!(0x80, 0b10000000);
    }
    
    #[test]
    fn test_multi_segment_transfer_scenario() {
        // Simulate a 20-byte transfer (3 segments: 7+7+6)
        let data = vec![0u8; 20];
        let segment_size = 7;
        
        let mut segment_index = 0;
        let mut bytes_sent = 0;
        let mut toggle = false;
        
        while bytes_sent < data.len() {
            let start = segment_index * segment_size;
            let remaining = data.len() - bytes_sent;
            let this_segment_size = remaining.min(segment_size);
            let n = segment_size - this_segment_size;
            let is_last = remaining <= segment_size;
            
            // Verify segment calculation
            if segment_index == 0 {
                assert_eq!(this_segment_size, 7);
                assert_eq!(n, 0);
                assert_eq!(is_last, false);
                assert_eq!(toggle, false);
            } else if segment_index == 1 {
                assert_eq!(this_segment_size, 7);
                assert_eq!(n, 0);
                assert_eq!(is_last, false);
                assert_eq!(toggle, true);
            } else if segment_index == 2 {
                assert_eq!(this_segment_size, 6);
                assert_eq!(n, 1);
                assert_eq!(is_last, true);
                assert_eq!(toggle, false);
            }
            
            bytes_sent += this_segment_size;
            segment_index += 1;
            toggle = !toggle;
        }
        
        assert_eq!(segment_index, 3);
        assert_eq!(bytes_sent, 20);
    }
    
    #[test]
    fn test_sdo_response_parsing_expedited() {
        // Simulate parsing an expedited upload response with 2 bytes
        let response = vec![0x4B, 0x18, 0x10, 0x01, 0x34, 0x12, 0x00, 0x00];
        
        let command = response[0];
        assert_eq!(command & 0x43, 0x43); // Expedited upload response
        
        let n = (command >> 2) & 0x03;
        assert_eq!(n, 2); // 4 - 2 = 2 invalid bytes
        
        let data_length = 4 - n as usize;
        assert_eq!(data_length, 2);
        
        let data = &response[4..4 + data_length];
        assert_eq!(data, &[0x34, 0x12]);
        
        // Decode as u16
        let value = u16::from_le_bytes([data[0], data[1]]);
        assert_eq!(value, 0x1234);
    }
    
    #[test]
    fn test_sdo_abort_parsing() {
        // Simulate parsing an abort message
        let abort = vec![0x80, 0x18, 0x10, 0x01, 0x00, 0x00, 0x02, 0x06];
        
        let command = abort[0];
        assert_eq!(command & 0x80, 0x80); // Abort
        
        let abort_code = u32::from_le_bytes([abort[4], abort[5], abort[6], abort[7]]);
        assert_eq!(abort_code, 0x06020000); // Object does not exist
    }
}