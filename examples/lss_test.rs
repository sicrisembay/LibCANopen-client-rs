/// LSS (Layer Setting Services) Test Example
///
/// This example demonstrates:
/// - Selecting slaves by LSS address
/// - Configuring node-ID
/// - Configuring bit-rate
/// - Inquiring LSS address and node-ID
/// - Identifying remote slaves
/// - Storing configuration
use libcanopen_client::*;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    println!("libCANopen-simple LSS (Layer Setting Services) Test Example");
    println!("=============================================================\n");

    // Create PEAK CAN adapter
    println!("Connecting to CAN hardware at 1 Mbps...");
    let peak_adapter = PeakCanAdapter::new(PcanHandle::PcanUsbbus1, BusSpeed::Baud1M);

    // Create CANopen instance
    let mut canopen = CANopenSimple::new(Box::new(peak_adapter));

    // Connect to hardware
    canopen.connect(BusSpeed::Baud1M).await?;
    println!("Connected successfully!\n");

    // Test 1: Switch to global configuration mode
    println!("1. LSS Global Mode Test");
    println!("=======================");
    println!("Switching all unconfigured slaves to configuration mode...");

    canopen
        .lss_switch_state_global(LssMode::Configuration)
        .await?;
    println!("✓ Switched to global configuration mode");

    println!("Waiting 500ms for slaves to process...");
    sleep(Duration::from_millis(500)).await;

    println!();

    // Test 2: Inquire LSS identity fields (for demonstration, may timeout if no unconfigured slave)
    println!("2. LSS Identity Inquiry Test");
    println!("=============================");
    println!("Attempting to inquire LSS identity from slave in configuration mode...");
    println!("(Each field is queried individually for efficient CAN bus usage)");
    println!();

    // Try to query individual fields
    let vendor_id_result = canopen.lss_inquire_vendor_id(1000).await;
    let product_code_result = canopen.lss_inquire_product_code(1000).await;
    let revision_result = canopen.lss_inquire_revision_number(1000).await;
    let serial_result = canopen.lss_inquire_serial_number(1000).await;

    if let (Ok(vendor_id), Ok(product_code), Ok(revision), Ok(serial)) = (
        vendor_id_result,
        product_code_result,
        revision_result,
        serial_result,
    ) {
        println!("✓ LSS Identity received:");
        println!("  Vendor ID:       0x{:08X}", vendor_id);
        println!("  Product Code:    0x{:08X}", product_code);
        println!("  Revision Number: 0x{:08X}", revision);
        println!("  Serial Number:   0x{:08X}", serial);
        println!("\n  This identity can be used for selective mode");
    } else {
        println!("⚠ Timeout - no unconfigured slave responded");
        println!("  (This is normal if all slaves are configured)");
    }

    println!("\nWaiting 500ms before next test...");
    sleep(Duration::from_millis(500)).await;

    println!();

    // Test 3: Switch back to operation mode
    println!("3. Switch to Operation Mode");
    println!("===========================");
    canopen.lss_switch_state_global(LssMode::Operation).await?;
    println!("✓ Switched back to operation mode");
    println!();

    // Test 4: Selective mode (example with known address)
    println!("4. LSS Selective Mode Test");
    println!("==========================");
    println!("Demonstrating selective mode with example address...");

    // Example LSS address (replace with actual values from your device)
    let example_address = LssAddress {
        vendor_id: 0x00000000,       // Replace with actual Vendor ID
        product_code: 0x12345678,    // Replace with actual Product Code
        revision_number: 0x00010000, // Replace with actual Revision
        serial_number: 0xABCDEF00,   // Replace with actual Serial Number
    };

    println!("Selecting slave with address:");
    println!("  Vendor ID:       0x{:08X}", example_address.vendor_id);
    println!("  Product Code:    0x{:08X}", example_address.product_code);
    println!(
        "  Revision Number: 0x{:08X}",
        example_address.revision_number
    );
    println!("  Serial Number:   0x{:08X}", example_address.serial_number);

    canopen.lss_switch_state_selective(&example_address).await?;
    println!("✓ Selective mode command sent");
    sleep(Duration::from_millis(100)).await;

    println!();

    // Test 5: Identify remote slave
    println!("5. LSS Identify Remote Slave Test");
    println!("==================================");
    println!("Checking if selected slave responds...");

    match canopen.lss_identify_remote_slave(1000).await {
        Ok(true) => {
            println!("✓ Slave identified successfully!");
        }
        Ok(false) => {
            println!("⚠ No response - slave not found or not in configuration mode");
        }
        Err(e) => {
            println!("⚠ Error: {:?}", e);
        }
    }

    println!();

    // Test 6: Inquire current node-ID
    println!("6. LSS Inquire Node-ID Test");
    println!("============================");
    println!("Querying current node-ID from selected slave...");

    match canopen.lss_inquire_node_id(1000).await {
        Ok(node_id) => {
            println!("✓ Current Node-ID: {}", node_id);
            if node_id == 255 || node_id == 0 {
                println!("  (Node is unconfigured - ID is invalid)");
            }
        }
        Err(CANopenError::Timeout) => {
            println!("⚠ Timeout - no response from slave");
        }
        Err(e) => {
            println!("⚠ Error: {:?}", e);
        }
    }

    println!();

    // Test 7: Configure new node-ID (CAUTION: This will change the node-ID!)
    println!("7. LSS Configure Node-ID Test (DEMO ONLY)");
    println!("==========================================");
    println!("WARNING: This test would configure a new node-ID!");
    println!("         Disabled in this example to prevent accidental changes.");
    println!();
    println!("Example code:");
    println!("  let new_node_id = 10;");
    println!("  match canopen.lss_configure_node_id(new_node_id, 1000).await {{");
    println!("      Ok(LssError::Success) => {{");
    println!("          println!(\"Node-ID configured to {{}}\", new_node_id);");
    println!("      }}");
    println!("      Ok(error) => {{");
    println!("          println!(\"Error: {{}}\", error.description());");
    println!("      }}");
    println!("      Err(e) => println!(\"Timeout or error: {{:?}}\", e),");
    println!("  }}");
    println!();

    // Uncomment to actually configure (BE CAREFUL!):
    /*
    let new_node_id = 10;
    match canopen.lss_configure_node_id(new_node_id, 1000).await {
        Ok(LssError::Success) => {
            println!("✓ Node-ID configured to {}", new_node_id);

            // Store configuration to non-volatile memory
            println!("Storing configuration...");
            match canopen.lss_store_configuration(2000).await {
                Ok(LssError::Success) => {
                    println!("✓ Configuration stored");
                }
                Ok(error) => {
                    println!("⚠ Store failed: {}", error.description());
                }
                Err(e) => {
                    println!("⚠ Error: {:?}", e);
                }
            }
        }
        Ok(error) => {
            println!("⚠ Configuration failed: {}", error.description());
        }
        Err(e) => {
            println!("⚠ Error: {:?}", e);
        }
    }
    */

    // Test 8: Bit-rate configuration (DEMO ONLY)
    println!("8. LSS Configure Bit-Rate Test (DEMO ONLY)");
    println!("===========================================");
    println!("WARNING: Bit-rate configuration affects network communication!");
    println!("         Disabled in this example to prevent network disruption.");
    println!();
    println!("Common bit-rate table indices (CiA 301):");
    println!("  0: 1000 kbit/s");
    println!("  1: 800 kbit/s");
    println!("  2: 500 kbit/s");
    println!("  3: 250 kbit/s");
    println!("  4: 125 kbit/s");
    println!("  5: 50 kbit/s");
    println!("  6: 20 kbit/s");
    println!("  7: 10 kbit/s");
    println!();
    println!("Example code:");
    println!("  // Configure 500 kbit/s");
    println!("  let table_selector = 0; // CiA 301 table");
    println!("  let table_index = 2;    // 500 kbit/s");
    println!("  ");
    println!("  match canopen.lss_configure_bit_rate(table_selector, table_index, 1000).await {{");
    println!("      Ok(LssError::Success) => {{");
    println!("          // Activate new bit-rate with 100ms delay");
    println!("          canopen.lss_activate_bit_rate(100).await?;");
    println!("          ");
    println!("          // Wait for switch delay");
    println!("          sleep(Duration::from_millis(100)).await;");
    println!("          ");
    println!("          // NOW YOU MUST ALSO SWITCH YOUR CAN HARDWARE!");
    println!("          // canopen.disconnect().await?;");
    println!("          // canopen.connect(BusSpeed::Baud500K).await?;");
    println!("      }}");
    println!("      Ok(error) => println!(\"Error: {{}}\", error.description()),");
    println!("      Err(e) => println!(\"Error: {{:?}}\", e),");
    println!("  }}");
    println!();

    // Test 9: Complete LSS workflow example
    println!("9. Complete LSS Workflow (Summary)");
    println!("===================================");
    println!("Typical LSS configuration workflow:");
    println!();
    println!("1. Switch to configuration mode (global or selective)");
    println!("   → lss_switch_state_global(LssMode::Configuration)");
    println!("   → OR lss_switch_state_selective(&address)");
    println!();
    println!("2. Verify slave is selected");
    println!("   → lss_identify_remote_slave(timeout)");
    println!();
    println!("3. Inquire current configuration");
    println!("   → lss_inquire_address(timeout)");
    println!("   → lss_inquire_node_id(timeout)");
    println!();
    println!("4. Configure new parameters");
    println!("   → lss_configure_node_id(new_id, timeout)");
    println!("   → lss_configure_bit_rate(selector, index, timeout)");
    println!("   → lss_activate_bit_rate(switch_delay)");
    println!();
    println!("5. Store configuration");
    println!("   → lss_store_configuration(timeout)");
    println!();
    println!("6. Switch back to operation mode");
    println!("   → lss_switch_state_global(LssMode::Operation)");
    println!();

    // Test 10: LSS Error codes demonstration
    println!("10. LSS Error Codes");
    println!("===================");
    println!("LSS operations return error codes:");
    println!();
    println!("  LssError::Success            - Operation successful");
    println!("  LssError::UnsupportedCommand - Command not supported by slave");
    println!("  LssError::MediaAccessFailure - CAN communication error");
    println!("  LssError::InvalidParameter   - Invalid parameter value");
    println!();

    println!("Cleaning up...");

    // Make sure we're back in operation mode
    canopen.lss_switch_state_global(LssMode::Operation).await?;

    // Disconnect
    canopen.disconnect().await?;
    println!("Disconnecting...");

    println!("\nLSS test completed successfully!");
    println!("\n⚠ IMPORTANT NOTES:");
    println!("   - LSS operations modify slave configuration");
    println!("   - Always verify slave address before configuring");
    println!("   - Bit-rate changes affect the entire CAN bus");
    println!("   - Store configuration to make changes permanent");
    println!("   - Some tests were disabled to prevent accidental changes");

    Ok(())
}
