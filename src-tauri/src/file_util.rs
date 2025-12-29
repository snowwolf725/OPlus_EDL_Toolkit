use crate::xml_file_util;
use glob::glob;
use regex::Regex;
use std::fmt;
use std::io;
use std::fs;
use std::fs::metadata;
use std::path::Path;

/// Custom error type (simplified error handling)
#[derive(Debug)]
pub enum CheckFileError {
    InvalidPath,          // Invalid folder path
    DirectoryNotFound,    // Folder does not exist
    GlobError(glob::PatternError), // Wildcard pattern parsing error
    GlobIterError(glob::GlobError),       // Glob iteration error
}

pub struct EdlPackage {
    pub is_miss_file: bool,
    pub is_miss_super_image: bool,
    pub super_define: String,
    pub raw_program_files: Vec<String>,
    pub raw_programs: Vec<(String, String)>,
    pub patch_files: Vec<String>,
}

// Implement error formatting (for easy printing)
impl fmt::Display for CheckFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CheckFileError::InvalidPath => write!(f, "Invalid folder path"),
            CheckFileError::DirectoryNotFound => write!(f, "Directory not found"),
            CheckFileError::GlobError(e) => write!(f, "Wildcard pattern error: {}", e),
            CheckFileError::GlobIterError(e) => write!(f, "Glob iteration error: {}", e),
        }
    }
}

pub fn check_file_exist(file_path: &str) -> bool {
    let path = Path::new(file_path);

    let file_metadata = match metadata(path) {
        Ok(meta) => meta,
        Err(_) => return false,
    };

    file_metadata.is_file() && file_metadata.len() > 0
}

pub fn check_folder_exist(path_str: &str) -> bool {
    let path = Path::new(path_str);
    
    match path.try_exists() {
        Ok(exists) => exists,
        Err(_) => false,
    }
}

pub fn create_dir_if_not_exists(path: &str) -> io::Result<()> {
    let dir_path = Path::new(path);
    if !dir_path.exists() {
        fs::create_dir_all(dir_path)?;
    }
    Ok(())
}

pub fn write_to_file(path: &str, output_dir: &str, content: &str) {
    if let Err(e) = create_dir_if_not_exists(output_dir) {
        println!("create {} dir failed:{}", output_dir, e);
    }
    let file_name = format!("{}/{}", &output_dir, &path);
    println!("file:{}", &file_name);
    if let Err(e) = fs::write(&file_name, content) {
        eprintln!("write file{}failed:{}", file_name, e);
    } else {
        println!("write file success:{}", file_name);
    }
}


pub fn read_text_file(file_path: &str) -> Result<String, String> {
    // Convert string path to Path object for validation
    let path = Path::new(file_path);

    // Validate path exists before reading
    if !path.exists() {
        return Err(format!("File does not exist: '{}'", file_path));
    }

    // Validate path is a file (not a directory)
    if !path.is_file() {
        return Err(format!("Path is a directory, not a file: '{}'", file_path));
    }

    // Read file content to string with error handling
    match fs::read_to_string(path) {
        Ok(content) => Ok(content),
        Err(e) => {
            // Map system errors to human-readable messages
            let error_msg = match e.kind() {
                std::io::ErrorKind::PermissionDenied => {
                    format!("Permission denied: Cannot read file '{}'", file_path)
                }
                std::io::ErrorKind::NotFound => {
                    format!("File not found: '{}'", file_path)
                }
                std::io::ErrorKind::InvalidData => {
                    format!("Invalid text encoding: File '{}' is not a valid text file", file_path)
                }
                _ => format!(
                    "Failed to read file '{}': {}",
                    file_path, e
                ),
            };
            Err(error_msg)
        }
    }
}

pub fn parse_file_path(parent_dir:&str, path: &str) -> (String, String) {
    // Convert the input string to a Path object for path manipulation
    let mut full_path = Path::new(path);
    let mut file_path = path.to_string();
    if parent_dir.is_empty() {
        if check_file_exist(&file_path) == false {
            return ("".to_string(), "".to_string());
        }
    } else {
        file_path = format!("{}/{}", parent_dir, path);
        if check_file_exist(&file_path) == false {
            return ("".to_string(), "".to_string());
        } else {
            full_path = Path::new(&file_path);
        }
    }

    // 1. Extract the file name from the path with error handling
    let file_name = full_path
        .file_name()
        .ok_or_else(|| format!("Failed to extract file name from path '{}'", file_path)) // Return error string
        .and_then(|os_str| {
            // Convert OsStr to &str, return error if invalid Unicode
            os_str.to_str()
                .ok_or_else(|| format!("Path '{}' contains invalid Unicode characters", file_path))
        })
        .unwrap() // Panic on error (simple error handling for this example)
        .to_string();

    // 2. Extract the parent directory of the file with error handling
    let directory = full_path
        .parent()
        .ok_or_else(|| format!("Failed to extract directory from path '{}' (may be root directory)", file_path))
        .and_then(|path| {
            // Convert Path to &str, return error if invalid Unicode
            path.to_str()
                .ok_or_else(|| format!("Directory path '{}' contains invalid Unicode characters", file_path))
        })
        .unwrap() // Panic on error (simple error handling for this example)
        .to_string();

    (file_name, directory)
}

pub fn analysis_info(input: &str) -> String {
    let mut output = String::new();
    let re = Regex::new(r"0x[0-9a-fA-F]+").expect("Reg compile failed");
    input.lines().for_each(|line| {
        if line.contains("Device Total Physical Partitions") {
            if let Some(hex_match) = re.find(line) {
                let hex_str = hex_match.as_str();
                match u64::from_str_radix(&hex_str[2..], 16) {
                    Ok(decimal) => { output = format!("{}\n Device Total Physical Partitions:{}", output, decimal);},
                    Err(e) => eprintln!("Convert Failed: {}", e),
                }
            }
            println!("{}", line);
        } else if line.contains("Device Serial Number") {
            if let Some(hex_match) = re.find(line) {
                let hex_str = hex_match.as_str();
                output = format!("{}\n Device Serial Number:{}", output, hex_str);
            }
        } else if line.contains("UFS Inquiry Command Output") {
            if let Some((_, content)) = line.split_once("Output:") {
                let model = content.replace("'", " ");
                output = format!("{}\n Storage:{}", output, model.trim());
            }
        } else if line.contains("Boot Partition Enabled") {
            if let Some(hex_match) = re.find(line) {
                let hex_str = hex_match.as_str();
                match u64::from_str_radix(&hex_str[2..], 16) {
                    Ok(decimal) => {
                        let slot = if decimal == 1 {
                            "A"
                        } else {
                            "B"
                        };
                        output = format!("{}\n Active slot:{}", output, slot);
                    },
                    Err(e) => eprintln!("Convert Failed: {}", e),
                }
            }
        } 
    });
    return output;
}

/// Check all files matching pattern in the specified folder and return their paths
/// 
/// # Parameters
/// - `dir_path`: Folder path (absolute/relative path)
/// - `pattern`: file glob pattern
/// 
/// # Returns
/// - `Result<Vec<String>, CheckFileError>`: Ok(Vec<String>) contains all matched file paths, Error if exception occurs
fn get_matched_files_in_folder(dir_path: &str, glob_pattern: &str) -> Result<Vec<String>, CheckFileError> {
    // 1. Validate the target directory's existence and type
    let dir = Path::new(dir_path);
    if !dir.exists() {
        return Err(CheckFileError::DirectoryNotFound);
    }
    if !dir.is_dir() {
        return Err(CheckFileError::InvalidPath);
    }

    // 2. Initialize a vector to store paths of matched files
    let mut matched_files = Vec::new();

    // 3. Parse the glob pattern and get an iterator of matched entries
    let entries = glob(glob_pattern)
        .map_err(|e| CheckFileError::GlobError(e))?;

    // 4. Iterate over all matched entries, filter files, and collect their paths
    for entry in entries {
        match entry {
            Ok(path) => {
                // Only retain regular files (exclude directories and special files)
                if path.is_file() {
                    // Convert Path to UTF-8 string and store it
                    match path.to_str() {
                        Some(file_path) => {
                            println!("Matched file found: {}", file_path);
                            matched_files.push(file_path.to_string());
                        }
                        None => {
                            // Skip files with non-UTF-8 paths and log a warning
                            eprintln!("Warning: Found a file with non-UTF-8 encoded path, skipped.");
                        }
                    }
                }
            }
            Err(e) => {
                // Return error immediately if iteration fails
                return Err(CheckFileError::GlobIterError(e));
            }
        }
    }

    // 5. Return the collected list of matched file paths
    Ok(matched_files)
}

pub fn check_necessary_files_in_edl_folder(path: &str, is_protect_lun5: bool) -> Result<EdlPackage, CheckFileError> {
    let mut package = EdlPackage {
        is_miss_file: false,
        is_miss_super_image: false,
        super_define: "".to_string(),
        raw_program_files: Vec::<String>::new(),
        raw_programs: Vec::<(String, String)>::new(),
        patch_files: Vec::<String>::new()
        };
    if check_folder_exist(&path) {
        // 1. check necessary json and xml file
        let meta_folder = format!("{}/META", path);
        if check_folder_exist(&meta_folder) {
            let super_define = format!("{}/super_def.*.json", meta_folder);
            match get_matched_files_in_folder(&meta_folder, &super_define) {
                Ok(files) => {
                    if files.is_empty() == false {
                        package.super_define = format!("{}", files.last().unwrap());
                        println!("✅ Folder {} contains super_def.*.json files", &meta_folder);
                    } else {
                        println!("❌ No super_def.*.json files found in folder {}", &meta_folder);
                        return Err(CheckFileError::InvalidPath)
                    }
                }
                Err(e) => {
                    eprintln!("❌ Check failed: {}", e);
                    return Err(CheckFileError::InvalidPath)
                }
            }
        } else {
            eprintln!("Folder not found:{}", &meta_folder);
            return Err(CheckFileError::DirectoryNotFound)
        }

        let img_folder = format!("{}/IMAGES", path);
        if check_folder_exist(&img_folder) {
            match get_matched_files_in_folder(&img_folder, 
                &if is_protect_lun5 {
                    format!("{}/rawprogram[0-4].xml", img_folder)
                } else {
                    format!("{}/rawprogram[0-5].xml", img_folder)
                }) 
            {
                Ok(files) => {
                    if files.is_empty() == false {
                        package.raw_program_files = files;
                        println!("✅ Folder {} contains rawprogram?.xml files", &img_folder);
                    } else {
                        println!("❌ No rawprogram?.xml files found in folder {}", &img_folder);
                        return Err(CheckFileError::InvalidPath)
                    }
                }
                Err(e) => {
                    eprintln!("❌ Check failed: {}", e);
                    return Err(CheckFileError::InvalidPath)
                }
            }

            match get_matched_files_in_folder(&img_folder, 
                &if is_protect_lun5 {
                    format!("{}/patch[0-4].xml", img_folder)
                } else {
                    format!("{}/patch[0-5].xml", img_folder)
                }) 
            {
                Ok(files) => {
                    if files.is_empty() == false {
                        package.patch_files = files;
                        println!("✅ Folder {} contains patch?.xml files", &img_folder);
                    } else {
                        println!("❌ No patch?.xml files found in folder {}", &img_folder);
                        return Err(CheckFileError::InvalidPath)
                    }
                }
                Err(e) => {
                    eprintln!("❌ Check failed: {}", e);
                    return Err(CheckFileError::InvalidPath)
                }
            }
        } else {
            eprintln!("Folder not found:{}", &img_folder);
            return Err(CheckFileError::DirectoryNotFound)
        }

        // 2. parser img name in rawprogram.xml
        let skip_list: Vec<String> = vec![
            "super".to_string(),
            "ocdt".to_string(),
            "persist".to_string(),
            "secdata".to_string(),
            "oplusdycnvbk".to_string(),
            "oplusstanvbk_a".to_string(),
        ];
        for file in &package.raw_program_files {
            let (_file_name, dir_path) = parse_file_path("", &file);
            match read_text_file(&file) {
                Ok(content) => {
                    let items = xml_file_util::parser_program_xml_skip_empty(&dir_path, &content);
                    for (label, file_name, xml_content) in items {
                        if file_name.is_empty() {
                            if label == "super" {
                                package.is_miss_super_image = true;
                                package.raw_programs.push((label, xml_content));
                            } else if skip_list.contains(&label) == false {
                                println!("Label:{}, {}", label, file_name);
                                package.is_miss_file = true;
                                return Err(CheckFileError::InvalidPath)
                            }
                        } else {
                            if skip_list.contains(&label) == false { // skip import partition
                                package.raw_programs.push((label, xml_content));
                            }
                        }
                    }
                },
                Err(_e) => {return Err(CheckFileError::InvalidPath)}
            }
        }
    } else {
        eprintln!("Folder not found:{}", &path);
        return Err(CheckFileError::DirectoryNotFound)
    }
    Ok(package)
}