use log::{debug, warn};
/// EMCY (Emergency) Object Support
///
/// Emergency messages are used by CANopen devices to signal error conditions.
/// Each node has a unique EMCY COB-ID = 0x80 + Node-ID
///
/// EMCY message format (8 bytes):
/// - Bytes 0-1: Emergency Error Code (little-endian u16)
/// - Byte 2: Error Register
/// - Bytes 3-7: Manufacturer-specific error field (5 bytes)
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;

/// Emergency COB-ID base (0x80)
pub const EMCY_COB_ID_BASE: u16 = 0x80;

/// Type alias for emergency handler callback
pub type EmcyHandlerCallback = Arc<dyn Fn(&EmergencyMessage) + Send + Sync>;

/// Emergency message structure
#[derive(Debug, Clone, PartialEq)]
pub struct EmergencyMessage {
    /// Node ID that sent the emergency
    pub node_id: u8,

    /// Emergency error code (CANopen standard codes)
    pub error_code: u16,

    /// Error register (bit field)
    pub error_register: u8,

    /// Manufacturer-specific error data (5 bytes)
    pub manufacturer_data: [u8; 5],
}

impl EmergencyMessage {
    /// Parse an emergency message from CAN data
    ///
    /// # Arguments
    /// * `cob_id` - The COB-ID of the message (should be 0x80 + node_id)
    /// * `data` - The 8-byte emergency message data
    ///
    /// # Returns
    /// * `Some(EmergencyMessage)` if valid
    /// * `None` if invalid format
    pub fn from_can_data(cob_id: u16, data: &[u8]) -> Option<Self> {
        // EMCY COB-ID must be in range 0x81-0xFF (node 1-127)
        if !(0x81..=0xFF).contains(&cob_id) {
            return None;
        }

        // EMCY messages must be exactly 8 bytes
        if data.len() != 8 {
            warn!(
                "Invalid EMCY message length: {} bytes (expected 8)",
                data.len()
            );
            return None;
        }

        let node_id = (cob_id - EMCY_COB_ID_BASE) as u8;

        // Parse error code (little-endian u16)
        let error_code = u16::from_le_bytes([data[0], data[1]]);

        // Parse error register
        let error_register = data[2];

        // Parse manufacturer data (5 bytes)
        let mut manufacturer_data = [0u8; 5];
        manufacturer_data.copy_from_slice(&data[3..8]);

        Some(Self {
            node_id,
            error_code,
            error_register,
            manufacturer_data,
        })
    }

    /// Get a human-readable description of the error code
    pub fn error_code_description(&self) -> &'static str {
        match self.error_code {
            0x0000 => "No Error / Error Reset",
            0x1000 => "Generic Error",
            0x2000 => "Current - Generic",
            0x2010 => "Current - Input Side",
            0x2020 => "Current - Output Side",
            0x3000 => "Voltage - Generic",
            0x3010 => "Voltage - Mains Input",
            0x3020 => "Voltage - Inside Device",
            0x3030 => "Voltage - Output",
            0x4000 => "Temperature - Generic",
            0x4010 => "Temperature - Ambient",
            0x4020 => "Temperature - Device",
            0x5000 => "Device Hardware",
            0x6000 => "Device Software - Generic",
            0x6010 => "Internal Software",
            0x6020 => "User Software",
            0x6030 => "Data Set",
            0x7000 => "Additional Modules",
            0x8000 => "Monitoring - Generic",
            0x8100 => "Communication - Generic",
            0x8110 => "CAN Overrun (Objects Lost)",
            0x8120 => "CAN Passive Mode",
            0x8130 => "Life Guard Error / Heartbeat Error",
            0x8140 => "Bus Off Recovered",
            0x8150 => "Transmit COB-ID Collision",
            0x8200 => "Protocol Error - Generic",
            0x8210 => "PDO Length Error",
            0x8220 => "PDO Length Exceeded",
            0x8230 => "DAM MPDO Not Processed",
            0x8240 => "Unexpected SYNC Length",
            0x8250 => "RPDO Timeout",
            0x9000 => "External Error",
            0xF000 => "Additional Functions",
            0xFF00 => "Device Specific",
            _ => "Unknown Error Code",
        }
    }

    /// Check if this is an error reset message (error code 0x0000)
    pub fn is_error_reset(&self) -> bool {
        self.error_code == 0x0000
    }
}

/// Emergency message manager
pub struct EmcyManager {
    /// Registered emergency handlers (key = node_id)
    emcy_handlers: Arc<RwLock<HashMap<u8, EmcyHandlerCallback>>>,

    /// Recent emergency messages (key = node_id)
    recent_emcy: Arc<RwLock<HashMap<u8, EmergencyMessage>>>,
}

impl Default for EmcyManager {
    fn default() -> Self {
        Self::new()
    }
}

impl EmcyManager {
    /// Create a new emergency manager
    pub fn new() -> Self {
        Self {
            emcy_handlers: Arc::new(RwLock::new(HashMap::new())),
            recent_emcy: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register an emergency handler for a specific node
    ///
    /// # Arguments
    /// * `node_id` - The node ID to monitor (1-127)
    /// * `handler` - Callback function invoked when emergency is received
    ///
    /// # Example
    /// ```
    /// # use libcanopen_client::EmcyManager;
    /// let emcy_manager = EmcyManager::new();
    /// emcy_manager.register_emcy_handler(5, |emcy| {
    ///     println!("Emergency from node {}: Error 0x{:04X}",
    ///         emcy.node_id, emcy.error_code);
    /// });
    /// ```
    pub fn register_emcy_handler<F>(&self, node_id: u8, handler: F)
    where
        F: Fn(&EmergencyMessage) + Send + Sync + 'static,
    {
        if node_id == 0 || node_id > 127 {
            warn!("Invalid node ID for EMCY handler: {}", node_id);
            return;
        }

        let mut handlers = self.emcy_handlers.write().unwrap();
        handlers.insert(node_id, Arc::new(handler));
        debug!("Registered EMCY handler for node {}", node_id);
    }

    /// Unregister emergency handler for a node
    pub fn unregister_emcy_handler(&self, node_id: u8) {
        let mut handlers = self.emcy_handlers.write().unwrap();
        if handlers.remove(&node_id).is_some() {
            debug!("Unregistered EMCY handler for node {}", node_id);
        }
    }

    /// Process an incoming emergency message
    pub fn process_emcy(&self, cob_id: u16, data: &[u8]) {
        if let Some(emcy) = EmergencyMessage::from_can_data(cob_id, data) {
            debug!(
                "Emergency from node {}: code=0x{:04X} ({}), register=0x{:02X}",
                emcy.node_id,
                emcy.error_code,
                emcy.error_code_description(),
                emcy.error_register
            );

            // Store recent emergency
            let mut recent = self.recent_emcy.write().unwrap();
            recent.insert(emcy.node_id, emcy.clone());

            // Invoke handler if registered
            let handlers = self.emcy_handlers.read().unwrap();
            if let Some(handler) = handlers.get(&emcy.node_id) {
                handler(&emcy);
            }
        }
    }

    /// Get the most recent emergency message from a node
    pub fn get_recent_emcy(&self, node_id: u8) -> Option<EmergencyMessage> {
        let recent = self.recent_emcy.read().unwrap();
        recent.get(&node_id).cloned()
    }

    /// Clear all stored emergency messages
    pub fn clear_recent_emcy(&self) {
        let mut recent = self.recent_emcy.write().unwrap();
        recent.clear();
        debug!("Cleared recent EMCY messages");
    }

    /// Check if COB-ID is an emergency message (0x81-0xFF)
    pub fn is_emcy_message(cob_id: u16) -> bool {
        (0x81..=0xFF).contains(&cob_id)
    }
}

impl Clone for EmcyManager {
    fn clone(&self) -> Self {
        Self {
            emcy_handlers: Arc::clone(&self.emcy_handlers),
            recent_emcy: Arc::clone(&self.recent_emcy),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emcy_cob_id_base() {
        assert_eq!(EMCY_COB_ID_BASE, 0x80);
        assert_eq!(EMCY_COB_ID_BASE, 128); // Decimal value
    }

    #[test]
    fn test_emcy_parse() {
        let data = [
            0x10, 0x10, // Error code 0x1010 (little-endian)
            0x01, // Error register
            0xAA, 0xBB, 0xCC, 0xDD, 0xEE, // Manufacturer data
        ];

        let emcy = EmergencyMessage::from_can_data(0x85, &data).unwrap();
        assert_eq!(emcy.node_id, 5);
        assert_eq!(emcy.error_code, 0x1010);
        assert_eq!(emcy.error_register, 0x01);
        assert_eq!(emcy.manufacturer_data, [0xAA, 0xBB, 0xCC, 0xDD, 0xEE]);
    }

    #[test]
    fn test_emcy_all_valid_node_ids() {
        // Test EMCY from all valid node IDs (1-127)
        let data = [0x00, 0x10, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00];

        for node_id in 1..=127u8 {
            let cob_id = EMCY_COB_ID_BASE + node_id as u16;
            let emcy = EmergencyMessage::from_can_data(cob_id, &data).unwrap();
            assert_eq!(emcy.node_id, node_id);
        }
    }

    #[test]
    fn test_emcy_error_code_little_endian() {
        // Test little-endian byte order for error code
        let data = [0x34, 0x12, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let emcy = EmergencyMessage::from_can_data(0x81, &data).unwrap();
        assert_eq!(emcy.error_code, 0x1234); // Little-endian: 0x34, 0x12
    }

    #[test]
    fn test_emcy_invalid_length() {
        // EMCY messages must be exactly 8 bytes
        let too_short = [0x10, 0x10, 0x01];
        assert!(EmergencyMessage::from_can_data(0x85, &too_short).is_none());

        let too_long = [0x10, 0x10, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF];
        assert!(EmergencyMessage::from_can_data(0x85, &too_long).is_none());

        let empty = [];
        assert!(EmergencyMessage::from_can_data(0x85, &empty).is_none());
    }

    #[test]
    fn test_emcy_invalid_cob_id() {
        let data = [0x10, 0x10, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00];

        // COB-ID 0x80 is SYNC, not EMCY
        assert!(EmergencyMessage::from_can_data(0x80, &data).is_none());

        // COB-ID 0x100 is out of EMCY range
        assert!(EmergencyMessage::from_can_data(0x100, &data).is_none());

        // COB-ID 0x200 is PDO range
        assert!(EmergencyMessage::from_can_data(0x200, &data).is_none());

        // COB-ID 0x00 is NMT
        assert!(EmergencyMessage::from_can_data(0x00, &data).is_none());
    }

    #[test]
    fn test_emcy_error_reset() {
        // Error code 0x0000 indicates error reset / no error
        let data = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let emcy = EmergencyMessage::from_can_data(0x85, &data).unwrap();
        assert!(emcy.is_error_reset());
        assert_eq!(emcy.error_code, 0x0000);
    }

    #[test]
    fn test_emcy_not_error_reset() {
        let data = [0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let emcy = EmergencyMessage::from_can_data(0x85, &data).unwrap();
        assert!(!emcy.is_error_reset());

        let data = [0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let emcy = EmergencyMessage::from_can_data(0x85, &data).unwrap();
        assert!(!emcy.is_error_reset());
    }

    #[test]
    fn test_is_emcy_message() {
        // Not EMCY
        assert!(!EmcyManager::is_emcy_message(0x00)); // NMT
        assert!(!EmcyManager::is_emcy_message(0x80)); // SYNC
        assert!(!EmcyManager::is_emcy_message(0x180)); // PDO
        assert!(!EmcyManager::is_emcy_message(0x580)); // SDO
        assert!(!EmcyManager::is_emcy_message(0x700)); // Heartbeat
        assert!(!EmcyManager::is_emcy_message(0x7E5)); // LSS

        // Valid EMCY (0x81-0xFF)
        assert!(EmcyManager::is_emcy_message(0x81)); // Node 1
        assert!(EmcyManager::is_emcy_message(0x85)); // Node 5
        assert!(EmcyManager::is_emcy_message(0xFF)); // Node 127
        assert!(EmcyManager::is_emcy_message(0xC0)); // Node 64
    }

    #[test]
    fn test_emcy_error_code_descriptions() {
        // Test various standard error codes
        let test_cases = vec![
            (0x0000, "No Error / Error Reset"),
            (0x1000, "Generic Error"),
            (0x2000, "Current - Generic"),
            (0x3000, "Voltage - Generic"),
            (0x4000, "Temperature - Generic"),
            (0x5000, "Device Hardware"),
            (0x6000, "Device Software - Generic"),
            (0x8000, "Monitoring - Generic"),
            (0x9000, "External Error"),
        ];

        for (code, expected_desc) in test_cases {
            let data = [
                (code & 0xFF) as u8,
                ((code >> 8) & 0xFF) as u8,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
            ];
            let emcy = EmergencyMessage::from_can_data(0x81, &data).unwrap();
            assert_eq!(emcy.error_code, code);
            assert_eq!(emcy.error_code_description(), expected_desc);
        }
    }

    #[test]
    fn test_emcy_error_register_bits() {
        // Test different error register values
        let data = [0x00, 0x10, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00]; // Bit 0: Generic error
        let emcy = EmergencyMessage::from_can_data(0x81, &data).unwrap();
        assert_eq!(emcy.error_register, 0x01);

        let data = [0x00, 0x10, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00]; // All bits set
        let emcy = EmergencyMessage::from_can_data(0x81, &data).unwrap();
        assert_eq!(emcy.error_register, 0xFF);
    }

    #[test]
    fn test_emcy_manufacturer_data() {
        // Test various manufacturer data patterns
        let data = [0x00, 0x00, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55];
        let emcy = EmergencyMessage::from_can_data(0x81, &data).unwrap();
        assert_eq!(emcy.manufacturer_data, [0x11, 0x22, 0x33, 0x44, 0x55]);

        let data = [0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
        let emcy = EmergencyMessage::from_can_data(0x81, &data).unwrap();
        assert_eq!(emcy.manufacturer_data, [0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);

        let data = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let emcy = EmergencyMessage::from_can_data(0x81, &data).unwrap();
        assert_eq!(emcy.manufacturer_data, [0x00, 0x00, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn test_emcy_clone() {
        let data = [0x10, 0x20, 0x05, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE];
        let emcy1 = EmergencyMessage::from_can_data(0x85, &data).unwrap();
        let emcy2 = emcy1.clone();

        assert_eq!(emcy1.node_id, emcy2.node_id);
        assert_eq!(emcy1.error_code, emcy2.error_code);
        assert_eq!(emcy1.error_register, emcy2.error_register);
        assert_eq!(emcy1.manufacturer_data, emcy2.manufacturer_data);
    }

    #[test]
    fn test_emcy_equality() {
        let data = [0x10, 0x20, 0x05, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE];
        let emcy1 = EmergencyMessage::from_can_data(0x85, &data).unwrap();
        let emcy2 = EmergencyMessage::from_can_data(0x85, &data).unwrap();

        assert_eq!(emcy1, emcy2);

        // Different error code
        let data2 = [0x11, 0x20, 0x05, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE];
        let emcy3 = EmergencyMessage::from_can_data(0x85, &data2).unwrap();
        assert_ne!(emcy1, emcy3);
    }

    #[test]
    fn test_emcy_specific_error_codes() {
        // Test specific CANopen error codes
        let test_errors = vec![
            0x2000, // Current - General
            0x2010, // Current - Input side
            0x2020, // Current - Output side
            0x3000, // Voltage - General
            0x3010, // Voltage - Mains
            0x3020, // Voltage - Inside device
            0x4000, // Temperature - General
            0x5000, // Device Hardware
            0x6000, // Device Software
            0x8000, // Monitoring - General
            0x8100, // Communication
            0x8200, // Protocol Error
        ];

        for error_code in test_errors {
            let data = [
                (error_code & 0xFF) as u8,
                ((error_code >> 8) & 0xFF) as u8,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
            ];
            let emcy = EmergencyMessage::from_can_data(0x81, &data);
            assert!(emcy.is_some());
            assert_eq!(emcy.unwrap().error_code, error_code);
        }
    }

    #[test]
    fn test_emcy_cob_id_calculation() {
        // Verify COB-ID calculation: 0x80 + node_id
        for node_id in 1..=127u8 {
            let expected_cob_id = 0x80 + node_id as u16;
            let data = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
            let emcy = EmergencyMessage::from_can_data(expected_cob_id, &data).unwrap();
            assert_eq!(emcy.node_id, node_id);
            assert_eq!(expected_cob_id, EMCY_COB_ID_BASE + node_id as u16);
        }
    }

    #[test]
    fn test_emcy_boundary_values() {
        let data = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];

        // Node 1 (minimum)
        let emcy = EmergencyMessage::from_can_data(0x81, &data).unwrap();
        assert_eq!(emcy.node_id, 1);
        assert_eq!(emcy.error_code, 0xFFFF);
        assert_eq!(emcy.error_register, 0xFF);

        // Node 127 (maximum)
        let emcy = EmergencyMessage::from_can_data(0xFF, &data).unwrap();
        assert_eq!(emcy.node_id, 127);
        assert_eq!(emcy.error_code, 0xFFFF);
    }
}
