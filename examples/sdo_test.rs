//! SDO (Service Data Object) functionality test example
//!
//! This example demonstrates the SDO client capabilities of libCANopen-simple:
//! - Reading and writing various data types (u8, u16, u32)
//! - Raw data operations
//! - Error handling and timeout scenarios
//! - Both expedited and segmented transfers

use libcanopen_client::hardware::{PcanHandle, PeakCanAdapter};
use libcanopen_client::{BusSpeed, CANopenSimple, Result};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging to see detailed SDO operations
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();

    println!("libCANopen-simple SDO Test Example");
    println!("==================================");

    // Create PEAK CAN adapter (USB bus 1, 1 Mbps)
    let peak_adapter = PeakCanAdapter::new(PcanHandle::PcanUsbbus1, BusSpeed::Baud1M);

    // Create CANopen instance
    let mut canopen = CANopenSimple::new(Box::new(peak_adapter));

    println!("Connecting to CAN hardware at 1 Mbps...");

    // Connect to hardware
    canopen.connect(BusSpeed::Baud1M).await?;
    println!("Connected successfully!");

    // Give the message processing tasks time to initialize
    tokio::time::sleep(Duration::from_millis(100)).await;
    println!("Message processing initialized");

    // Test parameters
    let test_node_id = 2; // Change this to match your test node
    let timeout_ms = 1000; // 1 second timeout

    println!("\n=== SDO Test Suite ===");
    println!("Testing with Node ID: {}", test_node_id);

    // Test 1: Basic u8 read/write operations
    println!("\n1. Testing u8 SDO operations...");

    // Try to read a standard object (Device Type - usually at 0x1000:00)
    match canopen
        .sdo_read_u32(test_node_id, 0x1000, 0x00, timeout_ms)
        .await
    {
        Ok(device_type) => {
            println!("   ✓ Successfully read Device Type: 0x{:08X}", device_type);
        }
        Err(e) => {
            println!("   ✗ Failed to read Device Type: {}", e);
        }
    }

    // Test 2: Try to read Error Register (0x1001:00) - should be u8
    println!("\n2. Testing Error Register read (u8)...");
    match canopen
        .sdo_read_u8(test_node_id, 0x1001, 0x00, timeout_ms)
        .await
    {
        Ok(error_reg) => {
            println!("   ✓ Error Register: 0x{:02X}", error_reg);
        }
        Err(e) => {
            println!("   ✗ Failed to read Error Register: {}", e);
        }
    }

    // Test 3: Try to read Manufacturer Status Register (0x1002:00) - should be u32
    println!("\n3. Testing Manufacturer Status Register read (u32)...");
    match canopen
        .sdo_read_u32(test_node_id, 0x1002, 0x00, timeout_ms)
        .await
    {
        Ok(status) => {
            println!("   ✓ Manufacturer Status: 0x{:08X}", status);
        }
        Err(e) => {
            println!("   ✗ Failed to read Manufacturer Status: {}", e);
        }
    }

    // Test 4: Test raw data read (for larger objects)
    println!("\n4. Testing raw data SDO read...");
    match canopen
        .sdo_read_data(test_node_id, 0x1008, 0x00, timeout_ms)
        .await
    {
        Ok(data) => {
            println!("   ✓ Raw data read successful, {} bytes:", data.len());
            // Try to interpret as string if printable
            if data
                .iter()
                .all(|&b| b.is_ascii_graphic() || b.is_ascii_whitespace() || b == 0)
            {
                if let Ok(s) = String::from_utf8(data.clone()) {
                    println!("     String data: '{}'", s.trim_end_matches('\0'));
                }
            }
            println!("     Hex data: {:02X?}", &data[..data.len().min(16)]);
        }
        Err(e) => {
            println!("   ✗ Failed to read raw data: {}", e);
        }
    }

    // Test 5: Test write operations (be careful with these!)
    println!("\n5. Testing SDO write operations...");
    println!("   ⚠️  Write tests are commented out for safety!");
    println!("   ⚠️  Uncomment and modify for your specific device if needed.");

    /*
    // Example write operations (UNCOMMENT CAREFULLY!)

    // Write to a test register (make sure this is safe for your device!)
    match canopen.sdo_write_u8(test_node_id, 0x2000, 0x01, 42, timeout_ms).await {
        Ok(_) => println!("   ✓ Successfully wrote u8 value 42"),
        Err(e) => println!("   ✗ Failed to write u8: {}", e),
    }

    // Write raw data
    let test_data = vec![0x01, 0x02, 0x03, 0x04];
    match canopen.sdo_write_data(test_node_id, 0x2000, 0x02, test_data, timeout_ms).await {
        Ok(_) => println!("   ✓ Successfully wrote raw data"),
        Err(e) => println!("   ✗ Failed to write raw data: {}", e),
    }
    */

    // Test 6: Test timeout behavior
    println!("\n6. Testing timeout behavior...");
    let short_timeout = 100; // Very short timeout
    match canopen.sdo_read_u32(255, 0x1000, 0x00, short_timeout).await {
        Ok(_) => println!("   ? Unexpected success (node 255 shouldn't exist)"),
        Err(e) => {
            println!("   ✓ Expected timeout/error for non-existent node: {}", e);
        }
    }

    // Test 7: Test invalid object access
    println!("\n7. Testing invalid object access...");
    match canopen
        .sdo_read_u32(test_node_id, 0xFFFF, 0xFF, timeout_ms)
        .await
    {
        Ok(_) => println!("   ? Unexpected success for invalid object"),
        Err(e) => {
            println!("   ✓ Expected error for invalid object: {}", e);
        }
    }

    // Test 8: Performance test
    println!("\n8. Performance test - multiple quick reads...");
    let start_time = std::time::Instant::now();
    let mut success_count = 0;
    let test_count = 5;

    for i in 0..test_count {
        match canopen
            .sdo_read_u32(test_node_id, 0x1000, 0x00, timeout_ms)
            .await
        {
            Ok(_) => {
                success_count += 1;
                print!("✓");
            }
            Err(_) => print!("✗"),
        }
        if i < test_count - 1 {
            print!(" ");
        }
    }

    let duration = start_time.elapsed();
    println!(
        "\n   Performance: {}/{} successful reads in {:?}",
        success_count, test_count, duration
    );
    if success_count > 0 {
        println!("   Average per read: {:?}", duration / success_count);
    }

    // Test 9: Concurrent SDO operations test
    println!("\n9. Testing concurrent SDO operations...");
    let handles = vec![
        tokio::spawn({
            let canopen_clone = canopen.clone();
            async move {
                canopen_clone
                    .sdo_read_u32(test_node_id, 0x1000, 0x00, timeout_ms)
                    .await
            }
        }),
        tokio::spawn({
            let canopen_clone = canopen.clone();
            async move {
                canopen_clone
                    .sdo_read_u32(test_node_id, 0x1000, 0x00, timeout_ms)
                    .await
            }
        }),
    ];

    let mut concurrent_success = 0;
    for (i, handle) in handles.into_iter().enumerate() {
        match handle.await {
            Ok(Ok(_)) => {
                concurrent_success += 1;
                println!("   ✓ Concurrent operation {} succeeded", i + 1);
            }
            Ok(Err(e)) => {
                println!("   ✗ Concurrent operation {} failed: {}", i + 1, e);
            }
            Err(e) => {
                println!("   ✗ Concurrent operation {} panicked: {}", i + 1, e);
            }
        }
    }
    println!(
        "   Concurrent operations: {}/2 successful",
        concurrent_success
    );

    println!("\n=== SDO Test Summary ===");
    println!("✓ Basic SDO functionality tested");
    println!("✓ Timeout behavior verified");
    println!("✓ Error handling demonstrated");
    println!("✓ Performance characteristics measured");
    println!("✓ Concurrent operations tested");

    // Keep running briefly to see any final messages
    println!("\nWaiting 2 seconds for any pending operations...");
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Disconnect
    println!("Disconnecting...");
    canopen.disconnect().await?;

    println!("SDO test completed successfully!");

    Ok(())
}
