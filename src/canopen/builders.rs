// Message builders for constructing different CANopen message types

use crate::canopen::{CanMessage, ObjectId, SdoAbortCode};
use crate::canopen::message::{NmtCommand, EmergencyErrorCode};
use crate::{Result, CANopenError};

/// Builder for NMT command messages
pub struct NmtMessageBuilder;

impl NmtMessageBuilder {
    /// Create an NMT command message
    pub fn command(node_id: u8, command: NmtCommand) -> CanMessage {
        let data = vec![command as u8, node_id];
        CanMessage::new_unchecked(0x000, data) // NMT uses COB-ID 0x000
    }

    /// Create Node Guard request message
    pub fn node_guard_request(node_id: u8) -> CanMessage {
        CanMessage::with_timestamp(0x700 + node_id as u16, vec![], true).unwrap()
    }

    /// Create Heartbeat message
    pub fn heartbeat(node_id: u8, state: u8) -> CanMessage {
        let data = vec![state];
        CanMessage::new_unchecked(0x700 + node_id as u16, data)
    }
}

/// Builder for SDO client request messages
pub struct SdoClientMessageBuilder;

impl SdoClientMessageBuilder {
    /// Create SDO upload request (read from server)
    pub fn upload_request(server_node_id: u8, object_id: ObjectId) -> CanMessage {
        let mut data = vec![0x40]; // Upload request command
        data.extend_from_slice(&object_id.index.to_le_bytes());
        data.push(object_id.subindex);
        data.extend_from_slice(&[0x00; 4]); // Reserved bytes
        
        CanMessage::new_unchecked(0x600 + server_node_id as u16, data)
    }

    /// Create SDO download request (write to server) - expedited transfer
    pub fn download_request_expedited(
        server_node_id: u8, 
        object_id: ObjectId, 
        data: &[u8]
    ) -> Result<CanMessage> {
        if data.len() > 4 {
            return Err(CANopenError::InvalidData("Data too large for expedited transfer".to_string()));
        }

        let mut msg_data = vec![0x23]; // Download request, expedited
        
        // Modify command byte based on data length
        if data.len() < 4 {
            msg_data[0] |= ((4 - data.len()) as u8) << 2; // Size specified
            msg_data[0] |= 0x01; // Size indicated
        }
        
        msg_data.extend_from_slice(&object_id.index.to_le_bytes());
        msg_data.push(object_id.subindex);
        
        // Add data with padding to 4 bytes
        let mut padded_data = data.to_vec();
        padded_data.resize(4, 0);
        msg_data.extend_from_slice(&padded_data);
        
        Ok(CanMessage::new_unchecked(0x600 + server_node_id as u16, msg_data))
    }

    /// Create SDO download request (write to server) - segmented transfer initiate
    pub fn download_request_segmented_init(
        server_node_id: u8,
        object_id: ObjectId,
        data_size: u32
    ) -> CanMessage {
        let mut msg_data = vec![0x21]; // Download request, segmented, size indicated
        msg_data.extend_from_slice(&object_id.index.to_le_bytes());
        msg_data.push(object_id.subindex);
        msg_data.extend_from_slice(&data_size.to_le_bytes());
        
        CanMessage::new_unchecked(0x600 + server_node_id as u16, msg_data)
    }

    /// Create SDO download segment
    pub fn download_segment(
        server_node_id: u8,
        toggle: bool,
        last_segment: bool,
        segment_data: &[u8]
    ) -> Result<CanMessage> {
        if segment_data.len() > 7 {
            return Err(CANopenError::InvalidData("Segment data too large".to_string()));
        }

        let can_id = CanId::new_standard(0x600 + server_node_id as u32).unwrap();
        
        let mut command = 0x00; // Download segment command
        if toggle {
            command |= 0x10; // Toggle bit
        }
        if last_segment {
            command |= 0x01; // Last segment
            command |= ((7 - segment_data.len()) as u8) << 1; // Number of bytes that do not contain data
        }
        
        let mut msg_data = vec![command];
        let mut padded_data = segment_data.to_vec();
        padded_data.resize(7, 0);
        msg_data.extend_from_slice(&padded_data);
        
        Ok(CanMessage {
            id: can_id,
            data: msg_data,
            remote: false,
        })
    }

    /// Create SDO abort transfer message
    pub fn abort_transfer(server_node_id: u8, object_id: ObjectId, abort_code: SdoAbortCode) -> CanMessage {
        let can_id = CanId::new_standard(0x600 + server_node_id as u32).unwrap();
        
        let mut data = vec![0x80]; // Abort command
        data.extend_from_slice(&object_id.index.to_le_bytes());
        data.push(object_id.subindex);
        data.extend_from_slice(&abort_code.to_u32().to_le_bytes());
        
        CanMessage {
            id: can_id,
            data,
            remote: false,
        }
    }
}

/// Builder for SDO server response messages  
pub struct SdoServerMessageBuilder;

impl SdoServerMessageBuilder {
    /// Create SDO upload response (read response)
    pub fn upload_response_expedited(
        client_node_id: u8,
        object_id: ObjectId,
        data: &[u8]
    ) -> Result<CanMessage> {
        if data.len() > 4 {
            return Err(CANopenError::InvalidData("Data too large for expedited transfer".to_string()));
        }

        let can_id = CanId::new_standard(0x580 + client_node_id as u32).unwrap();
        
        let mut msg_data = vec![0x43]; // Upload response, expedited
        
        // Modify command byte based on data length
        if data.len() < 4 {
            msg_data[0] |= ((4 - data.len()) as u8) << 2; // Size specified
            msg_data[0] |= 0x01; // Size indicated
        }
        
        msg_data.extend_from_slice(&object_id.index.to_le_bytes());
        msg_data.push(object_id.subindex);
        
        // Add data with padding to 4 bytes
        let mut padded_data = data.to_vec();
        padded_data.resize(4, 0);
        msg_data.extend_from_slice(&padded_data);
        
        Ok(CanMessage {
            id: can_id,
            data: msg_data,
            remote: false,
        })
    }

    /// Create SDO download response (write confirmation)
    pub fn download_response(client_node_id: u8, object_id: ObjectId) -> CanMessage {
        let can_id = CanId::new_standard(0x580 + client_node_id as u32).unwrap();
        
        let mut data = vec![0x60]; // Download response
        data.extend_from_slice(&object_id.index.to_le_bytes());
        data.push(object_id.subindex);
        data.extend_from_slice(&[0x00; 4]); // Reserved
        
        CanMessage {
            id: can_id,
            data,
            remote: false,
        }
    }

    /// Create SDO abort transfer message
    pub fn abort_transfer(client_node_id: u8, object_id: ObjectId, abort_code: SdoAbortCode) -> CanMessage {
        let can_id = CanId::new_standard(0x580 + client_node_id as u32).unwrap();
        
        let mut data = vec![0x80]; // Abort command
        data.extend_from_slice(&object_id.index.to_le_bytes());
        data.push(object_id.subindex);
        data.extend_from_slice(&abort_code.to_u32().to_le_bytes());
        
        CanMessage {
            id: can_id,
            data,
            remote: false,
        }
    }
}

/// Builder for Emergency messages
pub struct EmergencyMessageBuilder;

impl EmergencyMessageBuilder {
    /// Create Emergency message
    pub fn emergency(
        node_id: u8,
        error_code: EmergencyErrorCode,
        error_register: u8,
        manufacturer_specific: &[u8; 5]
    ) -> CanMessage {
        let can_id = CanId::new_standard(0x080 + node_id as u32).unwrap();
        
        let mut data = Vec::with_capacity(8);
        data.extend_from_slice(&(error_code as u16).to_le_bytes());
        data.push(error_register);
        data.extend_from_slice(manufacturer_specific);
        
        CanMessage {
            id: can_id,
            data,
            remote: false,
        }
    }
}

/// Builder for PDO messages
pub struct PdoMessageBuilder;

impl PdoMessageBuilder {
    /// Create TPDO (Transmit PDO) message
    pub fn tpdo(node_id: u8, pdo_number: u8, data: Vec<u8>) -> Result<CanMessage> {
        if pdo_number > 3 {
            return Err(CANopenError::InvalidData("PDO number must be 0-3".to_string()));
        }
        if data.len() > 8 {
            return Err(CANopenError::InvalidData("PDO data too large".to_string()));
        }

        let base_cob_id = match pdo_number {
            0 => 0x180,
            1 => 0x280, 
            2 => 0x380,
            3 => 0x480,
            _ => unreachable!(),
        };
        
        let can_id = CanId::new_standard(base_cob_id + node_id as u32).unwrap();
        
        Ok(CanMessage {
            id: can_id,
            data,
            remote: false,
        })
    }

    /// Create RPDO (Receive PDO) message  
    pub fn rpdo(node_id: u8, pdo_number: u8, data: Vec<u8>) -> Result<CanMessage> {
        if pdo_number > 3 {
            return Err(CANopenError::InvalidData("PDO number must be 0-3".to_string()));
        }
        if data.len() > 8 {
            return Err(CANopenError::InvalidData("PDO data too large".to_string()));
        }

        let base_cob_id = match pdo_number {
            0 => 0x200,
            1 => 0x300,
            2 => 0x400, 
            3 => 0x500,
            _ => unreachable!(),
        };
        
        let can_id = CanId::new_standard(base_cob_id + node_id as u32).unwrap();
        
        Ok(CanMessage {
            id: can_id,
            data,
            remote: false,
        })
    }
}

/// Builder for SYNC messages
pub struct SyncMessageBuilder;

impl SyncMessageBuilder {
    /// Create SYNC message
    pub fn sync() -> CanMessage {
        let can_id = CanId::new_standard(0x080).unwrap(); // Standard SYNC COB-ID
        
        CanMessage {
            id: can_id,
            data: vec![],
            remote: false,
        }
    }

    /// Create SYNC message with counter
    pub fn sync_with_counter(counter: u8) -> CanMessage {
        let can_id = CanId::new_standard(0x080).unwrap();
        
        CanMessage {
            id: can_id,
            data: vec![counter],
            remote: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nmt_command_message() {
        let msg = NmtMessageBuilder::command(5, NmtCommand::StartRemoteNode);
        
        assert_eq!(msg.id.raw(), 0x000);
        assert_eq!(msg.data, vec![0x01, 0x05]);
        assert!(!msg.remote);
    }

    #[test]
    fn test_node_guard_request() {
        let msg = NmtMessageBuilder::node_guard_request(10);
        
        assert_eq!(msg.id.raw(), 0x70A); // 0x700 + 10
        assert!(msg.data.is_empty());
        assert!(msg.remote);
    }

    #[test]
    fn test_sdo_upload_request() {
        let msg = SdoClientMessageBuilder::upload_request(5, ObjectId::new(0x1000, 0x00));
        
        assert_eq!(msg.id.raw(), 0x605); // 0x600 + 5
        assert_eq!(msg.data.len(), 8);
        assert_eq!(msg.data[0], 0x40); // Upload request command
        assert_eq!(msg.data[1..3], [0x00, 0x10]); // Index 0x1000 in little endian
        assert_eq!(msg.data[3], 0x00); // Subindex
    }

    #[test]
    fn test_sdo_download_expedited() {
        let data = vec![0x12, 0x34];
        let msg = SdoClientMessageBuilder::download_request_expedited(
            3, 
            ObjectId::new(0x2000, 0x01), 
            &data
        ).unwrap();

        assert_eq!(msg.id.raw(), 0x603); // 0x600 + 3
        assert_eq!(msg.data.len(), 8);
        assert_eq!(msg.data[0] & 0x20, 0x20); // Expedited bit set
        assert_eq!(msg.data[1..3], [0x00, 0x20]); // Index 0x2000 in little endian
        assert_eq!(msg.data[3], 0x01); // Subindex
        assert_eq!(msg.data[4..6], [0x12, 0x34]); // Data
    }

    #[test]
    fn test_emergency_message() {
        let manufacturer_data = [0x01, 0x02, 0x03, 0x04, 0x05];
        let msg = EmergencyMessageBuilder::emergency(
            7,
            EmergencyErrorCode::Generic,
            0x80,
            &manufacturer_data
        );

        assert_eq!(msg.id.raw(), 0x087); // 0x080 + 7
        assert_eq!(msg.data.len(), 8);
        assert_eq!(msg.data[0..2], [0x00, 0x00]); // Generic error code
        assert_eq!(msg.data[2], 0x80); // Error register
        assert_eq!(msg.data[3..8], manufacturer_data);
    }

    #[test]
    fn test_pdo_message() {
        let data = vec![0x01, 0x02, 0x03, 0x04];
        let msg = PdoMessageBuilder::tpdo(12, 1, data.clone()).unwrap();

        assert_eq!(msg.id.raw(), 0x28C); // 0x280 + 12 (TPDO2)
        assert_eq!(msg.data, data);
        assert!(!msg.remote);
    }

    #[test]
    fn test_sync_message() {
        let msg = SyncMessageBuilder::sync();
        
        assert_eq!(msg.id.raw(), 0x080);
        assert!(msg.data.is_empty());
        assert!(!msg.remote);
        
        let msg_with_counter = SyncMessageBuilder::sync_with_counter(42);
        assert_eq!(msg_with_counter.data, vec![42]);
    }
}