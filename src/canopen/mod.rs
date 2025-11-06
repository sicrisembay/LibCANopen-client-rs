// CANopen module - contains all CANopen protocol implementation

pub mod message;
pub mod sdo;
pub mod nmt;
pub mod pdo;
pub mod events;

// Re-export common types
pub use message::{CanMessage, CanId, MessageType};
pub use sdo::{SdoClient, SdoDirection, SdoTransfer, SdoState};
pub use nmt::{NmtManager, NmtState, NodeState};
pub use events::{EventManager, MessageEvent};
pub use pdo::PdoManager;