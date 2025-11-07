use libcanopen_client::hardware::peak_can::{PcanHandle, PeakCanAdapter};
use libcanopen_client::{BusSpeed, CANopenSimple};
use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc,
};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("libCANopen-simple PDO Test Example");
    println!("===================================\n");

    // Create and connect to CAN hardware
    println!("Connecting to CAN hardware at 1 Mbps...");
    let hardware = Box::new(PeakCanAdapter::new(
        PcanHandle::PcanUsbbus1,
        BusSpeed::Baud1M,
    ));
    let mut canopen = CANopenSimple::new(hardware);
    canopen.connect(BusSpeed::Baud1M).await?;
    println!("Connected successfully!\n");

    // Wait a moment for the bus to stabilize
    sleep(Duration::from_millis(500)).await;

    println!("1. PDO Callback Registration Test");
    println!("===================================");

    // Counters for received PDOs
    let node1_count = Arc::new(AtomicU32::new(0));
    let node2_count = Arc::new(AtomicU32::new(0));

    // Register handlers for TPDO1 from nodes 1 and 2
    println!("Registering PDO handlers for:");
    println!("  - Node 1 TPDO1 (COB-ID 0x181)");
    println!("  - Node 2 TPDO1 (COB-ID 0x182)");

    let node1_count_clone = Arc::clone(&node1_count);
    canopen
        .register_pdo_handler(0x181, move |data| {
            node1_count_clone.fetch_add(1, Ordering::Relaxed);
            println!(
                "  [Node 1] TPDO1 received: {} bytes: {:02X?}",
                data.len(),
                data
            );
        })
        .await;

    let node2_count_clone = Arc::clone(&node2_count);
    canopen
        .register_pdo_handler(0x182, move |data| {
            node2_count_clone.fetch_add(1, Ordering::Relaxed);
            println!(
                "  [Node 2] TPDO1 received: {} bytes: {:02X?}",
                data.len(),
                data
            );
        })
        .await;

    println!("\nListening for PDO messages (5 seconds)...");
    sleep(Duration::from_secs(5)).await;

    let node1_received = node1_count.load(Ordering::Relaxed);
    let node2_received = node2_count.load(Ordering::Relaxed);

    println!("\nReceived PDOs:");
    println!("  Node 1: {} messages", node1_received);
    println!("  Node 2: {} messages", node2_received);

    if node1_received > 0 || node2_received > 0 {
        println!("✓ Successfully receiving PDO messages!\n");
    } else {
        println!("⚠ No PDO messages received (nodes may not be transmitting)\n");
    }

    println!("\n2. PDO Write Test");
    println!("==================");

    // Test sending PDOs to a node
    // RPDO1 for node 1 is at COB-ID 0x201
    let test_data = vec![0x11, 0x22, 0x33, 0x44];
    println!("Sending RPDO1 to node 1 (COB-ID 0x201)");
    println!("  Data: {:02X?}", test_data);

    match canopen.write_pdo(0x201, test_data).await {
        Ok(_) => println!("✓ PDO sent successfully"),
        Err(e) => println!("✗ Failed to send PDO: {:?}", e),
    }

    // Send a few more test PDOs
    println!("\nSending multiple PDOs...");
    for i in 0..3 {
        let data = vec![0x10 + i, 0x20 + i, 0x30 + i];
        canopen.write_pdo(0x201, data).await?;
        sleep(Duration::from_millis(100)).await;
    }
    println!("✓ Sent 3 test PDOs\n");

    println!("\n3. PDO Storage Test (without callbacks)");
    println!("========================================");

    // Register a handler for node 9
    println!("Registering handler for Node 9 TPDO1 (COB-ID 0x189)");
    canopen
        .register_pdo_handler(0x189, |data| {
            println!("  [Node 9] TPDO1: {:02X?}", data);
        })
        .await;

    // Wait for some PDOs to arrive
    sleep(Duration::from_secs(2)).await;

    // Try to get recent PDO data
    println!("\nChecking stored PDO data:");
    for cob_id in [0x181, 0x182, 0x189] {
        if let Some(data) = canopen.get_recent_pdo(cob_id).await {
            println!("  COB-ID 0x{:03X}: {:02X?}", cob_id, data);
        } else {
            println!("  COB-ID 0x{:03X}: No data", cob_id);
        }
    }

    println!("\n4. PDO Handler Unregister Test");
    println!("===============================");

    println!("Unregistering handler for Node 1 (COB-ID 0x181)");
    canopen.unregister_pdo_handler(0x181).await;

    println!("Waiting 2 seconds (Node 1 PDOs should no longer print)...");
    sleep(Duration::from_secs(2)).await;

    println!("✓ Node 1 handler unregistered successfully\n");

    println!("\n5. Multiple Node PDO Test");
    println!("==========================");

    // Register handlers for all discovered nodes
    println!("Registering handlers for multiple nodes:");

    let nodes_to_monitor = [1, 2, 3, 9];
    let counters: Vec<Arc<AtomicU32>> = (0..nodes_to_monitor.len())
        .map(|_| Arc::new(AtomicU32::new(0)))
        .collect();

    for (idx, &node_id) in nodes_to_monitor.iter().enumerate() {
        let cob_id = 0x180 + node_id as u16;
        let counter = Arc::clone(&counters[idx]);

        canopen
            .register_pdo_handler(cob_id, move |_data| {
                counter.fetch_add(1, Ordering::Relaxed);
            })
            .await;

        println!("  Node {} TPDO1: COB-ID 0x{:03X}", node_id, cob_id);
    }

    println!("\nMonitoring for 3 seconds...");
    sleep(Duration::from_secs(3)).await;

    println!("\nPDO Statistics:");
    for (idx, &node_id) in nodes_to_monitor.iter().enumerate() {
        let count = counters[idx].load(Ordering::Relaxed);
        if count > 0 {
            println!("  Node {}: {} PDOs received", node_id, count);
        }
    }

    println!("\n6. Broadcast PDO Test");
    println!("=====================");

    // Some devices listen for broadcast PDOs
    println!("Sending broadcast RPDO (COB-ID 0x200)");
    let broadcast_data = vec![0xFF, 0xFF, 0xFF, 0xFF];
    canopen.write_pdo(0x200, broadcast_data).await?;
    println!("✓ Broadcast PDO sent\n");

    // Clean up
    println!("\nCleaning up...");
    canopen.clear_recent_pdos().await;
    println!("✓ Cleared recent PDO storage");

    // Disconnect
    println!("\nDisconnecting...");
    canopen.disconnect().await?;
    println!("PDO test completed successfully!");

    Ok(())
}
