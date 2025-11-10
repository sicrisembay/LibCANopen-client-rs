//! SDO String Read Test
//!
//! This example demonstrates reading a string value from the CANopen object dictionary.
//! It reads index 0x1008:0 (Device Name) from node 10.
//!
//! Usage:
//!   cargo run --release --example sdo_string_test

use libcanopen_client::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    println!("=== CANopen SDO String Read Test ===\n");

    // Create PEAK CAN adapter
    let hardware = Box::new(PeakCanAdapter::new(
        PcanHandle::PcanUsbbus1,
        BusSpeed::Baud1M,
    ));

    // Create CANopen instance
    let mut canopen = CANopenSimple::new(hardware);

    // Connect to CAN bus
    println!("Connecting to CAN bus at 1 Mbps...");
    canopen.connect(BusSpeed::Baud1M).await?;
    println!("Connected!\n");

    // Give the bus a moment to stabilize
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Test parameters
    let node_id = 10;
    let index = 0x1008;
    let subindex = 0;
    let timeout_ms = 1000;

    println!("Reading Device Name from Node {}...", node_id);
    println!("  Index: 0x{:04X}", index);
    println!("  Subindex: {}", subindex);
    println!("  Timeout: {}ms\n", timeout_ms);

    // Read the string data
    match canopen
        .sdo_read_data(node_id, index, subindex, timeout_ms)
        .await
    {
        Ok(data) => {
            println!("✓ SDO Read Successful!");
            println!("  Raw data ({} bytes): {:02X?}", data.len(), data);

            // Convert to string (CANopen strings are typically null-terminated or length-prefixed)
            // Try to interpret as UTF-8 string, removing null terminator if present
            let device_name = if let Some(null_pos) = data.iter().position(|&b| b == 0) {
                // Null-terminated string
                String::from_utf8_lossy(&data[..null_pos]).to_string()
            } else {
                // No null terminator, use entire buffer
                String::from_utf8_lossy(&data).to_string()
            };

            println!("  Device Name: \"{}\"", device_name);
        }
        Err(CANopenError::Timeout) => {
            println!("✗ Error: Timeout waiting for response");
            println!("  Possible causes:");
            println!("    - Node {} is not present on the network", node_id);
            println!("    - Node is not in Pre-Operational or Operational state");
            println!("    - CAN bus speed mismatch");
            println!("    - Physical connection issue");
        }
        Err(CANopenError::Sdo { code }) => {
            println!("✗ SDO Abort Code: 0x{:08X}", code);
            println!("  Description: {}", get_sdo_abort_description(code));
        }
        Err(e) => {
            println!("✗ Error: {:?}", e);
        }
    }

    println!("\nDisconnecting...");
    canopen.disconnect().await?;
    println!("Done!");

    Ok(())
}

/// Get human-readable description for SDO abort codes
fn get_sdo_abort_description(code: u32) -> &'static str {
    match code {
        0x05040000 => "SDO protocol timed out",
        0x06010000 => "Unsupported access to an object",
        0x06010001 => "Attempt to read a write-only object",
        0x06010002 => "Attempt to write a read-only object",
        0x06020000 => "Object does not exist in the object dictionary",
        0x06040041 => "Object cannot be mapped to the PDO",
        0x06040042 => "Number and length of objects to be mapped exceeds PDO length",
        0x06040043 => "General parameter incompatibility reason",
        0x06040047 => "General internal incompatibility in device",
        0x06060000 => "Access failed due to hardware error",
        0x06070010 => "Data type does not match, length of service parameter does not match",
        0x06070012 => "Data type does not match, length of service parameter too high",
        0x06070013 => "Data type does not match, length of service parameter too low",
        0x06090011 => "Sub-index does not exist",
        0x06090030 => "Value range of parameter exceeded",
        0x06090031 => "Value of parameter written too high",
        0x06090032 => "Value of parameter written too low",
        0x06090036 => "Maximum value is less than minimum value",
        0x08000000 => "General error",
        0x08000020 => "Data cannot be transferred or stored to the application",
        0x08000021 => "Data cannot be transferred because of local control",
        0x08000022 => "Data cannot be transferred because of the present device state",
        0x08000023 => "Object dictionary dynamic generation fails or no OD is present",
        _ => "Unknown SDO abort code",
    }
}
