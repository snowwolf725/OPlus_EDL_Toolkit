use crate::file_util;
use quick_xml::de::from_str;
use quick_xml::se::to_string;
use serde::{Deserialize, Serialize, Serializer};
use std::fmt::Write;

// Define struct for the root <data> node in XML
#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename = "data")]  // Match the root node name in XML
pub struct DataRoot {
    // Match multiple <program> child nodes under <data>
    #[serde(rename = "program", default)]
    pub programs: Vec<Program>,
    
    // Match multiple <read> child nodes under <data>
    #[serde(rename = "read", default)]
    pub read_tags: Vec<ReadTag>,

    // Match multiple <erase> child nodes under <data>
    #[serde(rename = "erase", default)]
    pub erase_tags: Vec<EraseTag>,
}

// Define struct for the <program> node (matches all attributes)
#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename = "program")]
pub struct Program {
    #[serde(rename = "@start_sector")]
    pub start_sector: String,
    #[serde(rename = "@size_in_KB", serialize_with = "serialize_size_in_kb")]
    pub size_in_kb: f64,
    #[serde(rename = "@physical_partition_number")]
    pub physical_partition_number: u8,
    #[serde(rename = "@partofsingleimage")]
    pub part_of_single_image: bool,
    #[serde(rename = "@file_sector_offset")]
    pub file_sector_offset: u64,
    #[serde(rename = "@num_partition_sectors")]
    pub num_partition_sectors: u64,
    #[serde(rename = "@readbackverify")]
    pub readback_verify: bool,
    #[serde(rename = "@filename")]
    pub filename: String,
    #[serde(rename = "@sparse")]
    pub sparse: bool,
    #[serde(rename = "@start_byte_hex")]
    pub start_byte_hex: String,
    #[serde(rename = "@SECTOR_SIZE_IN_BYTES")]
    pub sector_size_in_bytes: u64,
    #[serde(rename = "@label")]
    pub label: String,
}

// Define struct for the <read> node (matches all attributes)
#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename = "erase")]
pub struct EraseTag {
    #[serde(rename = "@SECTOR_SIZE_IN_BYTES")]
    pub sector_size_in_bytes: u64,
    #[serde(rename = "@label")]
    pub label: String,
    #[serde(rename = "@physical_partition_number")]
    pub physical_partition_number: u8,
    #[serde(rename = "@start_sector")]
    pub start_sector: u64,
    #[serde(rename = "@num_partition_sectors")]
    pub num_partition_sectors: u64,
    
}

// Define struct for the <read> node (matches all attributes)
#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename = "read")]
pub struct ReadTag {
    #[serde(rename = "@filename")]
    pub filename: String,
    #[serde(rename = "@physical_partition_number")]
    pub physical_partition_number: u8,
    #[serde(rename = "@label")]
    pub label: String,
    #[serde(rename = "@start_sector")]
    pub start_sector: u64,
    #[serde(rename = "@num_partition_sectors")]
    pub num_partition_sectors: u64,
    #[serde(rename = "@SECTOR_SIZE_IN_BYTES")]
    pub sector_size_in_bytes: u64,
    #[serde(rename = "@sparse")]
    pub sparse: bool,
}

fn serialize_size_in_kb<S>(value: &f64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut size_str = String::new();
    write!(&mut size_str, "{0:.1}", value).unwrap();
    serializer.serialize_str(&size_str)
}


pub fn create_program_dynamic(
    lun: u8,
    start_sector: u64,
    num_partition_sectors: u64,
    label: &str
) -> Program {
    let size_in_kb = num_partition_sectors as f64 * 4.0;
    let sector_size = 4096;
    let start_byte = start_sector as u64 * sector_size as u64;
    let start_byte_hex = format!("{:X}", start_byte);

    Program {
        start_sector: format!("{}", start_sector),
        size_in_kb,
        physical_partition_number: lun,
        part_of_single_image: false,
        file_sector_offset: 0,
        num_partition_sectors,
        readback_verify: false,
        filename: format!("{}.img", label).to_string(),
        sparse: false,
        start_byte_hex,
        sector_size_in_bytes: 4096,
        label: label.to_string(),
    }
}

pub fn create_read_tag_dynamic(
    filename: &str,
    lun: u8,
    start_sector: u64,
    num_partition_sectors: u64,
    label: &str
) -> ReadTag {
    ReadTag {
        filename: filename.to_string(),
        physical_partition_number: lun,
        label: label.to_string(),
        start_sector,
        num_partition_sectors,
        sector_size_in_bytes: 4096,
        sparse: false,
    }
}

pub fn parser_program_xml(parent_dir: &str, content: &str) -> Vec<(String, String, String)> {
    let mut result = Vec::<(String, String, String)>::new();
    // Call the parsing function
    match from_str::<DataRoot>(&content) {
        Ok(data_root) => {
            let output_dir = "res";
            if let Err(e) = file_util::create_dir_if_not_exists(output_dir) {
                eprint!("create res dir failed:{}", e);
                return result;
            }
            // Iterate and print each program
            for mut program in data_root.programs {
               let (file_name, dir_path) = file_util::parse_file_path(&parent_dir, &program.filename);
               println!("Test {}, {}, {}", &program.filename, &file_name, &dir_path);
               program.filename = file_name;
               let program_xml = match to_string(&program) {
                    Ok(xml) => xml,
                    Err(_e) => {
                        continue;
                    }
               };
               let xml_content = format!("<?xml version=\"1.0\" ?>\n<data>\n{}\n</data>\n", program_xml);
               result.push((program.label, xml_content, dir_path));
            }
        }
        Err(e) => {
            eprint!("XML parsing failed: {}", e);
            return result;
        }
    }
    return result;
}

pub fn parser_program_xml_skip_empty(parent_dir: &str, content: &str) -> Vec<(String, String, String)> {
    let mut result = Vec::<(String, String, String)>::new();
    // Call the parsing function
    match from_str::<DataRoot>(&content) {
        Ok(data_root) => {
            // Iterate and print each program
            for program in data_root.programs {
                if program.filename.trim().is_empty() {
                    continue;
                }
                let (file_name, _dir_path) = file_util::parse_file_path(&parent_dir, &program.filename);

                let program_xml = match to_string(&program) {
                    Ok(xml) => xml,
                    Err(_e) => {
                        continue;
                    }
                };
                let xml_content = format!("<?xml version=\"1.0\" ?>\n<data>\n{}\n</data>\n", program_xml);
                result.push((program.label, file_name, xml_content));
            }
        }
        Err(e) => {
            eprint!("XML parsing failed: {}", e);
            return result;
        }
    }
    return result;
}

pub fn parser_erase_xml(content: &str) -> Vec<(String, String)> {
    let mut result = Vec::<(String, String)>::new();
    // Call the parsing function
    match from_str::<DataRoot>(&content) {
        Ok(data_root) => {
            let output_dir = "res";
            if let Err(e) = file_util::create_dir_if_not_exists(output_dir) {
                eprint!("create res dir failed:{}", e);
                return result;
            }
            // Iterate and print each tag
            for tag in data_root.erase_tags {
               let erase_xml = match to_string(&tag) {
                    Ok(xml) => xml,
                    Err(_e) => {
                        continue;
                    }
                };
               let xml_content = format!("<?xml version=\"1.0\" ?>\n<data>\n{}\n</data>\n", erase_xml);
               result.push((tag.label, xml_content));
            }
        }
        Err(e) => {
            eprint!("XML parsing failed: {}", e);
            return result;
        }
    }
    return result;
}


pub fn parser_read_xml(content: &str) -> Vec<(String, String)> {
    let mut result = Vec::<(String, String)>::new();
    // Call the parsing function
    match from_str::<DataRoot>(&content) {
        Ok(data_root) => {
            let output_dir = "res";
            if let Err(e) = file_util::create_dir_if_not_exists(output_dir) {
                eprint!("create res dir failed:{}", e);
                return result;
            }
            // Iterate and print each read
            for read in data_root.read_tags {
               let read_xml = match to_string(&read) {
                    Ok(xml) => xml,
                    Err(_e) => {
                        continue;
                    }
                };
               let xml_content = format!("<?xml version=\"1.0\" ?>\n<data>\n{}\n</data>\n", read_xml);
               result.push((read.label, xml_content));
            }
        }
        Err(e) => {
            eprint!("XML parsing failed: {}", e);
            return result;
        }
    }
    return result;
}

pub fn to_xml<T: serde::Serialize>(tag: &T) -> String {
    let read_xml = match to_string(&tag) {
        Ok(xml) => xml,
        Err(_e) => {
            "".to_string()
        }
    };
    return read_xml;
}
