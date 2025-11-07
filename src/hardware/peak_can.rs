// PEAK CAN adapter implementation using peak-can-sys

use async_trait::async_trait;
use tokio::sync::{mpsc, RwLock, broadcast};
use tokio::task::JoinHandle;
use std::sync::Arc;
use std::time::Duration;
// Import the functions we know are available
use peak_can_sys::{CAN_Initialize, CAN_Uninitialize, CAN_Read, CAN_Write};

// Define PCAN constants that should be available
// Constants for PEAK CAN operations
const PCAN_ERROR_OK: u32 = 0x00000;
const PCAN_ERROR_QRCVEMPTY: u32 = 0x0020;  // Receive queue is empty
const PCAN_USBBUS1: u16 = 0x51;
const PCAN_BAUD_125K: u16 = 0x031C;
const PCAN_BAUD_250K: u16 = 0x011C;
const PCAN_BAUD_500K: u16 = 0x001C;
const PCAN_BAUD_1M: u16 = 0x0014;
const PCAN_MESSAGE_STANDARD: u8 = 0x00;
const PCAN_MESSAGE_RTR: u8 = 0x02;
use crate::canopen::message::CanMessage;
use crate::{Result, CANopenError};
use crate::hardware::{CanHardware, BusSpeed};impl BusSpeed {
    /// Convert to PEAK CAN baud rate value
    pub fn to_pcan_baud(self) -> u16 {
        self as u16
    }
}

/// PEAK CAN adapter handle types
#[derive(Debug, Clone, Copy)]
pub enum PcanHandle {
    PcanUsbbus1 = 0x51,
    PcanUsbbus2 = 0x52,
    PcanUsbbus3 = 0x53,
    PcanUsbbus4 = 0x54,
    PcanUsbbus5 = 0x55,
    PcanUsbbus6 = 0x56,
    PcanUsbbus7 = 0x57,
    PcanUsbbus8 = 0x58,
}

/// PEAK CAN message structure for interfacing with peak-can-sys
/// This matches the TPCANMsg structure from PCAN API
#[repr(C)]
pub struct PcanMessage {
    pub id: u32,           // 32-bit CAN identifier
    pub msg_type: u8,      // Type of the message (Standard, RTR, etc.)
    pub len: u8,           // Data Length Code of the message (0..8)
    pub data: [u8; 8],     // Data of the message (DATA[0]..DATA[7])
}

/// PEAK CAN timestamp structure
#[repr(C)]
pub struct PcanTimestamp {
    pub millis: u32,       // Base-value: milliseconds: 0.. 2^32-1
    pub millis_overflow: u16,  // Roll-arounds of millis
    pub micros: u16,       // Microseconds: 0..999
}

/// PEAK CAN adapter implementation
pub struct PeakCanAdapter {
    handle: PcanHandle,
    speed: BusSpeed,
    is_connected: Arc<RwLock<bool>>,
    message_tx: Option<broadcast::Sender<CanMessage>>,
    receive_task: Option<JoinHandle<()>>,
}

impl PeakCanAdapter {
    pub fn new(handle: PcanHandle, speed: BusSpeed) -> Self {
        Self {
            handle,
            speed,
            is_connected: Arc::new(RwLock::new(false)),
            message_tx: None,
            receive_task: None,
        }
    }

    /// Convert our CanMessage to PEAK CAN format
    fn can_message_to_pcan(message: &CanMessage) -> PcanMessage {
        let mut data = [0u8; 8];
        let len = message.data.len().min(8);
        data[..len].copy_from_slice(&message.data[..len]);
        
        PcanMessage {
            id: message.id.raw() as u32,
            msg_type: if message.remote { 0x02 } else { 0x00 }, // RTR flag
            len: len as u8,
            data,
        }
    }

    /// Convert PEAK CAN message to our CanMessage
    fn pcan_to_can_message(pcan_msg: &PcanMessage) -> Result<CanMessage> {
        // Validate message length
        if pcan_msg.len > 8 {
            return Err(CANopenError::InvalidLength(pcan_msg.len as usize));
        }

        // Validate CAN ID (CANopen uses 11-bit IDs)
        if pcan_msg.id > 0x7FF {
            return Err(CANopenError::InvalidData("CAN ID exceeds 11-bit limit".to_string()));
        }

        // Extract data payload
        let data = pcan_msg.data[..pcan_msg.len as usize].to_vec();
        
        // Check if this is a remote transmission request
        let remote = (pcan_msg.msg_type & 0x02) != 0; // PCAN_MESSAGE_RTR

        // Create CANopen message
        CanMessage::with_timestamp(pcan_msg.id as u16, data, remote)
    }

    /// Start the message receiving task
    async fn start_receive_task(&mut self) -> Result<()> {
        let (tx, _) = broadcast::channel(1000);
        let handle = self.handle;
        let is_connected = Arc::clone(&self.is_connected);
        
        // Clone the sender for the task
        let task_tx = tx.clone();
        
        let task = tokio::spawn(async move {
            let mut pcan_msg = PcanMessage {
                id: 0,
                msg_type: 0,
                len: 0,
                data: [0; 8],
            };
            let mut timestamp = PcanTimestamp {
                millis: 0,
                millis_overflow: 0,
                micros: 0,
            };
            
            while *is_connected.read().await {
                // Try to read a message from PEAK CAN
                let result = unsafe {
                    CAN_Read(
                        handle as u16,
                        &mut pcan_msg as *mut PcanMessage as *mut _,
                        &mut timestamp as *mut PcanTimestamp as *mut _
                    )
                };
                
                if result == PCAN_ERROR_OK {
                    // Successfully read a message, convert and broadcast it
                    match Self::pcan_to_can_message(&pcan_msg) {
                        Ok(can_message) => {
                            if let Err(_) = task_tx.send(can_message.clone()) {
                                log::warn!("Failed to broadcast received CAN message - no receivers");
                            } else {
                                log::trace!("Received and broadcasted CAN message: ID=0x{:03X}, {} bytes", 
                                           can_message.id.raw(), can_message.data.len());
                            }
                        }
                        Err(e) => {
                            log::error!("Failed to convert received PCAN message: {}", e);
                        }
                    }
                } else if result == PCAN_ERROR_QRCVEMPTY {
                    // No message available, wait a bit before trying again
                    tokio::time::sleep(Duration::from_millis(1)).await;
                } else {
                    // Other error occurred
                    log::error!("Error reading from PCAN: status {:#X}", result);
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            }
            
            log::info!("PEAK CAN receive task terminated");
        });

        self.message_tx = Some(tx);
        self.receive_task = Some(task);
        
        Ok(())
    }

    /// Stop the receiving task
    async fn stop_receive_task(&mut self) {
        if let Some(task) = self.receive_task.take() {
            task.abort();
            let _ = task.await;
        }
        self.message_tx = None;
    }
}

#[async_trait]
impl CanHardware for PeakCanAdapter {
    async fn connect(&mut self, bus_speed: BusSpeed) -> Result<()> {
        log::info!("Connecting to PEAK CAN adapter at {:?}", bus_speed);
        
        // Convert bus speed to PEAK CAN baud rate values
        let pcan_baud = match bus_speed {
            BusSpeed::Baud125K => 0x031C,  // PCAN_BAUD_125K
            BusSpeed::Baud250K => 0x011C,  // PCAN_BAUD_250K  
            BusSpeed::Baud500K => 0x001C,  // PCAN_BAUD_500K
            BusSpeed::Baud1M => 0x0014,    // PCAN_BAUD_1M
        };

        // Use PCAN_USBBUS1 as default handle
        let handle = PcanHandle::PcanUsbbus1;
        self.handle = handle;

        // Initialize PEAK CAN hardware
        let result = unsafe { 
            CAN_Initialize(handle as u16, pcan_baud, 0, 0, 0) 
        };
        
        if result != PCAN_ERROR_OK {
            return Err(CANopenError::PeakCan(format!(
                "Failed to initialize PCAN on handle {:?}: status {:#X}", 
                handle, result
            )));
        }
        
        log::info!("PEAK CAN initialized successfully on handle {:?} with baud rate {:#X}", handle, pcan_baud);
        *self.is_connected.write().await = true;
        
        // Start the message receiving task
        self.start_receive_task().await?;
        
        log::info!("Successfully connected to PEAK CAN adapter");
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        log::info!("Disconnecting from PEAK CAN adapter");
        
        // Stop the receiving task first
        self.stop_receive_task().await;
        
        // Uninitialize PEAK CAN hardware
        let handle = self.handle;
        let result = unsafe { 
            CAN_Uninitialize(handle as u16) 
        };
        
        if result != PCAN_ERROR_OK {
            log::warn!("Failed to properly uninitialize PCAN handle {:?}: status {:#X}", handle, result);
        } else {
            log::info!("PEAK CAN disconnected successfully from handle {:?}", handle);
        }
        
        *self.is_connected.write().await = false;
        
        log::info!("Successfully disconnected from PEAK CAN adapter");
        Ok(())
    }

    async fn send_message(&self, message: &CanMessage) -> Result<()> {
        if !*self.is_connected.read().await {
            return Err(CANopenError::Connection);
        }
        
        log::debug!("Sending CAN message: ID=0x{:03X}, Data={:02X?}", 
                   message.id.raw(), message.data);
        
        // Convert to PEAK CAN format
        let pcan_msg = Self::can_message_to_pcan(message);
        
        // Send message via PEAK CAN API
        let result = unsafe { 
            CAN_Write(
                self.handle as u16, 
                &pcan_msg as *const PcanMessage as *mut _
            ) 
        };
        
        if result != PCAN_ERROR_OK {
            return Err(CANopenError::PeakCan(format!(
                "Failed to send CAN message ID=0x{:03X}: status {:#X}", 
                message.id.raw(), result
            )));
        }
        
        log::trace!("CAN message sent successfully: ID=0x{:03X}, {} bytes", 
                   message.id.raw(), message.data.len());
        
        Ok(())
    }

    fn subscribe_messages(&self) -> mpsc::Receiver<CanMessage> {
        if let Some(tx) = &self.message_tx {
            let mut rx = tx.subscribe();
            let (mpsc_tx, mpsc_rx) = mpsc::channel(1000);
            
            // Spawn a task to convert broadcast to mpsc
            tokio::spawn(async move {
                while let Ok(message) = rx.recv().await {
                    if mpsc_tx.send(message).await.is_err() {
                        break; // Receiver dropped
                    }
                }
            });
            
            mpsc_rx
        } else {
            // Return a closed channel if not connected
            let (_tx, rx) = mpsc::channel(1);
            rx
        }
    }

    fn is_connected(&self) -> bool {
        // This is a best-effort synchronous check
        // In practice, we'd use Arc<AtomicBool> for better performance
        match self.is_connected.try_read() {
            Ok(connected) => *connected,
            Err(_) => false,
        }
    }
}