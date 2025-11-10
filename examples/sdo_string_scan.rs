//! SDO String Read Test with Node Discovery
//!
//! This example first scans for active nodes, then attempts to read the Device Name
//! from a specified node (or the first discovered node).
//!
//! Usage:
//!   cargo run --release --example sdo_string_scan

use libcanopen_client::*;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    println!("=== CANopen SDO String Read Test with Discovery ===\n");

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

    // Scan for nodes
    println!("Scanning for active nodes (waiting 3 seconds for heartbeats)...");
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    let discovered_nodes = canopen.nmt_get_discovered_nodes().await;

    if discovered_nodes.is_empty() {
        println!("✗ No nodes discovered on the network!");
        println!("\nPossible issues:");
        println!("  - No CANopen devices connected");
        println!("  - Devices not sending heartbeats");
        println!("  - Wrong bus speed (currently using 1 Mbps)");
        println!("  - Physical connection problems");
        canopen.disconnect().await?;
        return Ok(());
    }

    println!("✓ Discovered {} node(s):", discovered_nodes.len());
    for node_id in &discovered_nodes {
        if let Some(state) = canopen.nmt_get_node_state(*node_id).await {
            println!("  - Node {}: {:?}", node_id, state);
        } else {
            println!("  - Node {}: Unknown state", node_id);
        }
    }
    println!();

    // Try to read Device Name from each discovered node
    let mut results = HashMap::new();

    for node_id in &discovered_nodes {
        println!("Reading Device Name from Node {}...", node_id);

        match canopen.sdo_read_data(*node_id, 0x1008, 0, 1000).await {
            Ok(data) => {
                let device_name = if let Some(null_pos) = data.iter().position(|&b| b == 0) {
                    String::from_utf8_lossy(&data[..null_pos]).to_string()
                } else {
                    String::from_utf8_lossy(&data).to_string()
                };
                println!("  ✓ Device Name: \"{}\"", device_name);
                results.insert(*node_id, Some(device_name));
            }
            Err(CANopenError::Sdo { code }) => {
                println!(
                    "  ✗ SDO Error: 0x{:08X} - {}",
                    code,
                    get_sdo_abort_description(code)
                );
                results.insert(*node_id, None);
            }
            Err(e) => {
                println!("  ✗ Error: {:?}", e);
                results.insert(*node_id, None);
            }
        }
        println!();
    }

    // Summary
    println!("=== Summary ===");
    for (node_id, name) in results {
        match name {
            Some(n) => println!("Node {}: \"{}\"", node_id, n),
            None => println!("Node {}: Failed to read", node_id),
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
