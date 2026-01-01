use byteorder::{LittleEndian, ReadBytesExt};
use encoding::all::UTF_16LE;
use encoding::{DecoderTrap, Encoding};
use std::io::{Cursor, Read, Seek, SeekFrom};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GptError {
    #[error("Insufficient data length")]
    InsufficientData,
    #[error("Invalid GPT signature")]
    InvalidSignature,
    #[error("Partition table entry out of data range")]
    EntryOutOfBounds,
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("UTF-16 decoding error")]
    Utf16DecodeError,
}

pub type Result<T> = std::result::Result<T, GptError>;

#[derive(Debug, Clone, PartialEq)]
pub struct PartitionEntry {
    pub name: String,
    pub first_lba: u64,
    pub last_lba: u64,
}

impl PartitionEntry {
    pub fn size_in_bytes(&self, sector_size: u64) -> u64 {
        (self.last_lba - self.first_lba + 1) * sector_size
    }

    pub fn size_in_sectors(&self) -> u64 {
        self.last_lba - self.first_lba + 1
    }
}

#[derive(Debug, Clone)]
pub struct GptParser {
    partitions: Vec<PartitionEntry>,
    sector_size: u32,
}

impl GptParser {
    pub fn new() -> Self {
        Self {
            partitions: Vec::new(),
            sector_size: 512,
        }
    }

    pub fn partitions(&self) -> &[PartitionEntry] {
        &self.partitions
    }

    pub fn parse(&mut self, data: &[u8], sector_size: u32) -> Result<()> {
        self.partitions.clear();
        self.sector_size = sector_size;

        // Check data length
        if data.len() < (sector_size * 2) as usize {
            return Err(GptError::InsufficientData);
        }

        let mut cursor = Cursor::new(data);

        // 1. Skip MBR (LBA 0)
        cursor.seek(SeekFrom::Start(sector_size as u64))?;

        // 2. Read and verify GPT signature
        let mut signature = [0u8; 8];
        cursor.read_exact(&mut signature)?;

        if &signature != b"EFI PART" {
            return Err(GptError::InvalidSignature);
        }

        // 3. Read key information from GPT Header
        // Seek to offset 72 (0x48): Partition Entry Start LBA
        cursor.seek(SeekFrom::Start(sector_size as u64 + 72))?;

        let part_entry_start_lba = cursor.read_u64::<LittleEndian>()?;
        let num_part_entries = cursor.read_u32::<LittleEndian>()?;
        let part_entry_size = cursor.read_u32::<LittleEndian>()?;

        println!("{},{}", part_entry_start_lba, part_entry_size);

        // 4. Calculate actual offset of partition table entries
        let entry_offset = part_entry_start_lba * sector_size as u64;
        if entry_offset as usize >= data.len() {
            return Err(GptError::EntryOutOfBounds);
        }

        cursor.seek(SeekFrom::Start(entry_offset))?;

        // 5. Iterate through all partition table entries
        for _i in 0..num_part_entries {
            // Check if there is enough data
            let entry_start_pos = cursor.position();
            if entry_start_pos + part_entry_size as u64 > data.len() as u64 {
                break;
            }

            // Read Type GUID (16 bytes)
            let mut type_guid = [0u8; 16];
            cursor.read_exact(&mut type_guid)?;

            // Check if it's an empty partition (all bytes are 0)
            let is_empty = type_guid.iter().all(|&b| b == 0);

            if !is_empty {
                // Seek to offset 32 (0x20): First LBA
                cursor.seek(SeekFrom::Start(entry_start_pos + 32))?;

                let first_lba = cursor.read_u64::<LittleEndian>()?;
                let last_lba = cursor.read_u64::<LittleEndian>()?;

                // Seek to offset 56 (0x38): Name (72 bytes UTF-16LE)
                cursor.seek(SeekFrom::Start(entry_start_pos + 56))?;

                let mut name_bytes = vec![0u8; 72];
                cursor.read_exact(&mut name_bytes)?;

                // Decode UTF-16LE string
                let name = Self::decode_utf16le(&name_bytes)?;

                self.partitions.push(PartitionEntry {
                    name,
                    first_lba,
                    last_lba,
                });
            }

            // Seek to next entry
            cursor.seek(SeekFrom::Start(entry_start_pos + part_entry_size as u64))?;
        }

        Ok(())
    }

    pub fn parse_file<P: AsRef<std::path::Path>>(
        &mut self,
        path: P,
        sector_size: u32,
    ) -> Result<()> {
        let data = std::fs::read(path)?;
        self.parse(&data, sector_size)
    }

    fn decode_utf16le(bytes: &[u8]) -> Result<String> {
        // Remove trailing null characters
        let mut trimmed_bytes = bytes;
        while trimmed_bytes.len() >= 2 && trimmed_bytes[trimmed_bytes.len() - 2..] == [0, 0] {
            trimmed_bytes = &trimmed_bytes[..trimmed_bytes.len() - 2];
        }

        UTF_16LE
            .decode(trimmed_bytes, DecoderTrap::Strict)
            .map_err(|_| GptError::Utf16DecodeError)
    }

    // Utility methods
    #[allow(dead_code)]
    pub fn find_partition_by_name(&self, name: &str) -> Option<&PartitionEntry> {
        self.partitions
            .iter()
            .find(|p| p.name.to_lowercase() == name.to_lowercase())
    }

    #[allow(dead_code)]
    pub fn get_partition(&self, index: usize) -> Option<&PartitionEntry> {
        self.partitions.get(index)
    }

    #[allow(dead_code)]
    pub fn partition_count(&self) -> usize {
        self.partitions.len()
    }

    #[allow(dead_code)]
    pub fn total_disk_size(&self, total_sectors: u64) -> u64 {
        total_sectors * self.sector_size as u64
    }

    #[allow(dead_code)]
    pub fn print_summary(&self) {
        println!("GPT Partition Table Parsing Result:");
        println!("Sector size: {} bytes", self.sector_size);
        println!("Number of partitions: {}", self.partitions.len());
        println!();

        for (i, partition) in self.partitions.iter().enumerate() {
            println!("Partition {}:", i);
            println!("  Name: {}", partition.name);
            println!("  Start LBA: {}", partition.first_lba);
            println!("  End LBA: {}", partition.last_lba);
            println!("  Number of sectors: {}", partition.size_in_sectors());
            println!(
                "  Size: {} bytes",
                partition.size_in_bytes(self.sector_size as u64)
            );
            println!(
                "  Size: {:.2} MB",
                partition.size_in_bytes(self.sector_size as u64) as f64 / (1024.0 * 1024.0)
            );
            println!();
        }
    }
}

// Implement Display trait for PartitionEntry
impl std::fmt::Display for PartitionEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} (LBA {} - {}, {} sectors)",
            self.name,
            self.first_lba,
            self.last_lba,
            self.last_lba - self.first_lba + 1
        )
    }
}
