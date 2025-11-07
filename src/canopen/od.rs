// CANopen Object Dictionary types and constants
use serde::{Deserialize, Serialize};

/// Object Dictionary Index type
pub type ObjectIndex = u16;

/// Object Dictionary Sub-index type  
pub type ObjectSubIndex = u8;

/// Object Dictionary Entry identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ObjectId {
    pub index: ObjectIndex,
    pub subindex: ObjectSubIndex,
}

impl ObjectId {
    pub fn new(index: ObjectIndex, subindex: ObjectSubIndex) -> Self {
        Self { index, subindex }
    }
}

impl std::fmt::Display for ObjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "0x{:04X}:{:02X}", self.index, self.subindex)
    }
}

/// CANopen data types as defined in CiA 301
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataType {
    Boolean = 0x01,
    Integer8 = 0x02,
    Integer16 = 0x03,
    Integer32 = 0x04,
    Unsigned8 = 0x05,
    Unsigned16 = 0x06,
    Unsigned32 = 0x07,
    Real32 = 0x08,
    VisibleString = 0x09,
    OctetString = 0x0A,
    UnicodeString = 0x0B,
    TimeOfDay = 0x0C,
    TimeDifference = 0x0D,
    Domain = 0x0F,
    Integer24 = 0x10,
    Real64 = 0x11,
    Integer40 = 0x12,
    Integer48 = 0x13,
    Integer56 = 0x14,
    Integer64 = 0x15,
    Unsigned24 = 0x16,
    Unsigned40 = 0x18,
    Unsigned48 = 0x19,
    Unsigned56 = 0x1A,
    Unsigned64 = 0x1B,
}

impl DataType {
    /// Get the size in bytes for fixed-size data types
    pub fn size_bytes(&self) -> Option<usize> {
        match self {
            DataType::Boolean => Some(1),
            DataType::Integer8 | DataType::Unsigned8 => Some(1),
            DataType::Integer16 | DataType::Unsigned16 => Some(2),
            DataType::Integer24 | DataType::Unsigned24 => Some(3),
            DataType::Integer32 | DataType::Unsigned32 | DataType::Real32 => Some(4),
            DataType::Integer40 | DataType::Unsigned40 => Some(5),
            DataType::Integer48 | DataType::Unsigned48 => Some(6),
            DataType::Integer56 | DataType::Unsigned56 => Some(7),
            DataType::Integer64 | DataType::Unsigned64 | DataType::Real64 => Some(8),
            DataType::TimeOfDay | DataType::TimeDifference => Some(6),
            // Variable length types
            DataType::VisibleString
            | DataType::OctetString
            | DataType::UnicodeString
            | DataType::Domain => None,
        }
    }

    /// Check if this is a string type
    pub fn is_string(&self) -> bool {
        matches!(self, DataType::VisibleString | DataType::UnicodeString)
    }

    /// Check if this is an integer type
    pub fn is_integer(&self) -> bool {
        matches!(
            self,
            DataType::Integer8
                | DataType::Integer16
                | DataType::Integer24
                | DataType::Integer32
                | DataType::Integer40
                | DataType::Integer48
                | DataType::Integer56
                | DataType::Integer64
        )
    }

    /// Check if this is an unsigned integer type
    pub fn is_unsigned(&self) -> bool {
        matches!(
            self,
            DataType::Unsigned8
                | DataType::Unsigned16
                | DataType::Unsigned24
                | DataType::Unsigned32
                | DataType::Unsigned40
                | DataType::Unsigned48
                | DataType::Unsigned56
                | DataType::Unsigned64
        )
    }

    /// Check if this is a floating point type
    pub fn is_real(&self) -> bool {
        matches!(self, DataType::Real32 | DataType::Real64)
    }
}

/// Standard CANopen Object Dictionary indices
pub mod standard_objects {
    use super::ObjectIndex;

    // Device Profile Objects (0x1000-0x1FFF)
    pub const DEVICE_TYPE: ObjectIndex = 0x1000;
    pub const ERROR_REGISTER: ObjectIndex = 0x1001;
    pub const MANUFACTURER_STATUS_REGISTER: ObjectIndex = 0x1002;
    pub const PRE_DEFINED_ERROR_FIELD: ObjectIndex = 0x1003;
    pub const COB_ID_SYNC: ObjectIndex = 0x1005;
    pub const COMMUNICATION_CYCLE_PERIOD: ObjectIndex = 0x1006;
    pub const SYNCHRONOUS_WINDOW_LENGTH: ObjectIndex = 0x1007;
    pub const MANUFACTURER_DEVICE_NAME: ObjectIndex = 0x1008;
    pub const MANUFACTURER_HARDWARE_VERSION: ObjectIndex = 0x1009;
    pub const MANUFACTURER_SOFTWARE_VERSION: ObjectIndex = 0x100A;
    pub const NODE_ID: ObjectIndex = 0x100B;
    pub const GUARD_TIME: ObjectIndex = 0x100C;
    pub const LIFE_TIME_FACTOR: ObjectIndex = 0x100D;
    pub const STORE_PARAMETERS: ObjectIndex = 0x1010;
    pub const RESTORE_DEFAULT_PARAMETERS: ObjectIndex = 0x1011;
    pub const COB_ID_TIME: ObjectIndex = 0x1012;
    pub const HIGH_RESOLUTION_TIME_STAMP: ObjectIndex = 0x1013;
    pub const COB_ID_EMERGENCY: ObjectIndex = 0x1014;
    pub const INHIBIT_TIME_EMERGENCY: ObjectIndex = 0x1015;
    pub const CONSUMER_HEARTBEAT_TIME: ObjectIndex = 0x1016;
    pub const PRODUCER_HEARTBEAT_TIME: ObjectIndex = 0x1017;
    pub const IDENTITY_OBJECT: ObjectIndex = 0x1018;

    // SDO Server Parameters (0x1200-0x127F)
    pub const SDO_SERVER_PARAMETER_BASE: ObjectIndex = 0x1200;

    // SDO Client Parameters (0x1280-0x12FF)
    pub const SDO_CLIENT_PARAMETER_BASE: ObjectIndex = 0x1280;

    // RPDO Communication Parameters (0x1400-0x15FF)
    pub const RPDO_COMMUNICATION_PARAMETER_BASE: ObjectIndex = 0x1400;

    // RPDO Mapping Parameters (0x1600-0x17FF)
    pub const RPDO_MAPPING_PARAMETER_BASE: ObjectIndex = 0x1600;

    // TPDO Communication Parameters (0x1800-0x19FF)
    pub const TPDO_COMMUNICATION_PARAMETER_BASE: ObjectIndex = 0x1800;

    // TPDO Mapping Parameters (0x1A00-0x1BFF)
    pub const TPDO_MAPPING_PARAMETER_BASE: ObjectIndex = 0x1A00;
}

/// Device profile specific objects
pub mod device_profile {
    use super::ObjectIndex;

    // Manufacturer Specific (0x2000-0x5FFF)
    pub const MANUFACTURER_SPECIFIC_START: ObjectIndex = 0x2000;
    pub const MANUFACTURER_SPECIFIC_END: ObjectIndex = 0x5FFF;

    // Standardized Device Profile (0x6000-0x9FFF)
    pub const DEVICE_PROFILE_START: ObjectIndex = 0x6000;
    pub const DEVICE_PROFILE_END: ObjectIndex = 0x9FFF;
}

/// Access types for Object Dictionary entries
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessType {
    ReadOnly,
    WriteOnly,
    ReadWrite,
    ReadWriteOnPreOp, // Read/Write only in Pre-Operational state
    ReadWriteOnSetup, // Read/Write only during setup
    Constant,
}

/// PDO mapping information
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PdoMapping {
    pub index: ObjectIndex,
    pub subindex: ObjectSubIndex,
    pub bit_length: u8,
}

impl PdoMapping {
    pub fn new(index: ObjectIndex, subindex: ObjectSubIndex, bit_length: u8) -> Self {
        Self {
            index,
            subindex,
            bit_length,
        }
    }

    /// Encode PDO mapping as 32-bit value
    pub fn encode(&self) -> u32 {
        ((self.index as u32) << 16) | ((self.subindex as u32) << 8) | (self.bit_length as u32)
    }

    /// Decode PDO mapping from 32-bit value
    pub fn decode(value: u32) -> Self {
        Self {
            index: ((value >> 16) & 0xFFFF) as u16,
            subindex: ((value >> 8) & 0xFF) as u8,
            bit_length: (value & 0xFF) as u8,
        }
    }
}

/// SDO abort codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SdoAbortCode {
    ToggleBitNotAlternated = 0x05030000,
    SdoProtocolTimeout = 0x05040000,
    CommandSpecifierInvalid = 0x05040001,
    InvalidBlockSize = 0x05040002,
    InvalidSequenceNumber = 0x05040003,
    CrcError = 0x05040004,
    OutOfMemory = 0x05040005,
    UnsupportedAccess = 0x06010000,
    AttemptReadWriteOnly = 0x06010001,
    AttemptWriteReadOnly = 0x06010002,
    ObjectNotExist = 0x06020000,
    ObjectCannotMappedPdo = 0x06040041,
    MappedObjectsExceedPdo = 0x06040042,
    GeneralParameterIncompatibility = 0x06040043,
    GeneralInternalIncompatibility = 0x06040047,
    HardwareFault = 0x06060000,
    TypeMismatch = 0x06070010,
    DataTypeLengthTooHigh = 0x06070012,
    DataTypeLengthTooLow = 0x06070013,
    SubIndexNotExist = 0x06090011,
    InvalidValue = 0x06090030,
    ValueTooHigh = 0x06090031,
    ValueTooLow = 0x06090032,
    MaxLessMin = 0x06090036,
    NoResourceAvailable = 0x060A0023,
    GeneralError = 0x08000000,
    DataCannotTransferred = 0x08000020,
    DataCannotTransferredLocalControl = 0x08000021,
    DataCannotTransferredDeviceState = 0x08000022,
    ObjectDictionaryNotPresent = 0x08000023,
    NoDataAvailable = 0x08000024,
}

impl SdoAbortCode {
    /// Convert to u32 for wire protocol
    pub fn to_u32(self) -> u32 {
        self as u32
    }

    /// Convert from u32 from wire protocol
    pub fn from_u32(code: u32) -> Option<Self> {
        match code {
            0x05030000 => Some(SdoAbortCode::ToggleBitNotAlternated),
            0x05040000 => Some(SdoAbortCode::SdoProtocolTimeout),
            0x05040001 => Some(SdoAbortCode::CommandSpecifierInvalid),
            0x05040002 => Some(SdoAbortCode::InvalidBlockSize),
            0x05040003 => Some(SdoAbortCode::InvalidSequenceNumber),
            0x05040004 => Some(SdoAbortCode::CrcError),
            0x05040005 => Some(SdoAbortCode::OutOfMemory),
            0x06010000 => Some(SdoAbortCode::UnsupportedAccess),
            0x06010001 => Some(SdoAbortCode::AttemptReadWriteOnly),
            0x06010002 => Some(SdoAbortCode::AttemptWriteReadOnly),
            0x06020000 => Some(SdoAbortCode::ObjectNotExist),
            0x06040041 => Some(SdoAbortCode::ObjectCannotMappedPdo),
            0x06040042 => Some(SdoAbortCode::MappedObjectsExceedPdo),
            0x06040043 => Some(SdoAbortCode::GeneralParameterIncompatibility),
            0x06040047 => Some(SdoAbortCode::GeneralInternalIncompatibility),
            0x06060000 => Some(SdoAbortCode::HardwareFault),
            0x06070010 => Some(SdoAbortCode::TypeMismatch),
            0x06070012 => Some(SdoAbortCode::DataTypeLengthTooHigh),
            0x06070013 => Some(SdoAbortCode::DataTypeLengthTooLow),
            0x06090011 => Some(SdoAbortCode::SubIndexNotExist),
            0x06090030 => Some(SdoAbortCode::InvalidValue),
            0x06090031 => Some(SdoAbortCode::ValueTooHigh),
            0x06090032 => Some(SdoAbortCode::ValueTooLow),
            0x06090036 => Some(SdoAbortCode::MaxLessMin),
            0x060A0023 => Some(SdoAbortCode::NoResourceAvailable),
            0x08000000 => Some(SdoAbortCode::GeneralError),
            0x08000020 => Some(SdoAbortCode::DataCannotTransferred),
            0x08000021 => Some(SdoAbortCode::DataCannotTransferredLocalControl),
            0x08000022 => Some(SdoAbortCode::DataCannotTransferredDeviceState),
            0x08000023 => Some(SdoAbortCode::ObjectDictionaryNotPresent),
            0x08000024 => Some(SdoAbortCode::NoDataAvailable),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_object_id() {
        let obj_id = ObjectId::new(0x1000, 0x00);
        assert_eq!(obj_id.index, 0x1000);
        assert_eq!(obj_id.subindex, 0x00);
        assert_eq!(obj_id.to_string(), "0x1000:00");
    }

    #[test]
    fn test_data_type_properties() {
        assert_eq!(DataType::Unsigned32.size_bytes(), Some(4));
        assert_eq!(DataType::VisibleString.size_bytes(), None);
        assert!(DataType::Integer32.is_integer());
        assert!(DataType::Unsigned16.is_unsigned());
        assert!(DataType::Real32.is_real());
        assert!(DataType::VisibleString.is_string());
    }

    #[test]
    fn test_pdo_mapping() {
        let mapping = PdoMapping::new(0x6000, 0x01, 16);
        let encoded = mapping.encode();
        let decoded = PdoMapping::decode(encoded);

        assert_eq!(mapping, decoded);
        assert_eq!(encoded, 0x60000110);
    }

    #[test]
    fn test_sdo_abort_codes() {
        let code = SdoAbortCode::ObjectNotExist;
        let raw = code.to_u32();
        let decoded = SdoAbortCode::from_u32(raw);

        assert_eq!(raw, 0x06020000);
        assert_eq!(decoded, Some(code));
    }
}
