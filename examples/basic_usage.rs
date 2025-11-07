// Basic usage example for libCANopen Simple
use libcanopen_simple::{CANopenSimple, BusSpeed, Result};
use libcanopen_simple::hardware::peak_can::{PeakCanAdapter, PcanHandle};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();

    println!("libCANopen-simple Basic Usage Example");
    println!("=====================================");

    // Create PEAK CAN adapter (handle and speed)
    let peak_adapter = PeakCanAdapter::new(PcanHandle::PcanUsbbus1, BusSpeed::Baud250K);

    // Create CANopen instance
    let mut canopen = CANopenSimple::new(Box::new(peak_adapter));

    println!("Connecting to CAN hardware at 250 kbps...");
    
    // Connect to hardware with specified bus speed
    canopen.connect(BusSpeed::Baud250K).await?;

    println!("Connected successfully!");

    // Check connection status
    println!("Connection status: {}", canopen.is_connected().await);

    println!("Sending test messages...");

    // Send NMT start to all nodes
    canopen.nmt_start(0).await?;
    println!("Sent NMT start command");

    // Send a test PDO
    canopen.write_pdo(0x181, vec![0x01, 0x02, 0x03, 0x04]).await?;
    println!("Sent test PDO");

    // Keep running for a few seconds
    println!("Running for 3 seconds...");
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Disconnect
    println!("Disconnecting...");
    canopen.disconnect().await?;
    
    println!("Example completed successfully!");

    Ok(())
}