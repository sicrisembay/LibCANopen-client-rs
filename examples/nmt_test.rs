//! NMT (Network Management) functionality test example
//!
//! This example demonstrates the NMT capabilities of libCANopen-simple:
//! - Node discovery via heartbeat messages
//! - Sending NMT commands (start, stop, reset, etc.)
//! - Monitoring node states
//! - Heartbeat timeout checking

use libcanopen_client::hardware::{PcanHandle, PeakCanAdapter};
use libcanopen_client::{BusSpeed, CANopenSimple, Result};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    println!("libCANopen-simple NMT Test Example");
    println!("===================================\n");

    // Create PEAK CAN adapter (USB bus 1, 1 Mbps)
    let peak_adapter = PeakCanAdapter::new(PcanHandle::PcanUsbbus1, BusSpeed::Baud1M);

    // Create CANopen instance
    let mut canopen = CANopenSimple::new(Box::new(peak_adapter));

    println!("Connecting to CAN hardware at 1 Mbps...");
    canopen.connect(BusSpeed::Baud1M).await?;
    println!("Connected successfully!\n");

    // Give time for nodes to send heartbeats
    println!("Waiting for heartbeat messages (3 seconds)...");
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Test 1: Node Discovery
    println!("\n1. Node Discovery");
    println!("=================");
    let discovered_nodes = canopen.nmt_get_discovered_nodes().await;

    if discovered_nodes.is_empty() {
        println!("⚠ No nodes discovered. Make sure CANopen devices are connected and sending heartbeats.");
    } else {
        println!("✓ Discovered {} node(s):", discovered_nodes.len());
        for node_id in &discovered_nodes {
            if let Some(state) = canopen.nmt_get_node_state(*node_id).await {
                println!("  - Node {}: State = {:?}", node_id, state);
            }
        }
    }

    // Test 2: Check if specific node is found
    println!("\n2. Testing Node Presence Check");
    println!("===============================");
    let test_node_id = 2;
    let is_found = canopen.nmt_is_node_found(test_node_id).await;

    if is_found {
        println!("✓ Node {} is present", test_node_id);
        if let Some(state) = canopen.nmt_get_node_state(test_node_id).await {
            println!("  Current state: {:?}", state);
        }
    } else {
        println!("✗ Node {} not found", test_node_id);
    }

    // Test 3: Heartbeat Timeout Check
    println!("\n3. Testing Heartbeat Monitoring");
    println!("================================");
    let timeout = Duration::from_secs(2);

    for node_id in &discovered_nodes {
        let is_alive = canopen.nmt_check_heartbeat(*node_id, timeout).await;
        if is_alive {
            println!("✓ Node {} heartbeat OK (< 2s)", node_id);
        } else {
            println!("✗ Node {} heartbeat timeout (> 2s)", node_id);
        }
    }

    // Test 4: NMT Commands (if nodes discovered)
    if !discovered_nodes.is_empty() {
        println!("\n4. Testing NMT Commands");
        println!("=======================");

        let target_node = discovered_nodes[0];
        println!("Testing with Node {}:", target_node);

        // Send Pre-Operational command
        println!("\n  a) Sending Pre-Operational command...");
        canopen.nmt_enter_pre_operational(target_node).await?;
        tokio::time::sleep(Duration::from_millis(500)).await;

        if let Some(state) = canopen.nmt_get_node_state(target_node).await {
            println!("     Node {} state: {:?}", target_node, state);
        }

        // Send Start command (Operational)
        println!("\n  b) Sending Start (Operational) command...");
        canopen.nmt_start(target_node).await?;
        tokio::time::sleep(Duration::from_millis(500)).await;

        if let Some(state) = canopen.nmt_get_node_state(target_node).await {
            println!("     Node {} state: {:?}", target_node, state);
        }

        // Note: We don't test Stop or Reset as they may disrupt devices
        println!("\n  ℹ Skipping Stop/Reset commands to avoid disrupting devices");
    }

    // Test 5: Broadcast NMT Command
    println!("\n5. Testing Broadcast Commands");
    println!("==============================");
    println!("Sending broadcast Pre-Operational command (to all nodes)...");
    canopen.nmt_enter_pre_operational(0).await?;
    tokio::time::sleep(Duration::from_millis(500)).await;

    println!("\nNode states after broadcast:");
    for node_id in &discovered_nodes {
        if let Some(state) = canopen.nmt_get_node_state(*node_id).await {
            println!("  - Node {}: {:?}", node_id, state);
        }
    }

    // Restore nodes to operational
    println!("\nRestoring all nodes to Operational state...");
    canopen.nmt_start(0).await?;
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Final status
    println!("\n6. Final Node Status");
    println!("====================");
    for node_id in &discovered_nodes {
        if let Some(state) = canopen.nmt_get_node_state(*node_id).await {
            let is_alive = canopen
                .nmt_check_heartbeat(*node_id, Duration::from_secs(2))
                .await;
            println!(
                "  - Node {}: {:?}, Heartbeat: {}",
                node_id,
                state,
                if is_alive { "OK" } else { "Timeout" }
            );
        }
    }

    // Disconnect
    println!("\nDisconnecting...");
    canopen.disconnect().await?;
    println!("NMT test completed successfully!");

    Ok(())
}
