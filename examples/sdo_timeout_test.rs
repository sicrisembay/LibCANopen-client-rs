/// SDO Timeout Error Handling Test
///
/// This example verifies that SDO write/read operations properly return errors
/// when they timeout instead of incorrectly returning Ok(()).
///
/// Tests:
/// 1. Write to non-existent node - should return Err(Timeout)
/// 2. Read from non-existent node - should return Err(Timeout)
use libcanopen_client::*;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    println!("SDO Timeout Error Handling Test");
    println!("================================\n");

    // Create PEAK CAN adapter
    println!("Connecting to CAN hardware at 1 Mbps...");
    let peak_adapter = PeakCanAdapter::new(PcanHandle::PcanUsbbus1, BusSpeed::Baud1M);

    // Create CANopen instance
    let mut canopen = CANopenSimple::new(Box::new(peak_adapter));

    // Connect to hardware
    canopen.connect(BusSpeed::Baud1M).await?;
    println!("Connected successfully!\n");

    // Test 1: Write to non-existent node (should timeout and return error)
    println!("Test 1: SDO Write Timeout Handling");
    println!("-----------------------------------");
    println!("Attempting to write to non-existent Node 99...");
    println!("(Writing value 1000 to index 0x1017, subindex 0 with 1000ms timeout)");

    match canopen.sdo_write_u16(99, 0x1017, 0, 1000, 1000).await {
        Ok(()) => {
            println!("❌ FAIL: Function returned Ok(()) when it should have returned an error!");
            println!("   This is the bug we're trying to fix.");
        }
        Err(CANopenError::Timeout) => {
            println!("✓ PASS: Function correctly returned Err(CANopenError::Timeout)");
            println!("   Error handling is working properly!");
        }
        Err(e) => {
            println!("✓ PASS: Function returned error: {:?}", e);
            println!("   (This is acceptable - any error is better than Ok)");
        }
    }

    println!();

    // Test 2: Read from non-existent node (should timeout and return error)
    println!("Test 2: SDO Read Timeout Handling");
    println!("----------------------------------");
    println!("Attempting to read from non-existent Node 99...");
    println!("(Reading from index 0x1017, subindex 0 with 1000ms timeout)");

    match canopen.sdo_read_u16(99, 0x1017, 0, 1000).await {
        Ok(value) => {
            println!(
                "❌ FAIL: Function returned Ok({}) when it should have returned an error!",
                value
            );
        }
        Err(CANopenError::Timeout) => {
            println!("✓ PASS: Function correctly returned Err(CANopenError::Timeout)");
            println!("   Error handling is working properly!");
        }
        Err(e) => {
            println!("✓ PASS: Function returned error: {:?}", e);
        }
    }

    println!();

    // Test 3: Write to existing node with invalid index (should return SDO abort)
    println!("Test 3: SDO Abort Error Handling");
    println!("---------------------------------");
    println!("Attempting to write to invalid index 0xFFFF on Node 1...");

    match canopen.sdo_write_u16(1, 0xFFFF, 0, 1000, 1000).await {
        Ok(()) => {
            println!("⚠ Unexpected: Write succeeded (index might actually exist)");
        }
        Err(CANopenError::Sdo { code }) => {
            println!("✓ PASS: Function correctly returned SDO abort error");
            println!("   Abort code: 0x{:08X}", code);
        }
        Err(CANopenError::Timeout) => {
            println!("✓ PASS: Function returned timeout (node might not be responding)");
        }
        Err(e) => {
            println!("✓ PASS: Function returned error: {:?}", e);
        }
    }

    println!();

    // Disconnect
    canopen.disconnect().await?;
    println!("Disconnected");

    println!("\n================================");
    println!("Test Summary");
    println!("================================");
    println!("If all tests show ✓ PASS, the error handling is working correctly.");
    println!("If any test shows ❌ FAIL, there's still a bug in error propagation.");

    Ok(())
}
