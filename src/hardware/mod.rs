// Hardware abstraction layer
use async_trait::async_trait;
use tokio::sync::mpsc;
use crate::canopen::message::CanMessage;
use crate::Result;

/// Hardware abstraction trait for CAN adapters
#[async_trait]
pub trait CanHardware {
    /// Connect to the CAN hardware
    async fn connect(&mut self) -> Result<()>;
    
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