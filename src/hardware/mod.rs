// Hardware abstraction layer
use crate::canopen::message::CanMessage;
use crate::Result;
use async_trait::async_trait;
use tokio::sync::mpsc;

/// Bus speed enumeration
#[derive(Debug, Clone, Copy)]
pub enum BusSpeed {
    Baud125K = 125_000,
    Baud250K = 250_000,
    Baud500K = 500_000,
    Baud1M = 1_000_000,
}

/// Hardware abstraction trait for CAN adapters
#[async_trait]
pub trait CanHardware {
    /// Connect to the CAN hardware with specified bus speed
    async fn connect(&mut self, bus_speed: BusSpeed) -> Result<()>;

    /// Disconnect from the CAN hardware
    async fn disconnect(&mut self) -> Result<()>;

    /// Send a CAN message
    async fn send_message(&self, message: &CanMessage) -> Result<()>;

    /// Subscribe to received messages
    fn subscribe_messages(&self) -> mpsc::Receiver<CanMessage>;

    /// Check if the hardware is connected
    fn is_connected(&self) -> bool;
}

pub mod peak_can;

// Re-export commonly used types
pub use peak_can::{PcanHandle, PeakCanAdapter};
