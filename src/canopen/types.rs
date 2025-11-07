// Data conversion utilities for CANopen data types

use crate::canopen::{DataType, ObjectId};
use crate::{CANopenError, Result};

/// Trait for converting Rust types to/from CANopen wire format
pub trait CANopenData: Sized {
    /// Get the CANopen data type for this Rust type
    fn canopen_data_type() -> DataType;

    /// Convert from CANopen wire format bytes
    fn from_canopen_bytes(data: &[u8]) -> Result<Self>;

    /// Convert to CANopen wire format bytes
    fn to_canopen_bytes(&self) -> Vec<u8>;

    /// Get the expected size in bytes for this type
    fn expected_size() -> Option<usize> {
        Self::canopen_data_type().size_bytes()
    }
}

/// Implement CANopenData for basic types
impl CANopenData for bool {
    fn canopen_data_type() -> DataType {
        DataType::Boolean
    }

    fn from_canopen_bytes(data: &[u8]) -> Result<Self> {
        if data.is_empty() {
            return Err(CANopenError::InvalidData(
                "Empty data for boolean".to_string(),
            ));
        }
        Ok(data[0] != 0)
    }

    fn to_canopen_bytes(&self) -> Vec<u8> {
        vec![if *self { 1 } else { 0 }]
    }
}

impl CANopenData for u8 {
    fn canopen_data_type() -> DataType {
        DataType::Unsigned8
    }

    fn from_canopen_bytes(data: &[u8]) -> Result<Self> {
        if data.is_empty() {
            return Err(CANopenError::InvalidData("Empty data for u8".to_string()));
        }
        Ok(data[0])
    }

    fn to_canopen_bytes(&self) -> Vec<u8> {
        vec![*self]
    }
}

impl CANopenData for i8 {
    fn canopen_data_type() -> DataType {
        DataType::Integer8
    }

    fn from_canopen_bytes(data: &[u8]) -> Result<Self> {
        if data.is_empty() {
            return Err(CANopenError::InvalidData("Empty data for i8".to_string()));
        }
        Ok(data[0] as i8)
    }

    fn to_canopen_bytes(&self) -> Vec<u8> {
        vec![*self as u8]
    }
}

impl CANopenData for u16 {
    fn canopen_data_type() -> DataType {
        DataType::Unsigned16
    }

    fn from_canopen_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < 2 {
            return Err(CANopenError::InvalidData(
                "Insufficient data for u16".to_string(),
            ));
        }
        Ok(u16::from_le_bytes([data[0], data[1]]))
    }

    fn to_canopen_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }
}

impl CANopenData for i16 {
    fn canopen_data_type() -> DataType {
        DataType::Integer16
    }

    fn from_canopen_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < 2 {
            return Err(CANopenError::InvalidData(
                "Insufficient data for i16".to_string(),
            ));
        }
        Ok(i16::from_le_bytes([data[0], data[1]]))
    }

    fn to_canopen_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }
}

impl CANopenData for u32 {
    fn canopen_data_type() -> DataType {
        DataType::Unsigned32
    }

    fn from_canopen_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < 4 {
            return Err(CANopenError::InvalidData(
                "Insufficient data for u32".to_string(),
            ));
        }
        Ok(u32::from_le_bytes([data[0], data[1], data[2], data[3]]))
    }

    fn to_canopen_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }
}

impl CANopenData for i32 {
    fn canopen_data_type() -> DataType {
        DataType::Integer32
    }

    fn from_canopen_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < 4 {
            return Err(CANopenError::InvalidData(
                "Insufficient data for i32".to_string(),
            ));
        }
        Ok(i32::from_le_bytes([data[0], data[1], data[2], data[3]]))
    }

    fn to_canopen_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }
}

impl CANopenData for u64 {
    fn canopen_data_type() -> DataType {
        DataType::Unsigned64
    }

    fn from_canopen_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < 8 {
            return Err(CANopenError::InvalidData(
                "Insufficient data for u64".to_string(),
            ));
        }
        Ok(u64::from_le_bytes([
            data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
        ]))
    }

    fn to_canopen_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }
}

impl CANopenData for i64 {
    fn canopen_data_type() -> DataType {
        DataType::Integer64
    }

    fn from_canopen_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < 8 {
            return Err(CANopenError::InvalidData(
                "Insufficient data for i64".to_string(),
            ));
        }
        Ok(i64::from_le_bytes([
            data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
        ]))
    }

    fn to_canopen_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }
}

impl CANopenData for f32 {
    fn canopen_data_type() -> DataType {
        DataType::Real32
    }

    fn from_canopen_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < 4 {
            return Err(CANopenError::InvalidData(
                "Insufficient data for f32".to_string(),
            ));
        }
        Ok(f32::from_le_bytes([data[0], data[1], data[2], data[3]]))
    }

    fn to_canopen_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }
}

impl CANopenData for f64 {
    fn canopen_data_type() -> DataType {
        DataType::Real64
    }

    fn from_canopen_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < 8 {
            return Err(CANopenError::InvalidData(
                "Insufficient data for f64".to_string(),
            ));
        }
        Ok(f64::from_le_bytes([
            data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
        ]))
    }

    fn to_canopen_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }
}

impl CANopenData for String {
    fn canopen_data_type() -> DataType {
        DataType::VisibleString
    }

    fn from_canopen_bytes(data: &[u8]) -> Result<Self> {
        String::from_utf8(data.to_vec())
            .map_err(|e| CANopenError::InvalidData(format!("Invalid UTF-8 string: {}", e)))
    }

    fn to_canopen_bytes(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }

    fn expected_size() -> Option<usize> {
        None // Variable length
    }
}

impl CANopenData for Vec<u8> {
    fn canopen_data_type() -> DataType {
        DataType::OctetString
    }

    fn from_canopen_bytes(data: &[u8]) -> Result<Self> {
        Ok(data.to_vec())
    }

    fn to_canopen_bytes(&self) -> Vec<u8> {
        self.clone()
    }

    fn expected_size() -> Option<usize> {
        None // Variable length
    }
}

/// Helper functions for data type validation and conversion
pub struct DataTypeConverter;

impl DataTypeConverter {
    /// Validate that data matches the expected data type
    pub fn validate_data_type(data: &[u8], expected_type: DataType) -> Result<()> {
        if let Some(expected_size) = expected_type.size_bytes() {
            if data.len() != expected_size {
                return Err(CANopenError::InvalidData(format!(
                    "Data size mismatch: expected {} bytes for {:?}, got {}",
                    expected_size,
                    expected_type,
                    data.len()
                )));
            }
        }

        match expected_type {
            DataType::VisibleString => {
                // Validate that all bytes are printable ASCII
                for &byte in data {
                    if !(0x20..=0x7E).contains(&byte) {
                        return Err(CANopenError::InvalidData(
                            "VisibleString contains non-printable characters".to_string(),
                        ));
                    }
                }
            }
            _ => {} // Other types don't need special validation
        }

        Ok(())
    }

    /// Convert generic data bytes to a specific Rust type
    pub fn convert_to_type<T: CANopenData>(data: &[u8]) -> Result<T> {
        Self::validate_data_type(data, T::canopen_data_type())?;
        T::from_canopen_bytes(data)
    }

    /// Convert a Rust type to CANopen bytes with validation
    pub fn convert_from_type<T: CANopenData>(value: &T) -> Vec<u8> {
        value.to_canopen_bytes()
    }

    /// Get the minimum number of bytes needed to represent a value
    pub fn get_minimal_encoding_size(data: &[u8], data_type: DataType) -> usize {
        match data_type {
            // For integers, find the minimal representation
            DataType::Unsigned16 | DataType::Integer16 => {
                if data.len() >= 2 {
                    if data[1] == 0 {
                        1
                    } else {
                        2
                    }
                } else {
                    data.len()
                }
            }
            DataType::Unsigned32 | DataType::Integer32 => {
                if data.len() >= 4 {
                    let mut size = 4;
                    for i in (1..4).rev() {
                        if data[i] == 0 {
                            size = i;
                        } else {
                            break;
                        }
                    }
                    size.clamp(1, 4)
                } else {
                    data.len()
                }
            }
            // For strings and octet strings, use actual length
            DataType::VisibleString | DataType::OctetString | DataType::UnicodeString => data.len(),
            // For fixed size types, use the full size
            _ => data_type.size_bytes().unwrap_or(data.len()),
        }
    }
}

/// Helper for working with Object Dictionary entries
pub struct ObjectDictionaryEntry<T: CANopenData> {
    pub object_id: ObjectId,
    pub value: T,
}

impl<T: CANopenData> ObjectDictionaryEntry<T> {
    pub fn new(object_id: ObjectId, value: T) -> Self {
        Self { object_id, value }
    }

    pub fn data_type(&self) -> DataType {
        T::canopen_data_type()
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.value.to_canopen_bytes()
    }

    pub fn from_bytes(object_id: ObjectId, data: &[u8]) -> Result<Self> {
        let value = T::from_canopen_bytes(data)?;
        Ok(Self::new(object_id, value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canopen::ObjectId;

    #[test]
    fn test_basic_type_conversions() {
        // Test u32
        let value: u32 = 0x12345678;
        let bytes = value.to_canopen_bytes();
        assert_eq!(bytes, vec![0x78, 0x56, 0x34, 0x12]); // Little endian

        let decoded = u32::from_canopen_bytes(&bytes).unwrap();
        assert_eq!(decoded, value);

        // Test i16
        let value: i16 = -1000;
        let bytes = value.to_canopen_bytes();
        let decoded = i16::from_canopen_bytes(&bytes).unwrap();
        assert_eq!(decoded, value);

        // Test bool
        let value = true;
        let bytes = value.to_canopen_bytes();
        assert_eq!(bytes, vec![1]);

        let decoded = bool::from_canopen_bytes(&bytes).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn test_string_conversion() {
        let value = "Hello".to_string();
        let bytes = value.to_canopen_bytes();
        assert_eq!(bytes, b"Hello");

        let decoded = String::from_canopen_bytes(&bytes).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn test_data_type_validation() {
        // Valid u32 data
        let data = vec![0x12, 0x34, 0x56, 0x78];
        assert!(DataTypeConverter::validate_data_type(&data, DataType::Unsigned32).is_ok());

        // Invalid u32 data (wrong size)
        let data = vec![0x12, 0x34];
        assert!(DataTypeConverter::validate_data_type(&data, DataType::Unsigned32).is_err());

        // Valid visible string
        let data = b"Hello World";
        assert!(DataTypeConverter::validate_data_type(data, DataType::VisibleString).is_ok());

        // Invalid visible string (non-printable character)
        let data = vec![0x01, 0x02, 0x03];
        assert!(DataTypeConverter::validate_data_type(&data, DataType::VisibleString).is_err());
    }

    #[test]
    fn test_minimal_encoding_size() {
        // u32 with minimal representation
        let data = vec![0x12, 0x00, 0x00, 0x00];
        let size = DataTypeConverter::get_minimal_encoding_size(&data, DataType::Unsigned32);
        assert_eq!(size, 1);

        // u32 requiring full representation
        let data = vec![0x12, 0x34, 0x56, 0x78];
        let size = DataTypeConverter::get_minimal_encoding_size(&data, DataType::Unsigned32);
        assert_eq!(size, 4);

        // String (variable length)
        let data = b"Hello";
        let size = DataTypeConverter::get_minimal_encoding_size(data, DataType::VisibleString);
        assert_eq!(size, 5);
    }

    #[test]
    fn test_object_dictionary_entry() {
        let obj_id = ObjectId::new(0x1000, 0x00);
        let entry = ObjectDictionaryEntry::new(obj_id, 0x12345678u32);

        assert_eq!(entry.data_type(), DataType::Unsigned32);
        assert_eq!(entry.to_bytes(), vec![0x78, 0x56, 0x34, 0x12]);

        let reconstructed =
            ObjectDictionaryEntry::<u32>::from_bytes(obj_id, &entry.to_bytes()).unwrap();
        assert_eq!(reconstructed.value, 0x12345678u32);
    }

    #[test]
    fn test_float_conversion() {
        let value: f32 = 1.23;
        let bytes = value.to_canopen_bytes();
        let decoded = f32::from_canopen_bytes(&bytes).unwrap();
        assert!((decoded - value).abs() < f32::EPSILON);

        let value: f64 = 4.56;
        let bytes = value.to_canopen_bytes();
        let decoded = f64::from_canopen_bytes(&bytes).unwrap();
        assert!((decoded - value).abs() < f64::EPSILON);
    }

    #[test]
    fn test_bool_edge_cases() {
        // Test true value
        assert_eq!(true.to_canopen_bytes(), vec![1]);
        assert!(bool::from_canopen_bytes(&[1]).unwrap());

        // Test false value
        assert_eq!(false.to_canopen_bytes(), vec![0]);
        assert!(!bool::from_canopen_bytes(&[0]).unwrap());

        // Test non-zero as true
        assert!(bool::from_canopen_bytes(&[255]).unwrap());
        assert!(bool::from_canopen_bytes(&[42]).unwrap());

        // Test empty data error
        assert!(bool::from_canopen_bytes(&[]).is_err());
    }

    #[test]
    fn test_integer_roundtrip() {
        // u8
        let val: u8 = 255;
        assert_eq!(
            u8::from_canopen_bytes(&val.to_canopen_bytes()).unwrap(),
            val
        );

        // i8
        let val: i8 = -128;
        assert_eq!(
            i8::from_canopen_bytes(&val.to_canopen_bytes()).unwrap(),
            val
        );

        // u16
        let val: u16 = 0xABCD;
        assert_eq!(
            u16::from_canopen_bytes(&val.to_canopen_bytes()).unwrap(),
            val
        );

        // i16
        let val: i16 = -32768;
        assert_eq!(
            i16::from_canopen_bytes(&val.to_canopen_bytes()).unwrap(),
            val
        );

        // u32
        let val: u32 = 0xDEADBEEF;
        assert_eq!(
            u32::from_canopen_bytes(&val.to_canopen_bytes()).unwrap(),
            val
        );

        // i32
        let val: i32 = -2147483648;
        assert_eq!(
            i32::from_canopen_bytes(&val.to_canopen_bytes()).unwrap(),
            val
        );

        // u64
        let val: u64 = 0x0123456789ABCDEF;
        assert_eq!(
            u64::from_canopen_bytes(&val.to_canopen_bytes()).unwrap(),
            val
        );

        // i64
        let val: i64 = -9223372036854775808;
        assert_eq!(
            i64::from_canopen_bytes(&val.to_canopen_bytes()).unwrap(),
            val
        );
    }

    #[test]
    fn test_little_endian_byte_order() {
        // Verify little-endian byte order (CANopen standard)
        let val: u16 = 0x1234;
        assert_eq!(val.to_canopen_bytes(), vec![0x34, 0x12]);

        let val: u32 = 0x12345678;
        assert_eq!(val.to_canopen_bytes(), vec![0x78, 0x56, 0x34, 0x12]);

        let val: u64 = 0x0123456789ABCDEF;
        assert_eq!(
            val.to_canopen_bytes(),
            vec![0xEF, 0xCD, 0xAB, 0x89, 0x67, 0x45, 0x23, 0x01]
        );
    }

    #[test]
    fn test_insufficient_data_errors() {
        // u16 with 1 byte
        assert!(u16::from_canopen_bytes(&[0x12]).is_err());

        // u32 with 3 bytes
        assert!(u32::from_canopen_bytes(&[0x12, 0x34, 0x56]).is_err());

        // u64 with 7 bytes
        assert!(u64::from_canopen_bytes(&[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07]).is_err());

        // i16 with empty data
        assert!(i16::from_canopen_bytes(&[]).is_err());

        // f32 with 2 bytes
        assert!(f32::from_canopen_bytes(&[0x00, 0x00]).is_err());

        // f64 with 4 bytes
        assert!(f64::from_canopen_bytes(&[0x00, 0x00, 0x00, 0x00]).is_err());
    }

    #[test]
    fn test_string_roundtrip() {
        let text = "Hello CANopen!".to_string();
        let bytes = text.to_canopen_bytes();
        let decoded = String::from_canopen_bytes(&bytes).unwrap();
        assert_eq!(decoded, text);

        // Empty string
        let empty = String::new();
        let bytes = empty.to_canopen_bytes();
        let decoded = String::from_canopen_bytes(&bytes).unwrap();
        assert_eq!(decoded, empty);

        // Test with special characters
        let special = "Test\r\n\t!@#$%".to_string();
        let bytes = special.to_canopen_bytes();
        let decoded = String::from_canopen_bytes(&bytes).unwrap();
        assert_eq!(decoded, special);
    }

    #[test]
    fn test_vec_u8_conversion() {
        let data: Vec<u8> = vec![0x01, 0x02, 0x03, 0xFF, 0xAA, 0x55];
        let bytes = data.to_canopen_bytes();
        let decoded = Vec::<u8>::from_canopen_bytes(&bytes).unwrap();
        assert_eq!(decoded, data);

        // Empty vector
        let empty: Vec<u8> = vec![];
        let bytes = empty.to_canopen_bytes();
        let decoded = Vec::<u8>::from_canopen_bytes(&bytes).unwrap();
        assert_eq!(decoded, empty);
    }

    #[test]
    fn test_data_type_sizes() {
        assert_eq!(DataType::Boolean.size_bytes(), Some(1));
        assert_eq!(DataType::Integer8.size_bytes(), Some(1));
        assert_eq!(DataType::Unsigned8.size_bytes(), Some(1));
        assert_eq!(DataType::Integer16.size_bytes(), Some(2));
        assert_eq!(DataType::Unsigned16.size_bytes(), Some(2));
        assert_eq!(DataType::Integer32.size_bytes(), Some(4));
        assert_eq!(DataType::Unsigned32.size_bytes(), Some(4));
        assert_eq!(DataType::Real32.size_bytes(), Some(4));
        assert_eq!(DataType::Integer64.size_bytes(), Some(8));
        assert_eq!(DataType::Unsigned64.size_bytes(), Some(8));

        // Variable-length types
        assert_eq!(DataType::VisibleString.size_bytes(), None);
        assert_eq!(DataType::OctetString.size_bytes(), None);
    }

    #[test]
    fn test_negative_integers() {
        // i8 boundary values
        let min_i8: i8 = -128;
        let max_i8: i8 = 127;
        assert_eq!(
            i8::from_canopen_bytes(&min_i8.to_canopen_bytes()).unwrap(),
            min_i8
        );
        assert_eq!(
            i8::from_canopen_bytes(&max_i8.to_canopen_bytes()).unwrap(),
            max_i8
        );

        // i16 boundary values
        let min_i16: i16 = -32768;
        let max_i16: i16 = 32767;
        assert_eq!(
            i16::from_canopen_bytes(&min_i16.to_canopen_bytes()).unwrap(),
            min_i16
        );
        assert_eq!(
            i16::from_canopen_bytes(&max_i16.to_canopen_bytes()).unwrap(),
            max_i16
        );

        // i32 boundary values
        let min_i32: i32 = -2147483648;
        let max_i32: i32 = 2147483647;
        assert_eq!(
            i32::from_canopen_bytes(&min_i32.to_canopen_bytes()).unwrap(),
            min_i32
        );
        assert_eq!(
            i32::from_canopen_bytes(&max_i32.to_canopen_bytes()).unwrap(),
            max_i32
        );
    }

    #[test]
    fn test_float_special_values() {
        // Test NaN
        let nan = f32::NAN;
        let bytes = nan.to_canopen_bytes();
        let decoded = f32::from_canopen_bytes(&bytes).unwrap();
        assert!(decoded.is_nan());

        // Test infinity
        let inf = f32::INFINITY;
        let bytes = inf.to_canopen_bytes();
        let decoded = f32::from_canopen_bytes(&bytes).unwrap();
        assert_eq!(decoded, f32::INFINITY);

        // Test negative infinity
        let neg_inf = f32::NEG_INFINITY;
        let bytes = neg_inf.to_canopen_bytes();
        let decoded = f32::from_canopen_bytes(&bytes).unwrap();
        assert_eq!(decoded, f32::NEG_INFINITY);

        // Test zero
        let zero: f32 = 0.0;
        let bytes = zero.to_canopen_bytes();
        let decoded = f32::from_canopen_bytes(&bytes).unwrap();
        assert_eq!(decoded, 0.0);
    }

    #[test]
    fn test_canopen_data_type_identification() {
        assert_eq!(bool::canopen_data_type(), DataType::Boolean);
        assert_eq!(u8::canopen_data_type(), DataType::Unsigned8);
        assert_eq!(i8::canopen_data_type(), DataType::Integer8);
        assert_eq!(u16::canopen_data_type(), DataType::Unsigned16);
        assert_eq!(i16::canopen_data_type(), DataType::Integer16);
        assert_eq!(u32::canopen_data_type(), DataType::Unsigned32);
        assert_eq!(i32::canopen_data_type(), DataType::Integer32);
        assert_eq!(u64::canopen_data_type(), DataType::Unsigned64);
        assert_eq!(i64::canopen_data_type(), DataType::Integer64);
        assert_eq!(f32::canopen_data_type(), DataType::Real32);
        assert_eq!(f64::canopen_data_type(), DataType::Real64);
        assert_eq!(String::canopen_data_type(), DataType::VisibleString);
        assert_eq!(Vec::<u8>::canopen_data_type(), DataType::OctetString);
    }
}
