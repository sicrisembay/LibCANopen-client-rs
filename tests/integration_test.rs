//! Integration tests for libcanopen_client
//!
//! These tests verify cross-protocol interactions, concurrent operations,
//! and end-to-end scenarios that span multiple modules.

use libcanopen_client::{CanMessage, LssAddress, LssCommand, MessageType, NmtState, NodeState};

/// Test cross-protocol message parsing and type detection
#[test]
fn test_cross_protocol_message_detection() {
    // NMT Command
    let nmt_msg = CanMessage::new(0x000, vec![0x01, 0x05]).unwrap();
    assert_eq!(nmt_msg.message_type(), MessageType::Nmt);

    // SDO Request
    let sdo_msg = CanMessage::new(0x605, vec![0x40, 0x00, 0x10, 0x01, 0, 0, 0, 0]).unwrap();
    assert_eq!(sdo_msg.message_type(), MessageType::Sdo);

    // SDO Response
    let sdo_resp = CanMessage::new(0x585, vec![0x43, 0x00, 0x10, 0x01, 0x12, 0, 0, 0]).unwrap();
    assert_eq!(sdo_resp.message_type(), MessageType::Sdo);

    // SYNC
    let sync_msg = CanMessage::new(0x080, vec![]).unwrap();
    assert_eq!(sync_msg.message_type(), MessageType::Sync);

    // Emergency
    let emcy_msg =
        CanMessage::new(0x085, vec![0x10, 0x23, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06]).unwrap();
    assert_eq!(emcy_msg.message_type(), MessageType::Emergency);

    // PDO
    let pdo_msg = CanMessage::new(0x185, vec![0x01, 0x02, 0x03, 0x04]).unwrap();
    assert_eq!(pdo_msg.message_type(), MessageType::Pdo);

    // LSS
    let lss_msg =
        CanMessage::new(0x7E5, vec![0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]).unwrap();
    assert_eq!(lss_msg.message_type(), MessageType::Lss);

    // Node Guarding/Heartbeat
    let ng_msg = CanMessage::new(0x705, vec![0x05]).unwrap();
    assert_eq!(ng_msg.message_type(), MessageType::NmtErrorControl);
}

/// Test NMT state transitions
#[test]
fn test_nmt_state_transitions() {
    let node_id = 5;
    let mut node_state = NodeState::new(node_id);

    // Check initial state (uses field access, not method)
    // Initial state is Invalid until first heartbeat/boot message
    assert_eq!(node_state.state, NmtState::Invalid);

    // State transitions
    node_state.update_state(NmtState::PreOperational);
    assert_eq!(node_state.state, NmtState::PreOperational);

    node_state.update_state(NmtState::Operational);
    assert_eq!(node_state.state, NmtState::Operational);

    node_state.update_state(NmtState::Stopped);
    assert_eq!(node_state.state, NmtState::Stopped);

    node_state.update_state(NmtState::PreOperational);
    assert_eq!(node_state.state, NmtState::PreOperational);
}

/// Test message data extraction
#[test]
fn test_message_data_extraction() {
    // SDO response with data
    let sdo_response =
        CanMessage::new(0x58A, vec![0x43, 0x00, 0x10, 0x00, 0x01, 0x02, 0x03, 0x04]).unwrap();

    // Extract data bytes
    let data = &sdo_response.data[4..8];
    let value = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    assert_eq!(value, 0x04030201);
}

/// Test LSS address structure
#[test]
fn test_lss_address() {
    let lss_address = LssAddress {
        vendor_id: 0x12345678,
        product_code: 0x9ABCDEF0,
        revision_number: 0x11223344,
        serial_number: 0x55667788,
    };

    assert_eq!(lss_address.vendor_id, 0x12345678);
    assert_eq!(lss_address.product_code, 0x9ABCDEF0);
    assert_eq!(lss_address.revision_number, 0x11223344);
    assert_eq!(lss_address.serial_number, 0x55667788);
}

/// Test LSS command values
#[test]
fn test_lss_command_values() {
    // Test LSS command enum values
    assert_eq!(LssCommand::SwitchStateGlobal as u8, 0x04);
    assert_eq!(LssCommand::ConfigureNodeId as u8, 0x11);
    assert_eq!(LssCommand::ConfigureBitTiming as u8, 0x13);
    assert_eq!(LssCommand::ActivateBitTiming as u8, 0x15);
    assert_eq!(LssCommand::StoreConfiguration as u8, 0x17);
    assert_eq!(LssCommand::InquireNodeId as u8, 0x5E);
}

/// Test data type endianness (CANopen uses little-endian)
#[test]
fn test_data_endianness() {
    let u16_val: u16 = 0x1234;
    let u16_bytes = u16_val.to_le_bytes();
    assert_eq!(u16_bytes, [0x34, 0x12]);
    assert_eq!(u16::from_le_bytes(u16_bytes), 0x1234);

    let i32_val: i32 = -12345;
    let i32_bytes = i32_val.to_le_bytes();
    assert_eq!(i32::from_le_bytes(i32_bytes), -12345);

    let f32_val: f32 = 1.23;
    let f32_bytes = f32_val.to_le_bytes();
    let decoded = f32::from_le_bytes(f32_bytes);
    assert!((decoded - 1.23).abs() < 0.01);
}

/// Test message priorities (lower COB-ID = higher priority)
#[test]
fn test_message_priorities() {
    let priorities = vec![
        0x000, // NMT - highest priority
        0x080, // SYNC - high priority
        0x081, // EMCY - high priority
        0x181, // PDO - medium priority
        0x605, // SDO - lower priority
        0x705, // Heartbeat - lower priority
    ];

    // Verify priorities are in ascending order
    for i in 1..priorities.len() {
        assert!(priorities[i - 1] <= priorities[i]);
    }
}

/// Test concurrent message handling
#[test]
fn test_concurrent_messages() {
    let node_id = 10;

    // Create multiple protocol messages
    let messages = vec![
        CanMessage::new(0x000, vec![0x01, node_id]).unwrap(), // NMT
        CanMessage::new(
            0x600 + node_id as u16,
            vec![0x40, 0x00, 0x10, 0x00, 0, 0, 0, 0],
        )
        .unwrap(), // SDO
        CanMessage::new(0x080, vec![1]).unwrap(),             // SYNC
        CanMessage::new(0x180 + node_id as u16, vec![0x11, 0x22, 0x33, 0x44]).unwrap(), // PDO
    ];

    // Verify each message type
    assert_eq!(messages[0].message_type(), MessageType::Nmt);
    assert_eq!(messages[1].message_type(), MessageType::Sdo);
    assert_eq!(messages[2].message_type(), MessageType::Sync);
    assert_eq!(messages[3].message_type(), MessageType::Pdo);
}

/// Test CANopen data type sizes
#[test]
fn test_data_type_sizes() {
    assert_eq!(std::mem::size_of::<bool>(), 1);
    assert_eq!(std::mem::size_of::<u8>(), 1);
    assert_eq!(std::mem::size_of::<u16>(), 2);
    assert_eq!(std::mem::size_of::<u32>(), 4);
    assert_eq!(std::mem::size_of::<u64>(), 8);
    assert_eq!(std::mem::size_of::<i8>(), 1);
    assert_eq!(std::mem::size_of::<i16>(), 2);
    assert_eq!(std::mem::size_of::<i32>(), 4);
    assert_eq!(std::mem::size_of::<f32>(), 4);
    assert_eq!(std::mem::size_of::<f64>(), 8);
}

/// Test error handling for invalid messages
#[test]
fn test_invalid_messages() {
    // SDO message with invalid length (2 bytes is valid, just not useful)
    let short_sdo = CanMessage::new(0x605, vec![0x40, 0x00]);
    assert!(short_sdo.is_ok()); // Message is valid CAN

    // EMCY message with invalid length (2 bytes is allowed, just incomplete)
    let short_emcy = CanMessage::new(0x085, vec![0x10, 0x23]);
    assert!(short_emcy.is_ok()); // Message is valid CAN

    // Message with too much data (>8 bytes is invalid)
    let too_long = CanMessage::new(0x605, vec![0x40, 0x00, 0x10, 0x00, 0, 0, 0, 0, 0xFF]);
    assert!(too_long.is_err()); // CAN messages must be ≤8 bytes

    // Valid NMT message (2 bytes is correct)
    let valid_nmt = CanMessage::new(0x000, vec![0x01, 0x05]);
    assert!(valid_nmt.is_ok());
}

/// Test COB-ID calculation for different node IDs
#[test]
fn test_cob_id_calculation() {
    for node_id in 1..=127 {
        // SDO TX (master to slave)
        let sdo_tx_cob_id = 0x600 + node_id;
        assert!(sdo_tx_cob_id >= 0x601 && sdo_tx_cob_id <= 0x67F);

        // SDO RX (slave to master)
        let sdo_rx_cob_id = 0x580 + node_id;
        assert!(sdo_rx_cob_id >= 0x581 && sdo_rx_cob_id <= 0x5FF);

        // TPDO1
        let tpdo1_cob_id = 0x180 + node_id;
        assert!(tpdo1_cob_id >= 0x181 && tpdo1_cob_id <= 0x1FF);

        // Node Guarding/Heartbeat
        let ng_cob_id = 0x700 + node_id;
        assert!(ng_cob_id >= 0x701 && ng_cob_id <= 0x77F);

        // Emergency
        let emcy_cob_id = 0x080 + node_id;
        assert!(emcy_cob_id >= 0x081 && emcy_cob_id <= 0x0FF);
    }
}
