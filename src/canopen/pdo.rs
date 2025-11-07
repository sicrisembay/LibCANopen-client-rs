//! PDO (Process Data Object) handling
//! 
//! PDOs are used for real-time data exchange without the overhead of SDO protocol.
//! They are typically used for:
//! - Streaming sensor data
//! - Sending control commands
//! - Synchronized data exchange
//! 
//! PDO COB-ID Ranges:
//! - TPDO1: 0x180-0x1FF (Transmit from device perspective)
//! - TPDO2: 0x280-0x2FF
//! - TPDO3: 0x380-0x3FF
//! - TPDO4: 0x480-0x4FF
//! - RPDO1: 0x200-0x27F (Receive from device perspective)
//! - RPDO2: 0x300-0x37F
//! - RPDO3: 0x400-0x47F
//! - RPDO4: 0x500-0x57F

use crate::Result;
use crate::canopen::message::CanMessage;
use std::collections::HashMap;
use std::sync::Arc;

/// Type alias for PDO callback functions
pub type PdoCallback = Arc<dyn Fn(&[u8]) + Send + Sync>;

/// PDO Manager - handles Process Data Objects for real-time data exchange
/// 
/// PDO messages are used for efficient, real-time data transfer without
/// the request/response overhead of SDO protocol.
pub struct PdoManager {
    /// Registered callbacks for specific COB-IDs
    /// When a PDO with matching COB-ID is received, the callback is invoked with the data
    pdo_callbacks: HashMap<u16, PdoCallback>,
    
    /// Storage for recent PDO messages for applications that don't use callbacks
    recent_pdos: HashMap<u16, Vec<u8>>,
}

impl PdoManager {
    /// Create a new PDO manager
    pub fn new() -> Self {
        Self {
            pdo_callbacks: HashMap::new(),
            recent_pdos: HashMap::new(),
        }
    }

    /// Register a callback handler for PDO messages with a specific COB-ID
    /// 
    /// When a PDO message is received with the specified COB-ID, the handler
    /// function will be called with the PDO data payload.
    /// 
    /// # Arguments
    /// * `cob_id` - The COB-ID to listen for (e.g., 0x181 for TPDO1 from node 1)
    /// * `handler` - Callback function that receives the PDO data bytes
    /// 
    /// # Example
    /// ```ignore
    /// manager.register_pdo_handler(0x181, |data| {
    ///     println!("Received PDO from node 1: {:?}", data);
    /// });
    /// ```
    pub fn register_pdo_handler<F>(&mut self, cob_id: u16, handler: F)
    where
        F: Fn(&[u8]) + Send + Sync + 'static,
    {
        log::debug!("Registering PDO handler for COB-ID 0x{:03X}", cob_id);
        self.pdo_callbacks.insert(cob_id, Arc::new(handler));
    }

    /// Unregister a PDO handler for a specific COB-ID
    /// 
    /// # Arguments
    /// * `cob_id` - The COB-ID to stop listening for
    pub fn unregister_pdo_handler(&mut self, cob_id: u16) {
        log::debug!("Unregistering PDO handler for COB-ID 0x{:03X}", cob_id);
        self.pdo_callbacks.remove(&cob_id);
    }

    /// Process an incoming PDO message
    /// 
    /// This should be called by the message processing loop when a PDO message
    /// is received (COB-ID in range 0x180-0x57F).
    /// 
    /// # Arguments
    /// * `message` - The received CAN message
    /// 
    /// # Returns
    /// * `Ok(true)` if a callback was invoked
    /// * `Ok(false)` if no callback was registered
    pub fn process_pdo(&mut self, message: &CanMessage) -> Result<bool> {
        let cob_id = message.id.raw();
        
        // Verify this is a PDO message (COB-ID range: 0x180-0x57F)
        if !Self::is_pdo_message(cob_id) {
            return Ok(false);
        }
        
        // Store the most recent PDO data
        self.recent_pdos.insert(cob_id, message.data.clone());
        
        // Invoke callback if registered
        if let Some(callback) = self.pdo_callbacks.get(&cob_id) {
            log::trace!("Processing PDO COB-ID 0x{:03X}, {} bytes", cob_id, message.data.len());
            callback(&message.data);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Check if a COB-ID is in the PDO range
    /// 
    /// PDO range: 0x180-0x57F (TPDO1-4 and RPDO1-4 for all nodes)
    pub fn is_pdo_message(cob_id: u16) -> bool {
        cob_id >= 0x180 && cob_id <= 0x57F
    }

    /// Get the most recent PDO data for a specific COB-ID
    /// 
    /// Returns None if no PDO has been received for this COB-ID
    pub fn get_recent_pdo(&self, cob_id: u16) -> Option<Vec<u8>> {
        self.recent_pdos.get(&cob_id).cloned()
    }

    /// Clear all stored recent PDO data
    pub fn clear_recent_pdos(&mut self) {
        self.recent_pdos.clear();
    }

    /// Get the number of registered PDO handlers
    pub fn handler_count(&self) -> usize {
        self.pdo_callbacks.len()
    }
}

impl Default for PdoManager {
    fn default() -> Self {
        Self::new()
    }
}