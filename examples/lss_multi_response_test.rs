/// LSS Multi-Response Test Example
///
/// This example demonstrates the new capability to collect responses from
/// multiple LSS slaves simultaneously when in global configuration mode.
///
/// According to CiA 305 LSS protocol, when an inquiry command is broadcast
/// in global configuration mode, multiple LSS slaves can respond. This example
/// shows how to use the new plural inquiry functions to collect all responses.
use libcanopen_client::*;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    println!("libCANopen-simple LSS Multi-Response Test Example");
    println!("==================================================\n");

    // Create PEAK CAN adapter
    println!("Connecting to CAN hardware at 1 Mbps...");
    let peak_adapter = PeakCanAdapter::new(PcanHandle::PcanUsbbus1, BusSpeed::Baud1M);

    // Create CANopen instance
    let mut canopen = CANopenSimple::new(Box::new(peak_adapter));

    // Connect to hardware
    canopen.connect(BusSpeed::Baud1M).await?;
    println!("Connected successfully!\n");

    // Switch to global configuration mode
    println!("Switching all unconfigured slaves to configuration mode...");
    canopen
        .lss_switch_state_global(LssMode::Configuration)
        .await?;
    println!("✓ Switched to global configuration mode\n");

    // Wait for slaves to process
    sleep(Duration::from_millis(500)).await;

    // Test 1: Single response (backward compatible)
    println!("1. Single Response API (Backward Compatible)");
    println!("=============================================");
    println!("Using lss_inquire_vendor_id() - returns first response only");

    match canopen.lss_inquire_vendor_id(1000).await {
        Ok(vendor_id) => {
            println!("✓ Received vendor ID: 0x{:08X}", vendor_id);
        }
        Err(CANopenError::Timeout) => {
            println!("⚠ Timeout - no response received");
            println!("  (This is normal if no unconfigured slaves are present)");
        }
        Err(e) => {
            println!("⚠ Error: {:?}", e);
        }
    }

    println!();
    sleep(Duration::from_millis(200)).await;

    // Test 2: Multiple responses (new capability)
    println!("2. Multiple Response API (NEW Feature)");
    println!("=======================================");
    println!("Using lss_inquire_vendor_ids() - collects ALL responses\n");

    println!("Collecting vendor IDs from all slaves (1000ms timeout)...");
    match canopen.lss_inquire_vendor_ids(1000).await {
        Ok(vendor_ids) => {
            if vendor_ids.is_empty() {
                println!("⚠ No responses received");
                println!("  (This is normal if no unconfigured slaves are present)");
            } else {
                println!("✓ Received {} unique vendor ID(s):", vendor_ids.len());
                for (i, id) in vendor_ids.iter().enumerate() {
                    println!("  [{}] Vendor ID: 0x{:08X}", i + 1, id);
                }
            }
        }
        Err(e) => {
            println!("⚠ Error: {:?}", e);
        }
    }

    println!();
    sleep(Duration::from_millis(200)).await;

    // Test 3: Complete identity discovery
    println!("3. Complete Identity Discovery");
    println!("==============================");
    println!("Collecting all identity fields from multiple slaves\n");

    println!("Querying vendor IDs...");
    let vendor_ids = canopen
        .lss_inquire_vendor_ids(1000)
        .await
        .unwrap_or_default();

    println!("Querying product codes...");
    let product_codes = canopen
        .lss_inquire_product_codes(1000)
        .await
        .unwrap_or_default();

    println!("Querying revision numbers...");
    let revision_numbers = canopen
        .lss_inquire_revision_numbers(1000)
        .await
        .unwrap_or_default();

    println!("Querying serial numbers...");
    let serial_numbers = canopen
        .lss_inquire_serial_numbers(1000)
        .await
        .unwrap_or_default();

    println!("Querying node IDs...");
    let node_ids = canopen.lss_inquire_node_ids(1000).await.unwrap_or_default();

    println!();
    println!("Discovery Results:");
    println!("------------------");
    println!("Vendor IDs found:       {}", vendor_ids.len());
    println!("Product codes found:    {}", product_codes.len());
    println!("Revision numbers found: {}", revision_numbers.len());
    println!("Serial numbers found:   {}", serial_numbers.len());
    println!("Node IDs found:         {}", node_ids.len());

    if !vendor_ids.is_empty() {
        println!();
        println!("Detected Devices:");
        println!("-----------------");

        // Display vendor IDs
        println!("\nVendor IDs:");
        for (i, id) in vendor_ids.iter().enumerate() {
            println!("  Device {}: 0x{:08X}", i + 1, id);
        }

        // Display product codes
        if !product_codes.is_empty() {
            println!("\nProduct Codes:");
            for (i, code) in product_codes.iter().enumerate() {
                println!("  Device {}: 0x{:08X}", i + 1, code);
            }
        }

        // Display revision numbers
        if !revision_numbers.is_empty() {
            println!("\nRevision Numbers:");
            for (i, rev) in revision_numbers.iter().enumerate() {
                println!("  Device {}: 0x{:08X}", i + 1, rev);
            }
        }

        // Display serial numbers
        if !serial_numbers.is_empty() {
            println!("\nSerial Numbers:");
            for (i, serial) in serial_numbers.iter().enumerate() {
                println!("  Device {}: 0x{:08X}", i + 1, serial);
            }
        }

        // Display node IDs
        if !node_ids.is_empty() {
            println!("\nCurrent Node IDs:");
            for (i, node_id) in node_ids.iter().enumerate() {
                println!("  Device {}: {}", i + 1, node_id);
                if *node_id == 255 || *node_id == 0 {
                    println!("           (Unconfigured - ID is invalid)");
                }
            }
        }
    } else {
        println!("\n⚠ No unconfigured slaves detected on the network");
        println!("  This is normal if:");
        println!("  - All slaves are already configured with valid node IDs");
        println!("  - No LSS-capable slaves are connected");
        println!("  - Slaves are in operation mode (not configuration mode)");
    }

    println!();

    // Test 4: Performance comparison
    println!("4. Timeout Behavior");
    println!("===================");
    println!("Demonstrating how timeout affects response collection\n");

    println!("Short timeout (100ms) - may miss slower devices:");
    let start = std::time::Instant::now();
    let results_short = canopen
        .lss_inquire_vendor_ids(100)
        .await
        .unwrap_or_default();
    let duration_short = start.elapsed();
    println!(
        "  Collected {} response(s) in {:?}",
        results_short.len(),
        duration_short
    );

    sleep(Duration::from_millis(200)).await;

    println!("\nLonger timeout (2000ms) - catches all devices:");
    let start = std::time::Instant::now();
    let results_long = canopen
        .lss_inquire_vendor_ids(2000)
        .await
        .unwrap_or_default();
    let duration_long = start.elapsed();
    println!(
        "  Collected {} response(s) in {:?}",
        results_long.len(),
        duration_long
    );

    if results_long.len() > results_short.len() {
        println!(
            "\n✓ Longer timeout caught {} additional device(s)!",
            results_long.len() - results_short.len()
        );
    }

    println!();

    // Test 5: Use case example
    println!("5. Practical Use Case Example");
    println!("==============================");
    println!("Network Discovery Workflow:\n");

    println!("Step 1: Discover all unconfigured devices");
    let discovered_vendor_ids = canopen
        .lss_inquire_vendor_ids(1000)
        .await
        .unwrap_or_default();

    if !discovered_vendor_ids.is_empty() {
        println!(
            "  Found {} unconfigured device(s)",
            discovered_vendor_ids.len()
        );
        println!("\nStep 2: For each device, you could:");
        println!("  a) Query complete LSS address (vendor, product, revision, serial)");
        println!("  b) Use lss_switch_state_selective() to select specific device");
        println!("  c) Configure node ID with lss_configure_node_id()");
        println!("  d) Store configuration with lss_store_configuration()");
        println!("  e) Switch back to operation mode");
        println!("\nThis enables automated network commissioning!");
    } else {
        println!("  No unconfigured devices - network appears fully configured");
    }

    println!();

    // Switch back to operation mode
    println!("Switching back to operation mode...");
    canopen.lss_switch_state_global(LssMode::Operation).await?;
    println!("✓ Switched to operation mode");

    // Disconnect
    canopen.disconnect().await?;
    println!("\nTest completed successfully!");

    println!("\n{}", "=".repeat(60));
    println!("KEY FEATURES DEMONSTRATED:");
    println!("{}", "=".repeat(60));
    println!("✓ Backward compatible single-response API");
    println!("✓ New multi-response API for discovering multiple devices");
    println!("✓ Complete identity discovery across all fields");
    println!("✓ Timeout behavior and performance considerations");
    println!("✓ Practical network commissioning workflow");
    println!("\nAPI SUMMARY:");
    println!("  Single: lss_inquire_vendor_id()     -> Result<u32>");
    println!("  Multi:  lss_inquire_vendor_ids()    -> Result<Vec<u32>>");
    println!("  (Same pattern for product_code, revision_number, serial_number, node_id)");

    Ok(())
}
