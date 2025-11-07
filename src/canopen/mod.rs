// CANopen module - contains all CANopen protocol implementation

pub mod emcy;
pub mod events;
pub mod lss;
pub mod message;
pub mod nmt;
pub mod od;
pub mod pdo;
pub mod sdo;
pub mod sync;
// TODO: Fix builders module - temporarily disabled due to struct constructor issues
// pub mod builders;
pub mod types;

// Re-export common types
pub use emcy::{EmcyManager, EmergencyMessage};
pub use events::{EventManager, MessageEvent};
pub use lss::{LssAddress, LssCommand, LssError, LssManager, LssMode, LssResponse};
pub use message::{CanId, CanMessage, MessageType};
pub use nmt::{NmtManager, NmtState, NodeState};
pub use od::{DataType, ObjectId};
pub use pdo::PdoManager;
pub use sdo::{SdoClient, SdoDirection, SdoRequest, SdoState, SdoTransfer};
pub use sync::SyncManager;
// TODO: Re-enable when builders are fixed
// pub use builders::{
//     NmtMessageBuilder, SdoClientMessageBuilder, SdoServerMessageBuilder,
//     EmergencyMessageBuilder, PdoMessageBuilder, SyncMessageBuilder
// };
