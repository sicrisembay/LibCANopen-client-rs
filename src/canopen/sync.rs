use crate::canopen::message::CanMessage;
use log::{debug, trace, warn};
/// SYNC (Synchronization) Object Support
///
/// The SYNC object is used to synchronize PDO transmissions across the network.
/// SYNC messages are broadcast messages (COB-ID 0x80) that can include an optional counter.
///
/// SYNC is used for:
/// - Synchronous PDO transmission (devices transmit PDOs on SYNC)
/// - Network-wide synchronization of operations
/// - Coordinated timing across multiple nodes
use std::sync::Arc;
use std::sync::RwLock;

/// SYNC COB-ID (fixed at 0x80)
pub const SYNC_COB_ID: u16 = 0x80;

/// Type alias for SYNC callback
pub type SyncCallback = Arc<dyn Fn(u8) + Send + Sync>;

/// SYNC message manager
pub struct SyncManager {
    /// Current SYNC counter value (0-240, or 0 if counter not used)
    counter: Arc<RwLock<u8>>,

    /// Optional callback for SYNC reception
    sync_callback: Arc<RwLock<Option<SyncCallback>>>,

    /// Whether to use SYNC counter (standard allows 0-240)
    use_counter: Arc<RwLock<bool>>,
}

impl Default for SyncManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SyncManager {
    /// Create a new SYNC manager
    pub fn new() -> Self {
        Self {
            counter: Arc::new(RwLock::new(0)),
            sync_callback: Arc::new(RwLock::new(None)),
            use_counter: Arc::new(RwLock::new(false)),
        }
    }

    /// Enable or disable SYNC counter
    ///
    /// When enabled, SYNC messages will include a 1-byte counter (1-240, wrapping to 1)
    /// When disabled, SYNC messages have no data bytes
    pub fn set_counter_enabled(&self, enabled: bool) {
        let mut use_counter = self.use_counter.write().unwrap();
        *use_counter = enabled;

        if enabled {
            debug!("SYNC counter enabled");
        } else {
            debug!("SYNC counter disabled");
            // Reset counter when disabling
            let mut counter = self.counter.write().unwrap();
            *counter = 0;
        }
    }

    /// Get current SYNC counter value
    pub fn get_counter(&self) -> u8 {
        *self.counter.read().unwrap()
    }

    /// Generate a SYNC message to be transmitted
    ///
    /// Returns a CAN message with COB-ID 0x80 and optional counter byte
    pub fn create_sync_message(&self) -> CanMessage {
        let use_counter = *self.use_counter.read().unwrap();

        if use_counter {
            let mut counter = self.counter.write().unwrap();

            // Increment counter (1-240, wrap to 1)
            *counter = if *counter >= 240 { 1 } else { *counter + 1 };

            let counter_value = *counter;
            trace!("Generating SYNC message with counter: {}", counter_value);

            CanMessage::new(SYNC_COB_ID, vec![counter_value])
                .expect("SYNC message creation should never fail")
        } else {
            trace!("Generating SYNC message (no counter)");
            CanMessage::new(SYNC_COB_ID, vec![]).expect("SYNC message creation should never fail")
        }
    }

    /// Process an incoming SYNC message
    ///
    /// Extracts counter if present and invokes callback
    pub fn process_sync(&self, data: &[u8]) {
        let counter = if data.is_empty() {
            0
        } else if data.len() == 1 {
            data[0]
        } else {
            warn!("SYNC message has invalid length: {} bytes", data.len());
            return;
        };

        // Update our counter if using counter mode
        let use_counter = *self.use_counter.read().unwrap();
        if use_counter && counter > 0 {
            let mut current_counter = self.counter.write().unwrap();
            *current_counter = counter;
            trace!("Received SYNC with counter: {}", counter);
        } else if !use_counter && data.is_empty() {
            trace!("Received SYNC (no counter)");
        }

        // Invoke callback if registered
        let callback = self.sync_callback.read().unwrap();
        if let Some(ref cb) = *callback {
            cb(counter);
        }
    }

    /// Register a callback for SYNC reception
    ///
    /// The callback receives the SYNC counter value (0 if no counter present)
    ///
    /// # Example
    /// ```
    /// # use libcanopen_client::SyncManager;
    /// let sync_manager = SyncManager::new();
    /// sync_manager.register_sync_callback(|counter| {
    ///     println!("SYNC received: counter={}", counter);
    /// });
    /// ```
    pub fn register_sync_callback<F>(&self, callback: F)
    where
        F: Fn(u8) + Send + Sync + 'static,
    {
        let mut cb = self.sync_callback.write().unwrap();
        *cb = Some(Arc::new(callback));
        debug!("SYNC callback registered");
    }

    /// Unregister the SYNC callback
    pub fn unregister_sync_callback(&self) {
        let mut cb = self.sync_callback.write().unwrap();
        *cb = None;
        debug!("SYNC callback unregistered");
    }

    /// Reset SYNC counter to 0
    pub fn reset_counter(&self) {
        let mut counter = self.counter.write().unwrap();
        *counter = 0;
        debug!("SYNC counter reset");
    }
}

impl Clone for SyncManager {
    fn clone(&self) -> Self {
        Self {
            counter: Arc::clone(&self.counter),
            sync_callback: Arc::clone(&self.sync_callback),
            use_counter: Arc::clone(&self.use_counter),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_sync_cob_id_constant() {
        assert_eq!(SYNC_COB_ID, 0x80);
        assert_eq!(SYNC_COB_ID, 128); // Decimal value
    }

    #[test]
    fn test_sync_manager_new() {
        let sync = SyncManager::new();
        assert_eq!(sync.get_counter(), 0);

        // Default should be counter disabled
        let msg = sync.create_sync_message();
        assert_eq!(msg.data.len(), 0);
    }

    #[test]
    fn test_sync_counter_disabled() {
        let sync = SyncManager::new();

        let msg = sync.create_sync_message();
        assert_eq!(msg.id.raw(), SYNC_COB_ID);
        assert_eq!(msg.data.len(), 0);

        // Multiple SYNC messages should all be empty
        for _ in 0..10 {
            let msg = sync.create_sync_message();
            assert_eq!(msg.data.len(), 0);
            assert_eq!(msg.id.raw(), SYNC_COB_ID);
        }
    }

    #[test]
    fn test_sync_counter_enabled() {
        let sync = SyncManager::new();
        sync.set_counter_enabled(true);

        let msg1 = sync.create_sync_message();
        assert_eq!(msg1.data.len(), 1);
        assert_eq!(msg1.data[0], 1);

        let msg2 = sync.create_sync_message();
        assert_eq!(msg2.data.len(), 1);
        assert_eq!(msg2.data[0], 2);

        let msg3 = sync.create_sync_message();
        assert_eq!(msg3.data.len(), 1);
        assert_eq!(msg3.data[0], 3);
    }

    #[test]
    fn test_sync_counter_sequence() {
        let sync = SyncManager::new();
        sync.set_counter_enabled(true);

        // Test first 10 counts
        for expected in 1..=10 {
            let msg = sync.create_sync_message();
            assert_eq!(msg.data[0], expected);
            assert_eq!(sync.get_counter(), expected);
        }
    }

    #[test]
    fn test_sync_counter_wrap() {
        let sync = SyncManager::new();
        sync.set_counter_enabled(true);

        // Set counter to 240
        for _ in 0..240 {
            sync.create_sync_message();
        }

        assert_eq!(sync.get_counter(), 240);

        let msg = sync.create_sync_message();
        assert_eq!(msg.data[0], 1); // Should wrap to 1
        assert_eq!(sync.get_counter(), 1);

        let msg = sync.create_sync_message();
        assert_eq!(msg.data[0], 2); // Continue from 1
    }

    #[test]
    fn test_sync_counter_wrap_at_boundary() {
        let sync = SyncManager::new();
        sync.set_counter_enabled(true);

        // Generate 240 messages to reach the boundary
        for i in 1..=240 {
            let msg = sync.create_sync_message();
            assert_eq!(msg.data[0], i as u8);
        }

        // 241st message should wrap to 1
        let msg = sync.create_sync_message();
        assert_eq!(msg.data[0], 1, "After 240, should wrap to 1");

        // Continue and verify it keeps counting
        let msg = sync.create_sync_message();
        assert_eq!(msg.data[0], 2);

        let msg = sync.create_sync_message();
        assert_eq!(msg.data[0], 3);
    }

    #[test]
    fn test_sync_counter_enable_disable() {
        let sync = SyncManager::new();

        // Start disabled
        let msg = sync.create_sync_message();
        assert_eq!(msg.data.len(), 0);

        // Enable counter
        sync.set_counter_enabled(true);
        let msg = sync.create_sync_message();
        assert_eq!(msg.data.len(), 1);
        assert_eq!(msg.data[0], 1);

        let msg = sync.create_sync_message();
        assert_eq!(msg.data[0], 2);

        // Disable counter - should reset
        sync.set_counter_enabled(false);
        let msg = sync.create_sync_message();
        assert_eq!(msg.data.len(), 0);
        assert_eq!(sync.get_counter(), 0);

        // Re-enable - should start from 1 again
        sync.set_counter_enabled(true);
        let msg = sync.create_sync_message();
        assert_eq!(msg.data[0], 1);
    }

    #[test]
    fn test_sync_process_no_counter() {
        let sync = SyncManager::new();

        // Process SYNC without counter
        sync.process_sync(&[]);
        assert_eq!(sync.get_counter(), 0);
    }

    #[test]
    fn test_sync_process_with_counter() {
        let sync = SyncManager::new();
        sync.set_counter_enabled(true);

        // Process SYNC with counter
        sync.process_sync(&[5]);
        assert_eq!(sync.get_counter(), 5);

        sync.process_sync(&[10]);
        assert_eq!(sync.get_counter(), 10);

        sync.process_sync(&[240]);
        assert_eq!(sync.get_counter(), 240);
    }

    #[test]
    fn test_sync_process_invalid_length() {
        let sync = SyncManager::new();
        let initial_counter = sync.get_counter();

        // SYNC with more than 1 byte should be rejected
        sync.process_sync(&[1, 2]);
        // Counter should not change
        assert_eq!(sync.get_counter(), initial_counter);

        sync.process_sync(&[1, 2, 3]);
        assert_eq!(sync.get_counter(), initial_counter);
    }

    #[test]
    fn test_sync_callback_registration() {
        let sync = SyncManager::new();
        let received_values = Arc::new(Mutex::new(Vec::new()));
        let received_values_clone = Arc::clone(&received_values);

        sync.register_sync_callback(move |counter| {
            received_values_clone.lock().unwrap().push(counter);
        });

        sync.process_sync(&[]);
        sync.process_sync(&[5]);
        sync.process_sync(&[10]);

        let values = received_values.lock().unwrap();
        assert_eq!(*values, vec![0, 5, 10]);
    }

    #[test]
    fn test_sync_callback_unregister() {
        let sync = SyncManager::new();
        let call_count = Arc::new(Mutex::new(0));
        let call_count_clone = Arc::clone(&call_count);

        sync.register_sync_callback(move |_| {
            *call_count_clone.lock().unwrap() += 1;
        });

        sync.process_sync(&[]);
        assert_eq!(*call_count.lock().unwrap(), 1);

        // Unregister callback
        sync.unregister_sync_callback();

        sync.process_sync(&[]);
        // Count should not increase
        assert_eq!(*call_count.lock().unwrap(), 1);
    }

    #[test]
    fn test_sync_callback_with_counter() {
        let sync = SyncManager::new();
        let received_counters = Arc::new(Mutex::new(Vec::new()));
        let received_counters_clone = Arc::clone(&received_counters);

        sync.register_sync_callback(move |counter| {
            received_counters_clone.lock().unwrap().push(counter);
        });

        sync.set_counter_enabled(true);

        // Generate and process SYNC messages
        for i in 1..=5 {
            let msg = sync.create_sync_message();
            sync.process_sync(&msg.data);
            assert_eq!(sync.get_counter(), i);
        }

        let counters = received_counters.lock().unwrap();
        assert_eq!(*counters, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_sync_reset_counter() {
        let sync = SyncManager::new();
        sync.set_counter_enabled(true);

        // Generate some SYNC messages
        for _ in 0..10 {
            sync.create_sync_message();
        }
        assert_eq!(sync.get_counter(), 10);

        // Reset counter
        sync.reset_counter();
        assert_eq!(sync.get_counter(), 0);

        // Next message should start from 1
        let msg = sync.create_sync_message();
        assert_eq!(msg.data[0], 1);
    }

    #[test]
    fn test_sync_clone() {
        let sync1 = SyncManager::new();
        sync1.set_counter_enabled(true);
        sync1.create_sync_message(); // Counter = 1
        sync1.create_sync_message(); // Counter = 2

        let sync2 = sync1.clone();

        // Both should share the same counter
        assert_eq!(sync2.get_counter(), 2);

        // Incrementing one affects the other
        sync1.create_sync_message(); // Counter = 3
        assert_eq!(sync2.get_counter(), 3);

        sync2.create_sync_message(); // Counter = 4
        assert_eq!(sync1.get_counter(), 4);
    }

    #[test]
    fn test_sync_message_cob_id_always_0x80() {
        let sync = SyncManager::new();

        // Without counter
        let msg1 = sync.create_sync_message();
        assert_eq!(msg1.id.raw(), 0x80);

        // With counter
        sync.set_counter_enabled(true);
        for _ in 0..100 {
            let msg = sync.create_sync_message();
            assert_eq!(msg.id.raw(), 0x80);
        }
    }

    #[test]
    fn test_sync_counter_valid_range() {
        let sync = SyncManager::new();
        sync.set_counter_enabled(true);

        // Test full valid range (1-240)
        for expected in 1..=240 {
            let msg = sync.create_sync_message();
            assert_eq!(msg.data[0], expected);
            assert!(msg.data[0] >= 1 && msg.data[0] <= 240);
        }

        // Next should wrap to 1
        let msg = sync.create_sync_message();
        assert_eq!(msg.data[0], 1);
    }

    #[test]
    fn test_sync_multiple_callbacks() {
        let sync = SyncManager::new();
        let count1 = Arc::new(Mutex::new(0));
        let count1_clone = Arc::clone(&count1);

        // Register first callback
        sync.register_sync_callback(move |_| {
            *count1_clone.lock().unwrap() += 1;
        });

        sync.process_sync(&[]);
        assert_eq!(*count1.lock().unwrap(), 1);

        let count2 = Arc::new(Mutex::new(0));
        let count2_clone = Arc::clone(&count2);

        // Register second callback (replaces first)
        sync.register_sync_callback(move |_| {
            *count2_clone.lock().unwrap() += 1;
        });

        sync.process_sync(&[]);
        // First callback should not be called
        assert_eq!(*count1.lock().unwrap(), 1);
        // Second callback should be called
        assert_eq!(*count2.lock().unwrap(), 1);
    }

    #[test]
    fn test_sync_process_counter_zero() {
        let sync = SyncManager::new();
        sync.set_counter_enabled(true);

        // Process SYNC with counter = 0 (should update)
        sync.process_sync(&[0]);
        // Counter 0 is special - not updated when use_counter is true
        // (SYNC counter should be 1-240)

        // Process valid counter
        sync.process_sync(&[5]);
        assert_eq!(sync.get_counter(), 5);
    }

    #[test]
    fn test_sync_high_frequency_generation() {
        let sync = SyncManager::new();
        sync.set_counter_enabled(true);

        // Simulate high-frequency SYNC generation (e.g., 100 Hz)
        for i in 1..=1000 {
            let msg = sync.create_sync_message();
            assert_eq!(msg.id.raw(), SYNC_COB_ID);
            assert_eq!(msg.data.len(), 1);

            let expected_counter = ((i - 1) % 240) + 1;
            assert_eq!(msg.data[0], expected_counter as u8);
        }
    }
}
