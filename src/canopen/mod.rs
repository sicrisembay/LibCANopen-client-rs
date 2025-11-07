// CANopen module - contains all CANopen protocol implementation

pub mod message;
pub mod nmt;
pub mod sdo;
pub mod pdo;
pub mod sync;
pub mod emcy;
pub mod lss;
pub mod events;
pub mod od;
// TODO: Fix builders module - temporarily disabled due to struct constructor issues
// pub mod builders;
pub mod types;

// Re-export common types
pub use message::{CanMessage, CanId, MessageType};
pub use sdo::{SdoClient, SdoDirection, SdoRequest, SdoTransfer, SdoState};
pub use nmt::{NmtManager, NmtState, NodeState};
pub use events::{EventManager, MessageEvent};
pub use pdo::PdoManager;
pub use sync::SyncManager;
pub use emcy::{EmcyManager, EmergencyMessage};
pub use lss::{LssManager, LssAddress, LssCommand, LssError, LssMode, LssResponse};
pub use od::{ObjectId, DataType};
// TODO: Re-enable when builders are fixed
// pub use builders::{
//     NmtMessageBuilder, SdoClientMessageBuilder, SdoServerMessageBuilder,
//     EmergencyMessageBuilder, PdoMessageBuilder, SyncMessageBuilder
// };
