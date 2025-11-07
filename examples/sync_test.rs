/// SYNC (Synchronization) Test Example
/// 
/// This example demonstrates:
/// - Sending SYNC messages with and without counter
/// - Receiving SYNC messages from other nodes
/// - Using SYNC callbacks for synchronization

use libcanopen_simple::*;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .init();
    
    println!("libCANopen-simple SYNC Test Example");
    println!("====================================\n");
    
    // Create PEAK CAN adapter
    println!("Connecting to CAN hardware at 1 Mbps...");
    let peak_adapter = PeakCanAdapter::new(
        PcanHandle::PcanUsbbus1,
        BusSpeed::Baud1M
    );
    
    // Create CANopen instance
    let mut canopen = CANopenSimple::new(Box::new(peak_adapter));
    
    // Connect to hardware
    canopen.connect(BusSpeed::Baud1M).await?;
    println!("Connected successfully!\n");
    
    // Test 1: SYNC transmission without counter
    println!("1. SYNC Without Counter Test");
    println!("=============================");
    println!("Sending 5 SYNC messages (no counter)...");
    
    for i in 1..=5 {
        canopen.send_sync().await?;
        println!("  ✓ SYNC #{} sent", i);
        sleep(Duration::from_millis(100)).await;
    }
    println!();
    
    // Test 2: SYNC transmission with counter
    println!("2. SYNC With Counter Test");
    println!("==========================");
    println!("Enabling SYNC counter...");
    canopen.set_sync_counter_enabled(true).await?;
    
    println!("Sending 10 SYNC messages with counter...");
    for i in 1..=10 {
        canopen.send_sync().await?;
        let counter = canopen.get_sync_counter().await?;
        println!("  ✓ SYNC #{}: counter={}", i, counter);
        sleep(Duration::from_millis(100)).await;
    }
    println!();
    
    // Test 3: SYNC reception callback
    println!("3. SYNC Reception Test");
    println!("======================");
    
    let sync_count = Arc::new(AtomicU32::new(0));
    let sync_count_clone = Arc::clone(&sync_count);
    
    canopen.register_sync_callback(move |counter| {
        sync_count_clone.fetch_add(1, Ordering::SeqCst);
        if counter > 0 {
            println!("  [CALLBACK] SYNC received with counter: {}", counter);
        } else {
            println!("  [CALLBACK] SYNC received (no counter)");
        }
    }).await?;
    
    println!("Registered SYNC callback");
    println!("Listening for SYNC messages for 5 seconds...");
    println!("(Will also count our own transmitted SYNCs)");
    
    // Send some SYNCs to test the callback
    for _ in 0..5 {
        canopen.send_sync().await?;
        sleep(Duration::from_millis(1000)).await;
    }
    
    let received_count = sync_count.load(Ordering::SeqCst);
    println!("\nTotal SYNC messages processed: {}", received_count);
    
    // Unregister callback
    canopen.unregister_sync_callback().await?;
    println!("✓ SYNC callback unregistered\n");
    
    // Test 4: Counter wrap-around
    println!("4. Counter Wrap-Around Test");
    println!("============================");
    println!("Sending 250 SYNC messages to test counter wrap...");
    println!("(This will take ~25 seconds)");
    
    canopen.set_sync_counter_enabled(true).await?;
    
    let mut wrapped = false;
    for i in 1..=250 {
        canopen.send_sync().await?;
        let counter = canopen.get_sync_counter().await?;
        
        if counter == 1 && i > 1 {
            wrapped = true;
            println!("  ✓ Counter wrapped to 1 at SYNC #{}", i);
        }
        
        // Print progress
        if i % 50 == 0 {
            println!("    Progress: {}/250 SYNCs sent, counter={}", i, counter);
        }
        
        sleep(Duration::from_millis(100)).await;
    }
    
    if wrapped {
        println!("✓ Counter wrap-around verified (1-240, then wraps to 1)\n");
    } else {
        println!("⚠ Counter did not wrap (expected if <240 SYNCs sent)\n");
    }
    
    // Test 5: Periodic SYNC generation
    println!("5. Periodic SYNC Generation Test");
    println!("=================================");
    println!("Simulating periodic SYNC at 10ms intervals (100 Hz)");
    println!("Duration: 3 seconds (300 SYNCs)");
    
    canopen.set_sync_counter_enabled(true).await?;
    
    let start = std::time::Instant::now();
    let mut sync_sent = 0;
    
    while start.elapsed() < Duration::from_secs(3) {
        canopen.send_sync().await?;
        sync_sent += 1;
        sleep(Duration::from_millis(10)).await;
    }
    
    let elapsed = start.elapsed();
    let actual_rate = sync_sent as f64 / elapsed.as_secs_f64();
    
    println!("✓ Sent {} SYNCs in {:.2}s", sync_sent, elapsed.as_secs_f64());
    println!("  Average rate: {:.1} Hz (target: 100 Hz)\n", actual_rate);
    
    // Test 6: Disable counter
    println!("6. Disable Counter Test");
    println!("=======================");
    println!("Disabling SYNC counter...");
    canopen.set_sync_counter_enabled(false).await?;
    
    println!("Sending 3 SYNCs without counter...");
    for i in 1..=3 {
        canopen.send_sync().await?;
        let counter = canopen.get_sync_counter().await?;
        println!("  ✓ SYNC #{} sent, counter={} (should be 0)", i, counter);
        sleep(Duration::from_millis(100)).await;
    }
    
    println!();
    println!("Cleaning up...");
    
    // Disconnect
    canopen.disconnect().await?;
    println!("Disconnecting...");
    
    println!("\nSYNC test completed successfully!");
    
    Ok(())
}
