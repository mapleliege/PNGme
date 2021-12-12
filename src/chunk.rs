
use crate::{Error, Result};
use std::{
    convert::{TryFrom, TryInto},
    fmt::Display
};
use crate::chunk_type::ChunkType;

pub struct Chunk {
    chunk_type: ChunkType,
    message_bytes: Vec<u8>,
}

impl Chunk {
    pub const DATA_LENGTH_BYTES: usize = 4;
    pub const CHUNK_TYPE_BYTES: usize = 4;
    pub const CRC_BYTES: usize = 4;

    pub const METADATA_BYTES: usize =
        Chunk::DATA_LENGTH_BYTES + Chunk::CHUNK_TYPE_BYTES + Chunk::CRC_BYTES;

    pub fn length(&self) -> usize {
        self.message_bytes.len()
    }

    pub fn chunk_type(&self) -> &ChunkType {
        &self.chunk_type
    }

    pub fn crc(&self) -> u32 {
        let bytes: Vec<u8> = self.chunk_type
            .bytes()
            .iter()
            .chain(self.message_bytes.iter())
            .copied()
            .collect();
        crc::crc32::checksum_ieee(&bytes)
    }

    pub fn data(&self) -> &[u8] {
        &self.message_bytes.as_slice()
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        let data_length = self.message_bytes.len() as u32;
        data_length
            .to_be_bytes()
            .iter()
            .chain(self.chunk_type().bytes().iter())
            .chain(self.data().iter())
            .chain(self.crc().to_be_bytes().iter())
            .copied()
            .collect()
    }

    pub fn data_as_string(&self) -> Result<String> {
        let data_string = std::str::from_utf8(&self.message_bytes)?;
        Ok(data_string.to_string())
    }

}

impl TryFrom<&[u8]> for Chunk {
    type Error = Error;

    fn try_from(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < Chunk::METADATA_BYTES {
            return Err(Box::from(ChunkError::InputTooSmall))
        }
        // first 4 bytes is the length of the chunk
        let (data_length, bytes) = bytes.split_at(Chunk::DATA_LENGTH_BYTES);
        let data_length = u32::from_be_bytes(data_length.try_into()?) as usize;
        // next 4 bytes is the chunk type
        let (chunk_type_bytes, bytes) = bytes.split_at(Chunk::CHUNK_TYPE_BYTES);
        let chunk_type_bytes: [u8; 4] = chunk_type_bytes.try_into()?;
        let chunk_type: ChunkType = ChunkType::try_from(chunk_type_bytes)?;
        // validate chunk type
        if !chunk_type.is_valid() {
            return Err(Box::from(ChunkError::InvalidChunkType))
        }
        // next 4 bytes is the message
        let (message_bytes, bytes) = bytes.split_at(data_length);
        // last 4 bytes are the CRC, disregard last splitting of bytes
        let (crc_bytes, _) = bytes.split_at(Chunk::CRC_BYTES);

        let new = Self {
            chunk_type: chunk_type,
            message_bytes: message_bytes.into()
        };

        // validated crc
        let actual_crc = new.crc();
        let expected_crc = u32::from_be_bytes(crc_bytes.try_into()?);

        if expected_crc != actual_crc {
            return Err(Box::from(ChunkError::InvalidCrc(expected_crc, actual_crc)));
        }
        Ok(new)
    }
}

impl Display for Chunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Chunk {{",)?;
        writeln!(f, "  Length: {}", self.length())?;
        writeln!(f, "  Type: {}", self.chunk_type())?;
        writeln!(f, "  Data: {} bytes", self.data().len())?;
        writeln!(f, "  Crc: {}", self.crc())?;
        writeln!(f, "}}",)?;
        Ok(())
    }
}

// Chunk Errors
#[derive(Debug)]
pub enum ChunkError {
    // Chunk has Invalid CRC
    InvalidCrc(u32, u32),

    // Input String is too small to be a valid Chunk, 12 bytes minimum
    InputTooSmall,

    // Chunk Type is invalid
    InvalidChunkType
}

impl std::error::Error for ChunkError {}

impl Display for ChunkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChunkError::InvalidCrc(expected,actual) => write!(
                f,
                "Expected CRC: {} does not match actual CRC: {}",
                expected,
                actual
            ),
            ChunkError::InputTooSmall => {
                write!(f, "Input String is too small to be a valid Chunk")
            },
            ChunkError::InvalidChunkType => {
                write!(f, "Invalid ChunkType")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk_type::ChunkType;
    use std::str::FromStr;

    fn testing_chunk() -> Chunk {
        let data_length: u32 = 42;
        let chunk_type = "RuSt".as_bytes();
        let message_bytes = "This is where your secret message will be!".as_bytes();
        let crc: u32 = 2882656334;

        let chunk_data: Vec<u8> = data_length
            .to_be_bytes()
            .iter()
            .chain(chunk_type.iter())
            .chain(message_bytes.iter())
            .chain(crc.to_be_bytes().iter())
            .copied()
            .collect();
        
        Chunk::try_from(chunk_data.as_ref()).unwrap()
    }

    #[test]
    fn test_chunk_length() {
        let chunk = testing_chunk();
        assert_eq!(chunk.length(), 42);
    }

    #[test]
    fn test_chunk_type() {
        let chunk = testing_chunk();
        assert_eq!(chunk.chunk_type().to_string(), String::from("RuSt"));
    }

    #[test]
    fn test_chunk_string() {
        let chunk = testing_chunk();
        let chunk_string = chunk.data_as_string().unwrap();
        let expected_chunk_string = String::from("This is where your secret message will be!");
        assert_eq!(chunk_string, expected_chunk_string);
    }

    #[test]
    fn test_chunk_crc() {
        let chunk = testing_chunk();
        assert_eq!(chunk.crc(), 2882656334);
    }

    #[test]
    fn test_valid_chunk_from_bytes() {
        let data_length: u32 = 42;
        let chunk_type = "RuSt".as_bytes();
        let message_bytes = "This is where your secret message will be!".as_bytes();
        let crc: u32 = 2882656334;

        let chunk_data: Vec<u8> = data_length
            .to_be_bytes()
            .iter()
            .chain(chunk_type.iter())
            .chain(message_bytes.iter())
            .chain(crc.to_be_bytes().iter())
            .copied()
            .collect();

        let chunk = Chunk::try_from(chunk_data.as_ref()).unwrap();

        let chunk_string = chunk.data_as_string().unwrap();
        let expected_chunk_string = String::from("This is where your secret message will be!");

        assert_eq!(chunk.length(), 42);
        assert_eq!(chunk.chunk_type().to_string(), String::from("RuSt"));
        assert_eq!(chunk_string, expected_chunk_string);
        assert_eq!(chunk.crc(), 2882656334);
    }

    #[test]
    fn test_invalid_chunk_from_bytes() {
        let data_length: u32 = 42;
        let chunk_type = "RuSt".as_bytes();
        let message_bytes = "This is where your secret message will be!".as_bytes();
        let crc: u32 = 2882656333;

        let chunk_data: Vec<u8> = data_length
            .to_be_bytes()
            .iter()
            .chain(chunk_type.iter())
            .chain(message_bytes.iter())
            .chain(crc.to_be_bytes().iter())
            .copied()
            .collect();

        let chunk = Chunk::try_from(chunk_data.as_ref());

        assert!(chunk.is_err());
    }

    #[test]
    pub fn test_chunk_trait_impls() {
        let data_length: u32 = 42;
        let chunk_type = "RuSt".as_bytes();
        let message_bytes = "This is where your secret message will be!".as_bytes();
        let crc: u32 = 2882656334;

        let chunk_data: Vec<u8> = data_length
            .to_be_bytes()
            .iter()
            .chain(chunk_type.iter())
            .chain(message_bytes.iter())
            .chain(crc.to_be_bytes().iter())
            .copied()
            .collect();
        
        let chunk: Chunk = TryFrom::try_from(chunk_data.as_ref()).unwrap();
        
        let _chunk_string = format!("{}", chunk);
    }
}
