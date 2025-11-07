// CAN message types and COB-ID definitions
use crate::{CANopenError, Result};
use serde::{Deserialize, Serialize};

/// CAN identifier wrapper with CANopen-specific functionality
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CanId(pub u16);

impl CanId {
    /// Create a new CAN ID with validation
    pub fn new(id: u16) -> Result<Self> {
        if id > 0x7FF {
            return Err(CANopenError::InvalidMessage);
        }
        Ok(CanId(id))
    }

    /// Create CAN ID without validation (for internal use)
    pub fn new_unchecked(id: u16) -> Self {
        CanId(id)
    }

    /// Create a standard 11-bit CAN ID
    pub fn new_standard(id: u32) -> Result<Self> {
        if id > 0x7FF {
            return Err(CANopenError::InvalidMessage);
        }
        Ok(CanId(id as u16))
    }

    /// Get the raw CAN ID value
    pub fn raw(&self) -> u16 {
        self.0
    }

    /// Check if this is a valid 11-bit CAN ID
    pub fn is_valid(&self) -> bool {
        self.0 <= 0x7FF
    }

    /// Get the message type based on COB-ID
    pub fn message_type(&self) -> MessageType {
        MessageType::from_cob_id(self.0)
    }

    /// Extract node ID from COB-ID if applicable
    pub fn node_id(&self) -> Option<u8> {
        match self.message_type() {
            MessageType::NmtErrorControl => Some((self.0 & 0x7F) as u8),
            MessageType::Sdo => {
                if self.0 >= 0x580 && self.0 < 0x600 {
                    // SDO response
                    Some((self.0 - 0x580) as u8)
                } else if self.0 >= 0x600 && self.0 < 0x680 {
                    // SDO request
                    Some((self.0 - 0x600) as u8)
                } else {
                    None
                }
            }
            MessageType::Emergency => {
                if self.0 >= 0x081 && self.0 <= 0x0FF {
                    Some((self.0 - 0x080) as u8)
                } else {
                    None
                }
            }
            MessageType::Pdo => {
                // Extract node ID from various PDO ranges
                match self.0 {
                    0x180..=0x1FF => Some((self.0 - 0x180) as u8), // PDO1 TX
                    0x200..=0x27F => Some((self.0 - 0x200) as u8), // PDO1 RX
                    0x280..=0x2FF => Some((self.0 - 0x280) as u8), // PDO2 TX
                    0x300..=0x37F => Some((self.0 - 0x300) as u8), // PDO2 RX
                    0x380..=0x3FF => Some((self.0 - 0x380) as u8), // PDO3 TX
                    0x400..=0x47F => Some((self.0 - 0x400) as u8), // PDO3 RX
                    0x480..=0x4FF => Some((self.0 - 0x480) as u8), // PDO4 TX
                    0x500..=0x57F => Some((self.0 - 0x500) as u8), // PDO4 RX
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Get the PDO number (1-4) if this is a PDO COB-ID
    pub fn pdo_number(&self) -> Option<u8> {
        match self.0 {
            0x180..=0x27F => Some(1), // PDO1
            0x280..=0x37F => Some(2), // PDO2
            0x380..=0x47F => Some(3), // PDO3
            0x480..=0x57F => Some(4), // PDO4
            _ => None,
        }
    }

    /// Check if this is a PDO transmission COB-ID
    pub fn is_pdo_tx(&self) -> bool {
        matches!(self.0, 0x180..=0x1FF | 0x280..=0x2FF | 0x380..=0x3FF | 0x480..=0x4FF)
    }

    /// Check if this is a PDO reception COB-ID
    pub fn is_pdo_rx(&self) -> bool {
        matches!(self.0, 0x200..=0x27F | 0x300..=0x37F | 0x400..=0x47F | 0x500..=0x57F)
    }
}

impl From<u16> for CanId {
    fn from(id: u16) -> Self {
        CanId::new_unchecked(id)
    }
}

impl From<CanId> for u16 {
    fn from(id: CanId) -> Self {
        id.0
    }
}

impl std::fmt::Display for CanId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "0x{:03X}", self.0)
    }
}

impl TryFrom<u32> for CanId {
    type Error = CANopenError;

    fn try_from(id: u32) -> Result<Self> {
        if id > 0x7FF {
            return Err(CANopenError::InvalidMessage);
        }
        Ok(CanId(id as u16))
    }
}

/// Represents a CAN message with timestamp
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanMessage {
    pub id: CanId,
    pub data: Vec<u8>,
    pub remote: bool,
    pub timestamp: std::time::SystemTime,
}

impl CanMessage {
    /// Create a new CAN message with validation
    pub fn new(id: u16, data: Vec<u8>) -> Result<Self> {
        let can_id = CanId::new(id)?;
        Self::validate_data_length(&data)?;

        Ok(Self {
            id: can_id,
            data,
            remote: false,
            timestamp: std::time::SystemTime::now(),
        })
    }

    /// Create a new CAN message without validation (for internal use)
    pub fn new_unchecked(id: u16, data: Vec<u8>) -> Self {
        Self {
            id: CanId::new_unchecked(id),
            data,
            remote: false,
            timestamp: std::time::SystemTime::now(),
        }
    }

    /// Create a new CAN message with specific timestamp
    pub fn with_timestamp(id: u16, data: Vec<u8>, remote: bool) -> Result<Self> {
        let can_id = CanId::new(id)?;
        if !remote {
            Self::validate_data_length(&data)?;
        }

        Ok(Self {
            id: can_id,
            data,
            remote,
            timestamp: std::time::SystemTime::now(),
        })
    }

    /// Create from raw components with validation
    pub fn from_raw(id: u16, data: &[u8]) -> Result<Self> {
        let can_id = CanId::new(id)?;
        let data_vec = data.to_vec();
        Self::validate_data_length(&data_vec)?;

        Ok(Self {
            id: can_id,
            data: data_vec,
            remote: false,
            timestamp: std::time::SystemTime::now(),
        })
    }

    /// Validate CAN data length (0-8 bytes for standard CAN)
    fn validate_data_length(data: &[u8]) -> Result<()> {
        if data.len() > 8 {
            return Err(CANopenError::InvalidMessage);
        }
        Ok(())
    }

    /// Get the message type based on COB-ID
    pub fn message_type(&self) -> MessageType {
        self.id.message_type()
    }

    /// Get node ID for messages that contain it
    pub fn node_id(&self) -> Option<u8> {
        self.id.node_id()
    }

    /// Check if this message is valid according to CANopen specifications
    pub fn is_valid(&self) -> bool {
        self.id.is_valid() && self.data.len() <= 8 && self.validate_canopen_format()
    }

    /// Check if this is a remote transmission request
    pub fn is_remote_request(&self) -> bool {
        self.remote
    }

    /// Validate CANopen-specific message format
    fn validate_canopen_format(&self) -> bool {
        match self.message_type() {
            MessageType::Nmt => {
                // NMT messages should have exactly 2 bytes
                self.data.len() == 2
            }
            MessageType::Sync => {
                // SYNC messages should have 0 or 1 bytes (optional counter)
                self.data.len() <= 1
            }
            MessageType::TimeStamp => {
                // TIME messages should have 6 bytes
                self.data.len() == 6
            }
            MessageType::Emergency => {
                // Emergency messages should have exactly 8 bytes
                self.data.len() == 8
            }
            MessageType::Sdo => {
                // SDO messages should have exactly 8 bytes
                self.data.len() == 8
            }
            MessageType::NmtErrorControl => {
                // Heartbeat messages should have exactly 1 byte
                self.data.len() == 1
            }
            MessageType::Pdo => {
                // PDO messages can have 0-8 bytes
                self.data.len() <= 8
            }
            MessageType::Lss => {
                // LSS messages should have exactly 8 bytes
                self.data.len() == 8
            }
            MessageType::Unknown => {
                // Unknown messages - just check general CAN constraints
                true
            }
        }
    }

    /// Get data as specific type with bounds checking
    pub fn get_u8(&self, index: usize) -> Option<u8> {
        self.data.get(index).copied()
    }

    /// Get data as u16 (little endian) starting at index
    pub fn get_u16_le(&self, index: usize) -> Option<u16> {
        if index + 1 < self.data.len() {
            Some(u16::from_le_bytes([self.data[index], self.data[index + 1]]))
        } else {
            None
        }
    }

    /// Get data as u32 (little endian) starting at index
    pub fn get_u32_le(&self, index: usize) -> Option<u32> {
        if index + 3 < self.data.len() {
            Some(u32::from_le_bytes([
                self.data[index],
                self.data[index + 1],
                self.data[index + 2],
                self.data[index + 3],
            ]))
        } else {
            None
        }
    }

    /// Get the function code for SDO messages
    pub fn sdo_command(&self) -> Option<u8> {
        if self.message_type() == MessageType::Sdo && !self.data.is_empty() {
            Some(self.data[0])
        } else {
            None
        }
    }

    /// Get the NMT command
    pub fn nmt_command(&self) -> Option<NmtCommand> {
        if self.message_type() == MessageType::Nmt && self.data.len() >= 2 {
            NmtCommand::from_u8(self.data[0])
        } else {
            None
        }
    }

    /// Get the NMT state for heartbeat messages
    pub fn nmt_state(&self) -> Option<crate::canopen::nmt::NmtState> {
        if self.message_type() == MessageType::NmtErrorControl && !self.data.is_empty() {
            Some(crate::canopen::nmt::NmtState::from(self.data[0]))
        } else {
            None
        }
    }
}

/// COB-ID ranges for different message types
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

/// CANopen message types
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

/// NMT command codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NmtCommand {
    StartRemoteNode = 0x01,
    StopRemoteNode = 0x02,
    EnterPreOperational = 0x80,
    ResetNode = 0x81,
    ResetCommunication = 0x82,
}

impl NmtCommand {
    /// Convert from raw command byte
    pub fn from_u8(cmd: u8) -> Option<Self> {
        match cmd {
            0x01 => Some(NmtCommand::StartRemoteNode),
            0x02 => Some(NmtCommand::StopRemoteNode),
            0x80 => Some(NmtCommand::EnterPreOperational),
            0x81 => Some(NmtCommand::ResetNode),
            0x82 => Some(NmtCommand::ResetCommunication),
            _ => None,
        }
    }

    /// Convert to raw command byte
    pub fn to_u8(self) -> u8 {
        self as u8
    }
}

/// SDO client command specifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SdoClientCommand {
    DownloadSegment = 0x00,
    DownloadInitiate = 0x20,
    UploadInitiate = 0x40,
    UploadSegment = 0x60,
    AbortTransfer = 0x80,
}

/// SDO server command specifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)]
pub enum SdoServerCommand {
    UploadSegmentResponse = 0x00,
    DownloadSegmentResponse = 0x20,
    UploadInitiateResponse = 0x40,
    DownloadInitiateResponse = 0x60,
    AbortTransferResponse = 0x80,
}

/// Emergency error codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmergencyErrorCode {
    NoError = 0x0000,
    Generic = 0x0001, // Generic error
    GenericError = 0x1000,
    Current = 0x2000,
    CurrentInput = 0x2010,
    CurrentOutput = 0x2020,
    Voltage = 0x3000,
    VoltageMainsInput = 0x3010,
    VoltageInsideDevice = 0x3020,
    VoltageOutput = 0x3030,
    Temperature = 0x4000,
    TemperatureAmbient = 0x4010,
    TemperatureDevice = 0x4020,
    DeviceHardware = 0x5000,
    DeviceSoftware = 0x6000,
    InternalSoftware = 0x6010,
    UserSoftware = 0x6020,
    DataSet = 0x6030,
    AdditionalModules = 0x7000,
    Monitoring = 0x8000,
    Communication = 0x8100,
    ProtocolError = 0x8200,
    ExternalError = 0x9000,
    AdditionalFunctions = 0xF000,
    DeviceSpecific = 0xFF00,
}

impl MessageType {
    /// Determine message type from COB-ID
    pub fn from_cob_id(cob_id: u16) -> Self {
        match cob_id {
            cob_ids::NMT_COMMAND => MessageType::Nmt,
            cob_ids::SYNC => MessageType::Sync,
            cob_ids::TIME_STAMP => MessageType::TimeStamp,
            0x081..=0x0FF => MessageType::Emergency,
            0x180..=0x57F => MessageType::Pdo,
            0x580..=0x67F => MessageType::Sdo,
            0x700..=0x77F => MessageType::NmtErrorControl,
            0x7E4..=0x7E5 => MessageType::Lss,
            _ => MessageType::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_type_detection() {
        // NMT command
        let nmt_msg = CanMessage::new(0x000, vec![0x01, 0x00]).unwrap();
        assert_eq!(nmt_msg.message_type(), MessageType::Nmt);

        // SDO request
        let sdo_msg =
            CanMessage::new(0x601, vec![0x40, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00]).unwrap();
        assert_eq!(sdo_msg.message_type(), MessageType::Sdo);

        // PDO
        let pdo_msg = CanMessage::new(0x181, vec![0x01, 0x02, 0x03, 0x04]).unwrap();
        assert_eq!(pdo_msg.message_type(), MessageType::Pdo);

        // Heartbeat
        let hb_msg = CanMessage::new(0x701, vec![0x05]).unwrap();
        assert_eq!(hb_msg.message_type(), MessageType::NmtErrorControl);
    }

    #[test]
    fn test_node_id_extraction() {
        // SDO response
        let sdo_resp = CanMessage::new(0x581, vec![]).unwrap();
        assert_eq!(sdo_resp.node_id(), Some(1));

        // Heartbeat
        let hb_msg = CanMessage::new(0x705, vec![0x05]).unwrap();
        assert_eq!(hb_msg.node_id(), Some(5));

        // PDO1 TX
        let pdo_msg = CanMessage::new(0x183, vec![]).unwrap();
        assert_eq!(pdo_msg.node_id(), Some(3));
    }

    #[test]
    fn test_message_validation() {
        // Valid message
        assert!(CanMessage::new(0x181, vec![0x01, 0x02]).is_ok());

        // Invalid CAN ID (too large)
        assert!(CanMessage::new(0x800, vec![0x01]).is_err());

        // Invalid data length (too long)
        assert!(CanMessage::new(0x181, vec![0; 9]).is_err());

        // Test CANopen format validation
        let sync_msg = CanMessage::new(0x080, vec![]).unwrap();
        assert!(sync_msg.is_valid());

        // SYNC with counter is valid
        let sync_with_counter = CanMessage::new_unchecked(0x080, vec![0x01]);
        assert!(sync_with_counter.is_valid());

        // SYNC with more than 1 byte is invalid
        let invalid_sync = CanMessage::new_unchecked(0x080, vec![0x01, 0x02]);
        assert!(!invalid_sync.is_valid());
    }

    #[test]
    fn test_can_id_functionality() {
        let can_id = CanId::new(0x183).unwrap();

        assert_eq!(can_id.node_id(), Some(3));
        assert_eq!(can_id.pdo_number(), Some(1));
        assert!(can_id.is_pdo_tx());
        assert!(!can_id.is_pdo_rx());
        assert_eq!(can_id.message_type(), MessageType::Pdo);
    }

    #[test]
    fn test_nmt_commands() {
        assert_eq!(NmtCommand::from_u8(0x01), Some(NmtCommand::StartRemoteNode));
        assert_eq!(NmtCommand::from_u8(0x99), None);
        assert_eq!(NmtCommand::StartRemoteNode.to_u8(), 0x01);
    }

    #[test]
    fn test_data_extraction() {
        let msg = CanMessage::new(0x181, vec![0x12, 0x34, 0x56, 0x78]).unwrap();

        assert_eq!(msg.get_u8(0), Some(0x12));
        assert_eq!(msg.get_u8(4), None);
        assert_eq!(msg.get_u16_le(0), Some(0x3412));
        assert_eq!(msg.get_u32_le(0), Some(0x78563412));
    }

    #[test]
    fn test_cob_id_ranges() {
        // Test all COB-ID ranges
        assert_eq!(MessageType::from_cob_id(0x000), MessageType::Nmt);
        assert_eq!(MessageType::from_cob_id(0x080), MessageType::Sync);
        assert_eq!(MessageType::from_cob_id(0x081), MessageType::Emergency);
        assert_eq!(MessageType::from_cob_id(0x0FF), MessageType::Emergency);
        assert_eq!(MessageType::from_cob_id(0x181), MessageType::Pdo);
        assert_eq!(MessageType::from_cob_id(0x281), MessageType::Pdo);
        assert_eq!(MessageType::from_cob_id(0x381), MessageType::Pdo);
        assert_eq!(MessageType::from_cob_id(0x481), MessageType::Pdo);
        assert_eq!(MessageType::from_cob_id(0x581), MessageType::Sdo);
        assert_eq!(MessageType::from_cob_id(0x601), MessageType::Sdo);
        assert_eq!(
            MessageType::from_cob_id(0x701),
            MessageType::NmtErrorControl
        );
        assert_eq!(MessageType::from_cob_id(0x7E4), MessageType::Lss);
        assert_eq!(MessageType::from_cob_id(0x7E5), MessageType::Lss);
    }

    #[test]
    fn test_pdo_cob_id_parsing() {
        // PDO1 TX (0x180 + node_id)
        let pdo = CanId::new(0x181).unwrap();
        assert_eq!(pdo.pdo_number(), Some(1));
        assert_eq!(pdo.node_id(), Some(1));
        assert!(pdo.is_pdo_tx());
        assert!(!pdo.is_pdo_rx());

        // PDO1 RX (0x200 + node_id)
        let pdo = CanId::new(0x205).unwrap();
        assert_eq!(pdo.pdo_number(), Some(1));
        assert_eq!(pdo.node_id(), Some(5));
        assert!(!pdo.is_pdo_tx());
        assert!(pdo.is_pdo_rx());

        // PDO2 TX (0x280 + node_id)
        let pdo = CanId::new(0x28A).unwrap();
        assert_eq!(pdo.pdo_number(), Some(2));
        assert_eq!(pdo.node_id(), Some(10));
        assert!(pdo.is_pdo_tx());

        // PDO3 TX (0x380 + node_id)
        let pdo = CanId::new(0x390).unwrap();
        assert_eq!(pdo.pdo_number(), Some(3));
        assert_eq!(pdo.node_id(), Some(16));

        // PDO4 RX (0x500 + node_id)
        let pdo = CanId::new(0x510).unwrap();
        assert_eq!(pdo.pdo_number(), Some(4));
        assert_eq!(pdo.node_id(), Some(16));
        assert!(pdo.is_pdo_rx());
    }

    #[test]
    fn test_sdo_cob_id_parsing() {
        // SDO TX (0x580 + node_id) - server to client
        let sdo = CanId::new(0x581).unwrap();
        assert_eq!(sdo.message_type(), MessageType::Sdo);
        assert_eq!(sdo.node_id(), Some(1));

        // SDO RX (0x600 + node_id) - client to server
        let sdo = CanId::new(0x610).unwrap();
        assert_eq!(sdo.message_type(), MessageType::Sdo);
        assert_eq!(sdo.node_id(), Some(16));

        // Boundary test
        let sdo = CanId::new(0x5FF).unwrap();
        assert_eq!(sdo.message_type(), MessageType::Sdo);
        assert_eq!(sdo.node_id(), Some(127));
    }

    #[test]
    fn test_emergency_cob_id_parsing() {
        // Emergency base is 0x80, node IDs 1-127
        let emcy = CanId::new(0x081).unwrap();
        assert_eq!(emcy.message_type(), MessageType::Emergency);
        assert_eq!(emcy.node_id(), Some(1));

        let emcy = CanId::new(0x0FF).unwrap();
        assert_eq!(emcy.message_type(), MessageType::Emergency);
        assert_eq!(emcy.node_id(), Some(127));
    }

    #[test]
    fn test_nmt_error_control_parsing() {
        // Heartbeat/Node Guarding (0x700 + node_id)
        let nmt = CanId::new(0x701).unwrap();
        assert_eq!(nmt.message_type(), MessageType::NmtErrorControl);
        assert_eq!(nmt.node_id(), Some(1));

        let nmt = CanId::new(0x77F).unwrap();
        assert_eq!(nmt.message_type(), MessageType::NmtErrorControl);
        assert_eq!(nmt.node_id(), Some(127));
    }

    #[test]
    fn test_lss_cob_ids() {
        // LSS Master TX: 0x7E5, Slave TX: 0x7E4
        let lss_master = CanId::new(0x7E5).unwrap();
        assert_eq!(lss_master.message_type(), MessageType::Lss);

        let lss_slave = CanId::new(0x7E4).unwrap();
        assert_eq!(lss_slave.message_type(), MessageType::Lss);
    }

    #[test]
    fn test_sync_message() {
        // SYNC has COB-ID 0x080
        let sync = CanMessage::new(0x080, vec![]).unwrap();
        assert_eq!(sync.message_type(), MessageType::Sync);
        assert!(sync.is_valid()); // Valid SYNC with no data

        // SYNC with counter
        let sync_counter = CanMessage::new(0x080, vec![0x01]).unwrap();
        assert_eq!(sync_counter.message_type(), MessageType::Sync);
        assert!(sync_counter.is_valid()); // Valid SYNC with 1-byte counter
    }

    #[test]
    fn test_nmt_command_message() {
        // NMT command has COB-ID 0x000
        let nmt = CanMessage::new(0x000, vec![0x01, 0x05]).unwrap();
        assert_eq!(nmt.message_type(), MessageType::Nmt);
        assert!(nmt.is_valid());

        // NMT command should have 2 bytes
        let nmt_invalid = CanMessage::new_unchecked(0x000, vec![0x01]);
        assert!(!nmt_invalid.is_valid());
    }

    #[test]
    fn test_can_id_validation() {
        // Valid 11-bit IDs
        assert!(CanId::new(0x000).is_ok());
        assert!(CanId::new(0x7FF).is_ok());

        // Invalid ID (exceeds 11-bit)
        assert!(CanId::new(0x800).is_err());
        assert!(CanId::new(0xFFF).is_err());

        // Test is_valid method
        let valid = CanId::new_unchecked(0x181);
        assert!(valid.is_valid());

        let invalid = CanId::new_unchecked(0x800);
        assert!(!invalid.is_valid());
    }

    #[test]
    fn test_message_data_length() {
        // Valid data lengths (0-8 bytes)
        assert!(CanMessage::new(0x181, vec![]).is_ok());
        assert!(CanMessage::new(0x181, vec![0x01]).is_ok());
        assert!(CanMessage::new(0x181, vec![0; 8]).is_ok());

        // Invalid data length (>8 bytes)
        assert!(CanMessage::new(0x181, vec![0; 9]).is_err());
        assert!(CanMessage::new(0x181, vec![0; 100]).is_err());
    }

    #[test]
    fn test_nmt_command_conversion() {
        // Test all NMT commands
        assert_eq!(NmtCommand::from_u8(0x01), Some(NmtCommand::StartRemoteNode));
        assert_eq!(NmtCommand::from_u8(0x02), Some(NmtCommand::StopRemoteNode));
        assert_eq!(
            NmtCommand::from_u8(0x80),
            Some(NmtCommand::EnterPreOperational)
        );
        assert_eq!(NmtCommand::from_u8(0x81), Some(NmtCommand::ResetNode));
        assert_eq!(
            NmtCommand::from_u8(0x82),
            Some(NmtCommand::ResetCommunication)
        );

        // Invalid command
        assert_eq!(NmtCommand::from_u8(0xFF), None);

        // Round-trip conversion
        assert_eq!(NmtCommand::StartRemoteNode.to_u8(), 0x01);
        assert_eq!(NmtCommand::StopRemoteNode.to_u8(), 0x02);
        assert_eq!(NmtCommand::EnterPreOperational.to_u8(), 0x80);
        assert_eq!(NmtCommand::ResetNode.to_u8(), 0x81);
        assert_eq!(NmtCommand::ResetCommunication.to_u8(), 0x82);
    }

    #[test]
    fn test_data_extraction_edge_cases() {
        let msg =
            CanMessage::new(0x181, vec![0xFF, 0x00, 0x80, 0x7F, 0x01, 0x02, 0x03, 0x04]).unwrap();

        // u8 extraction
        assert_eq!(msg.get_u8(0), Some(0xFF));
        assert_eq!(msg.get_u8(7), Some(0x04));
        assert_eq!(msg.get_u8(8), None); // Out of bounds

        // u16 little-endian extraction
        assert_eq!(msg.get_u16_le(0), Some(0x00FF)); // 0xFF, 0x00
        assert_eq!(msg.get_u16_le(2), Some(0x7F80)); // 0x80, 0x7F
        assert_eq!(msg.get_u16_le(7), None); // Not enough bytes

        // u32 little-endian extraction
        assert_eq!(msg.get_u32_le(0), Some(0x7F8000FF)); // 0xFF, 0x00, 0x80, 0x7F
        assert_eq!(msg.get_u32_le(4), Some(0x04030201)); // 0x01, 0x02, 0x03, 0x04
        assert_eq!(msg.get_u32_le(5), None); // Not enough bytes
    }

    #[test]
    fn test_message_cloning() {
        let msg1 = CanMessage::new(0x181, vec![0x01, 0x02, 0x03]).unwrap();
        let msg2 = msg1.clone();

        assert_eq!(msg1.id, msg2.id);
        assert_eq!(msg1.data, msg2.data);
    }

    #[test]
    fn test_can_id_raw_value() {
        let id = CanId::new(0x183).unwrap();
        assert_eq!(id.raw(), 0x183);

        let id2 = CanId::new_unchecked(0x7FF);
        assert_eq!(id2.raw(), 0x7FF);
    }
}
