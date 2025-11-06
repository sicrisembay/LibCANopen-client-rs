// CAN message types and COB-ID definitions
use serde::{Deserialize, Serialize};

/// CAN identifier wrapper
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CanId(pub u16);

impl From<u16> for CanId {
    fn from(id: u16) -> Self {
        CanId(id)
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

/// CAN message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanMessage {
    pub id: CanId,
    pub data: Vec<u8>,
    pub timestamp: Option<u64>,
}

impl CanMessage {
    /// Create a new CAN message
    pub fn new(id: u16, data: Vec<u8>) -> Self {
        Self {
            id: CanId(id),
            data,
            timestamp: Some(crate::utils::get_timestamp_us()),
        }
    }

    /// Create a new CAN message with specific timestamp
    pub fn with_timestamp(id: u16, data: Vec<u8>, timestamp: u64) -> Self {
        Self {
            id: CanId(id),
            data,
            timestamp: Some(timestamp),
        }
    }

    /// Get the message type based on COB-ID
    pub fn message_type(&self) -> MessageType {
        MessageType::from_cob_id(self.id.0)
    }

    /// Get node ID for messages that contain it
    pub fn node_id(&self) -> Option<u8> {
        match self.message_type() {
            MessageType::NmtErrorControl => Some((self.id.0 & 0x7F) as u8),
            MessageType::Sdo => {
                if self.id.0 >= 0x580 && self.id.0 < 0x600 {
                    // SDO response
                    Some((self.id.0 - 0x580) as u8)
                } else if self.id.0 >= 0x600 && self.id.0 < 0x680 {
                    // SDO request
                    Some((self.id.0 - 0x600) as u8)
                } else {
                    None
                }
            }
            MessageType::Emergency => {
                if self.id.0 >= 0x081 && self.id.0 <= 0x0FF {
                    Some((self.id.0 - 0x080) as u8)
                } else {
                    None
                }
            }
            MessageType::Pdo => {
                // For PDOs, extract node ID from various ranges
                match self.id.0 {
                    0x180..=0x1FF => Some((self.id.0 - 0x180) as u8), // PDO1 TX
                    0x200..=0x27F => Some((self.id.0 - 0x200) as u8), // PDO1 RX
                    0x280..=0x2FF => Some((self.id.0 - 0x280) as u8), // PDO2 TX
                    0x300..=0x37F => Some((self.id.0 - 0x300) as u8), // PDO2 RX
                    0x380..=0x3FF => Some((self.id.0 - 0x380) as u8), // PDO3 TX
                    0x400..=0x47F => Some((self.id.0 - 0x400) as u8), // PDO3 RX
                    0x480..=0x4FF => Some((self.id.0 - 0x480) as u8), // PDO4 TX
                    0x500..=0x57F => Some((self.id.0 - 0x500) as u8), // PDO4 RX
                    _ => None,
                }
            }
            _ => None,
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
        let nmt_msg = CanMessage::new(0x000, vec![0x01, 0x00]);
        assert_eq!(nmt_msg.message_type(), MessageType::Nmt);

        // SDO request
        let sdo_msg = CanMessage::new(0x601, vec![0x40, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00]);
        assert_eq!(sdo_msg.message_type(), MessageType::Sdo);

        // PDO
        let pdo_msg = CanMessage::new(0x181, vec![0x01, 0x02, 0x03, 0x04]);
        assert_eq!(pdo_msg.message_type(), MessageType::Pdo);

        // Heartbeat
        let hb_msg = CanMessage::new(0x701, vec![0x05]);
        assert_eq!(hb_msg.message_type(), MessageType::NmtErrorControl);
    }

    #[test]
    fn test_node_id_extraction() {
        // SDO response
        let sdo_resp = CanMessage::new(0x581, vec![]);
        assert_eq!(sdo_resp.node_id(), Some(1));

        // Heartbeat
        let hb_msg = CanMessage::new(0x705, vec![0x05]);
        assert_eq!(hb_msg.node_id(), Some(5));

        // PDO1 TX
        let pdo_msg = CanMessage::new(0x183, vec![]);
        assert_eq!(pdo_msg.node_id(), Some(3));
    }
}