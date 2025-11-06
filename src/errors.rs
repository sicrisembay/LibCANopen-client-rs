// Error types and Result alias for the library
use thiserror::Error;

/// Main error type for CANopen operations
#[derive(Error, Debug)]
pub enum CANopenError {
    #[error("Hardware error: {0}")]
    Hardware(String),
    
    #[error("SDO error: code {code:08X}")]
    Sdo { code: u32 },
    
    #[error("Timeout occurred")]
    Timeout,
    
    #[error("Invalid message format")]
    InvalidMessage,
    
    #[error("Node not found: {node_id}")]
    NodeNotFound { node_id: u8 },
    
    #[error("Connection error")]
    Connection,
    
    #[error("Queue full")]
    QueueFull,
    
    #[error("Channel closed")]
    ChannelClosed,
    
    #[error("PEAK CAN error: {0}")]
    PeakCan(String), // We'll handle peak-can-sys errors as strings for now
}

/// Convenience Result type alias
pub type Result<T> = std::result::Result<T, CANopenError>;