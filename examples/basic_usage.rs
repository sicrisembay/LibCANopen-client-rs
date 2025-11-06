// Basic usage example for libCANopen Simple
use libcanopen_simple::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();

    // Create PEAK CAN adapter
    let peak_adapter = PeakCanAdapter::new(
        PcanHandle::PcanUsbbus1, 
        BusSpeed::Baud250K
    );

    // Create CANopen instance
    let mut canopen = CANopenSimple::new(Box::new(peak_adapter));

    println!("Connecting to CAN hardware...");
    
    // Connect to hardware
    canopen.connect().await?;

    println!("Connected! Setting up event subscriptions...");

    // Subscribe to events
    let mut packet_rx = canopen.subscribe_packets();
    
    // Spawn a task to handle incoming messages
    tokio::spawn(async move {
        while let Ok(event) = packet_rx.recv().await {
            println!("Received: {} with {} bytes at {:?}", 
                event.message.id, 
                event.message.data.len(),
                event.timestamp
            );
        }
    });

    println!("Sending test messages...");

    // Send NMT start to all nodes
    canopen.nmt_start(0).await?;
    println!("Sent NMT start command");

    // Send a test PDO
    canopen.write_pdo(0x181, vec![0x01, 0x02, 0x03, 0x04]).await?;
    println!("Sent test PDO");

    // Keep running for a few seconds
    println!("Running for 5 seconds...");
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // Disconnect
    println!("Disconnecting...");
    canopen.disconnect().await?;
    
    println!("Example completed successfully!");

    Ok(())
}