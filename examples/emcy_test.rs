/// EMCY (Emergency) Test Example
///
/// This example demonstrates:
/// - Monitoring emergency messages from CANopen nodes
/// - Parsing error codes and descriptions
/// - Using callbacks for real-time emergency notifications
/// - Retrieving recent emergency messages
use libcanopen_client::*;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    println!("libCANopen-simple Emergency (EMCY) Test Example");
    println!("=================================================\n");

    // Create PEAK CAN adapter
    println!("Connecting to CAN hardware at 1 Mbps...");
    let peak_adapter = PeakCanAdapter::new(PcanHandle::PcanUsbbus1, BusSpeed::Baud1M);

    // Create CANopen instance
    let mut canopen = CANopenSimple::new(Box::new(peak_adapter));

    // Connect to hardware
    canopen.connect(BusSpeed::Baud1M).await?;
    println!("Connected successfully!\n");

    // Test 1: Register emergency handlers for multiple nodes
    println!("1. Emergency Handler Registration");
    println!("==================================");

    let node1_emcy_count = Arc::new(AtomicU32::new(0));
    let node2_emcy_count = Arc::new(AtomicU32::new(0));
    let node9_emcy_count = Arc::new(AtomicU32::new(0));

    // Handler for Node 1
    {
        let count = Arc::clone(&node1_emcy_count);
        canopen
            .register_emcy_handler(1, move |emcy| {
                count.fetch_add(1, Ordering::SeqCst);
                println!("  [Node 1 EMCY] Error Code: 0x{:04X}", emcy.error_code);
                println!(
                    "                Description: {}",
                    emcy.error_code_description()
                );
                println!(
                    "                Error Register: 0x{:02X}",
                    emcy.error_register
                );
                println!(
                    "                Manufacturer Data: {:02X?}",
                    emcy.manufacturer_data
                );
            })
            .await?;
        println!("✓ Registered emergency handler for Node 1");
    }

    // Handler for Node 2
    {
        let count = Arc::clone(&node2_emcy_count);
        canopen
            .register_emcy_handler(2, move |emcy| {
                count.fetch_add(1, Ordering::SeqCst);
                println!(
                    "  [Node 2 EMCY] Error Code: 0x{:04X} - {}",
                    emcy.error_code,
                    emcy.error_code_description()
                );
            })
            .await?;
        println!("✓ Registered emergency handler for Node 2");
    }

    // Handler for Node 9
    {
        let count = Arc::clone(&node9_emcy_count);
        canopen
            .register_emcy_handler(9, move |emcy| {
                count.fetch_add(1, Ordering::SeqCst);
                if emcy.is_error_reset() {
                    println!("  [Node 9 EMCY] ✓ Error Reset (0x0000)");
                } else {
                    println!(
                        "  [Node 9 EMCY] Error: 0x{:04X} - {}",
                        emcy.error_code,
                        emcy.error_code_description()
                    );
                }
            })
            .await?;
        println!("✓ Registered emergency handler for Node 9");
    }

    println!();

    // Test 2: Listen for emergency messages
    println!("2. Emergency Message Monitoring");
    println!("===============================");
    println!("Listening for emergency messages for 10 seconds...");
    println!("(Waiting for CANopen devices to send EMCY messages)");
    println!();

    sleep(Duration::from_secs(10)).await;

    let node1_count = node1_emcy_count.load(Ordering::SeqCst);
    let node2_count = node2_emcy_count.load(Ordering::SeqCst);
    let node9_count = node9_emcy_count.load(Ordering::SeqCst);

    println!("\nEmergency Statistics:");
    println!("  Node 1: {} emergencies received", node1_count);
    println!("  Node 2: {} emergencies received", node2_count);
    println!("  Node 9: {} emergencies received", node9_count);

    if node1_count == 0 && node2_count == 0 && node9_count == 0 {
        println!("  ⚠ No emergency messages received");
        println!("    (This is normal if devices are not in error state)");
    }

    println!();

    // Test 3: Retrieve recent emergency messages
    println!("3. Recent Emergency Retrieval");
    println!("=============================");

    for node_id in [1u8, 2, 9] {
        match canopen.get_recent_emcy(node_id).await {
            Some(emcy) => {
                println!("Node {} - Last Emergency:", node_id);
                println!("  Error Code: 0x{:04X}", emcy.error_code);
                println!("  Description: {}", emcy.error_code_description());
                println!("  Error Register: 0x{:02X}", emcy.error_register);
                println!("  Manufacturer Data: {:02X?}", emcy.manufacturer_data);
            }
            None => {
                println!("Node {} - No emergency messages received", node_id);
            }
        }
    }

    println!();

    // Test 4: Unregister handlers
    println!("4. Handler Unregistration Test");
    println!("==============================");

    canopen.unregister_emcy_handler(1).await?;
    println!("✓ Unregistered handler for Node 1");

    println!("Waiting 3 seconds (Node 1 emergencies should no longer print)...");
    sleep(Duration::from_secs(3)).await;
    println!();

    // Test 5: Demonstrate common error codes
    println!("5. Common Emergency Error Codes");
    println!("================================");
    println!("CANopen standard emergency error codes:");
    println!();

    let common_codes = vec![
        (0x0000, "No Error / Error Reset"),
        (0x1000, "Generic Error"),
        (0x2000, "Current - Generic"),
        (0x3000, "Voltage - Generic"),
        (0x4000, "Temperature - Generic"),
        (0x5000, "Device Hardware"),
        (0x6000, "Device Software - Generic"),
        (0x8100, "Communication - Generic"),
        (0x8110, "CAN Overrun (Objects Lost)"),
        (0x8120, "CAN Passive Mode"),
        (0x8130, "Life Guard Error / Heartbeat Error"),
        (0x8200, "Protocol Error - Generic"),
        (0x8210, "PDO Length Error"),
        (0x8240, "Unexpected SYNC Length"),
        (0x8250, "RPDO Timeout"),
    ];

    for (code, description) in common_codes {
        println!("  0x{:04X}: {}", code, description);
    }

    println!();

    // Test 6: Clear recent emergency storage
    println!("6. Clear Emergency Storage");
    println!("==========================");

    canopen.clear_recent_emcy().await?;
    println!("✓ Cleared recent emergency message storage");

    // Verify it's cleared
    println!("Verifying storage is cleared...");
    let mut any_found = false;
    for node_id in [1u8, 2, 9] {
        if canopen.get_recent_emcy(node_id).await.is_some() {
            any_found = true;
            println!("  ⚠ Node {} still has stored emergency", node_id);
        }
    }

    if !any_found {
        println!("  ✓ All emergency storage cleared successfully");
    }

    println!();

    // Test 7: Extended monitoring
    println!("7. Extended Monitoring Test");
    println!("===========================");
    println!("Monitoring all registered nodes for 30 seconds...");
    println!("This allows time for devices to enter error states");
    println!();

    // Re-register Node 1 handler
    {
        let count = Arc::clone(&node1_emcy_count);
        canopen
            .register_emcy_handler(1, move |emcy| {
                count.fetch_add(1, Ordering::SeqCst);
                println!(
                    "  [Node 1 EMCY] 0x{:04X} - {}",
                    emcy.error_code,
                    emcy.error_code_description()
                );
            })
            .await?;
    }

    let start_time = std::time::Instant::now();
    let mut last_report = std::time::Instant::now();

    while start_time.elapsed() < Duration::from_secs(30) {
        sleep(Duration::from_secs(1)).await;

        // Print status every 5 seconds
        if last_report.elapsed() >= Duration::from_secs(5) {
            let elapsed = start_time.elapsed().as_secs();
            println!(
                "  [{}s] Monitoring... (Node 1: {}, Node 2: {}, Node 9: {})",
                elapsed,
                node1_emcy_count.load(Ordering::SeqCst),
                node2_emcy_count.load(Ordering::SeqCst),
                node9_emcy_count.load(Ordering::SeqCst)
            );
            last_report = std::time::Instant::now();
        }
    }

    println!();
    println!("Final Emergency Statistics:");
    println!(
        "  Node 1: {} emergencies",
        node1_emcy_count.load(Ordering::SeqCst)
    );
    println!(
        "  Node 2: {} emergencies",
        node2_emcy_count.load(Ordering::SeqCst)
    );
    println!(
        "  Node 9: {} emergencies",
        node9_emcy_count.load(Ordering::SeqCst)
    );

    println!();
    println!("Cleaning up...");

    // Disconnect
    canopen.disconnect().await?;
    println!("Disconnecting...");

    println!("\nEmergency test completed successfully!");
    println!("\nNote: If no emergencies were received, this is normal.");
    println!("Devices only send EMCY messages when errors occur.");
    println!("To test: disconnect a device, cause overload, etc.");

    Ok(())
}
