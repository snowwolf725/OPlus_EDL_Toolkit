use crate::file_util;
use serde::Deserialize;
use std::env;
use std::fs::File;
use std::path::Path;
use std::path::PathBuf;
use thiserror::Error;
use std::process::Command;

// ======================== 1. Custom Error Type (Optional, improves error handling) ========================
#[derive(Error, Debug)]
pub enum JsonParseError {
    #[error("File operation error: {0}")]
    FileError(#[from] std::io::Error),
    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),
}

// ======================== 2. Define Structs Matching JSON Structure ========================
/// Top-level struct corresponding to the entire JSON configuration
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")] // Ensure field names match JSON's snake_case convention
pub struct PartitionConfig {
    pub super_meta: SuperMeta,
    pub nv_text: String,
    pub block_devices: Vec<BlockDevice>,
    pub groups: Vec<Group>,
    pub nv_id: String,
    pub partitions: Vec<Partition>,
}

/// Struct for the "super_meta" sub-object in JSON
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SuperMeta {
    pub path: String,
    pub size: String, // Size stored as string (JSON uses string-encoded numbers; convert later if needed)
}

/// Struct for elements in the "block_devices" array
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct BlockDevice {
    pub block_size: String,
    pub name: String,
    pub alignment: String,
    pub size: String,
}

/// Struct for elements in the "groups" array (maximum_size is optional)
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Group {
    pub name: String,
    #[serde(default)] // Assign empty string if field is missing in JSON
    pub maximum_size: String,
}

/// Struct for elements in the "partitions" array (path/size are optional)
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Partition {
    pub is_dynamic: bool,
    pub name: String,
    pub group_name: String,
    #[serde(default)] // Optional field: empty string if missing
    pub path: String,
    #[serde(default)] // Optional field: empty string if missing
    pub size: String,
}

// ======================== 3. Core Function: Read JSON and Parse to Struct ========================
/// Reads a JSON file from the specified path and parses it into a PartitionConfig struct
/// 
/// # Arguments
/// * `path` - Path to the JSON configuration file (e.g., "partition_config.json")
/// 
/// # Returns
/// * `Ok(PartitionConfig)` - Successfully parsed configuration
/// * `Err(JsonParseError)` - Failed to open file or parse JSON
pub fn read_partition_config<P: AsRef<Path>>(path: P) -> Result<PartitionConfig, JsonParseError> {
    // 1. Open the JSON file
    let file = File::open(path)?;

    // 2. Parse JSON from file stream into PartitionConfig struct (serde_json auto-maps fields)
    let config = serde_json::from_reader(file)?;

    // 3. Return parsed configuration
    Ok(config)
}

fn exec_cmd(cmd: &str, args: Vec<String>, current_dir:&Path) -> String {
    if cmd.is_empty() {
        return "[Error] cmd is empty".to_string();
    }
    let mut exe_cmd = Command::new(cmd);
    #[cfg(target_os = "windows")]
    {
      use std::os::windows::process::CommandExt;
      exe_cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW constant
    }
    let mut cmd_str = format!("{} ", cmd);
    for (_index, s) in args.iter().enumerate() {
        exe_cmd.arg(s);
        cmd_str = format!("{} {}", cmd_str, s);
    }
    println!("cmd: {}", cmd_str);
    let output = exe_cmd.current_dir(current_dir).output();
    
    let result = match output {
        Ok(output) => {
            if output.status.success() {
                println!("{}",String::from_utf8_lossy(&output.stdout).to_string());
                String::from_utf8_lossy(&output.stdout).to_string()
            } else {
                let err_msg = String::from_utf8_lossy(&output.stderr).to_string();
                println!("[Error]: {}", err_msg);
                format!("[Error]: {}", err_msg)
            }
        }
        Err(e) => {
            let err_msg = format!("Execution failed: {}", e);
            println!("[Error]: {}", err_msg);
            format!("[Error]: {}", err_msg)
        }
    };
    return result;
}

struct EnvConfig {
    lpmake_path: String,
    lpmake_path_linux: String,
    simg2img_path: String,
    simg2img_path_linux: String,
    work_dir: PathBuf,
}

fn get_runtime_env(path: &str) -> EnvConfig {
    let mut config = EnvConfig {
        lpmake_path: String::new(),
        lpmake_path_linux: String::new(),
        simg2img_path: String::new(),
        simg2img_path_linux: String::new(),
        work_dir: PathBuf::new(),
    };
    let current_exe = match env::current_exe() {
        Ok(path) => path,
        Err(_e) => return config,
    };
    let parent_dir: PathBuf = current_exe.parent().unwrap_or(Path::new(".")).to_path_buf();
    let tools_dir = parent_dir.join("tools");
    let lpmake_path = tools_dir.join("lpmake.exe");
    let lpmake_path_linux = tools_dir.join("lpmake");
    let simg2img_path = tools_dir.join("simg2img.exe");
    let simg2img_path_linux = tools_dir.join("simg2img");

    let (_file_name, dir_path) = file_util::parse_file_path("", &path);
    let work_dir = PathBuf::from(dir_path).parent().unwrap_or(Path::new(".")).to_path_buf();

    config.lpmake_path = lpmake_path.to_str().unwrap_or("lpmake.exe").to_string();
    config.lpmake_path_linux = lpmake_path_linux.to_str().unwrap_or("lpmake").to_string();
    config.simg2img_path = simg2img_path.to_str().unwrap_or("simg2img.exe").to_string();
    config.simg2img_path_linux = simg2img_path_linux.to_str().unwrap_or("simg2img").to_string();
    config.work_dir = work_dir;
    return config;
}

pub fn creat_super_image(path: &str) -> bool {
    let define_path = Path::new(path);
    match read_partition_config(define_path) {
         Ok(config) => {
             if config.block_devices.len() > 0 && config.groups.len() > 0 {
                 let mut args = Vec::<String>::new();
                 args.push(format!("--device-size={}", config.block_devices[0].size));
                 args.push(format!("--metadata-size={}", config.super_meta.size));
                 args.push(format!("--metadata-slots={}", config.groups.len()));
                 args.push(format!("--super-name={}", config.block_devices[0].name));
                 args.push(format!("--virtual-ab"));
                 args.push(format!("-block-size={}", config.block_devices[0].block_size));
                 args.push(format!("--sparse"));
                 
                 for group in config.groups {
                     if group.maximum_size.is_empty() == false {
                         args.push(format!("--group={}:{}", group.name, group.maximum_size));
                     }
                 }

                 for partition in config.partitions {
                     if partition.size.is_empty() {
                         args.push(format!("--partition"));
                         args.push(format!("{}:none:0:{}", partition.name, partition.group_name));
                     } else {
                         args.push(format!("--partition"));
                         args.push(format!("{}:readonly:{}:{}", partition.name, partition.size, partition.group_name));
                         args.push(format!("--image"));
                         args.push(format!("{}={}", partition.name, partition.path));
                     }
                 }
                 args.push(format!("-F"));
                 args.push(format!("--output"));
                 args.push(format!("IMAGES/super.img"));

                 let config = get_runtime_env(&path);
                 exec_cmd(&config.lpmake_path, args, config.work_dir.as_path());
             } else {
                 return false;
             }
         },
         Err(_e) => { return false; }
    }
    return true;
}
