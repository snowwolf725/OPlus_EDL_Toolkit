mod gpt_parser;
use serialport::{available_ports, SerialPortType};
use std::env;
use std::fs;
use std::fs::metadata;
use std::path::{Path, PathBuf};
use std::io;
use std::process::Command;
use tauri::{AppHandle, Emitter};
use quick_xml::de::from_str;
use quick_xml::se::to_string;
use serde::{Deserialize, Serialize};

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
}

// Define struct for the <program> node (matches all attributes)
#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename = "program")]
pub struct Program {
    #[serde(rename = "@start_sector")]
    pub start_sector: u64,
    #[serde(rename = "@size_in_KB")]
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

#[derive(Debug, Clone)]
pub struct Config {
    pub fh_loader_path: String,
    
    pub sahara_server_path: String,

    pub port_conn_str: String,

    pub port_str: String,

    pub current_dir: PathBuf,

    pub is_connect: bool,
}

fn create_program_dynamic(
    lun: u8,
    start_sector: u64,
    num_partition_sectors: u64,
    label: &str
) -> Program {
    let size_in_kb = num_partition_sectors as f64 * 4.0;
    let sector_size = 4096;
    let sector_size_in_bytes = sector_size * num_partition_sectors;
    let start_byte = start_sector as u64 * sector_size as u64;
    let start_byte_hex = format!("{:X}", start_byte);

    Program {
        start_sector,
        size_in_kb,
        physical_partition_number: lun,
        part_of_single_image: false,
        file_sector_offset: 0,
        num_partition_sectors,
        readback_verify: false,
        filename: format!("{}.img", label).to_string(),
        sparse: false,
        start_byte_hex,
        sector_size_in_bytes: sector_size_in_bytes,
        label: label.to_string(),
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

pub fn check_exist(file_path: &str) -> bool {
    let path = Path::new(file_path);

    let file_metadata = match metadata(path) {
        Ok(meta) => meta,
        Err(_) => return false,
    };

    file_metadata.is_file() && file_metadata.len() > 0
}


pub fn parse_file_path(full_path: &str) -> (String, String) {
    // Convert the input string to a Path object for path manipulation
    let path = Path::new(full_path);
    if check_exist(full_path) == false {
        return ("".to_string(), "".to_string());
    }

    // 1. Extract the file name from the path with error handling
    let file_name = path
        .file_name()
        .ok_or_else(|| format!("Failed to extract file name from path '{}'", full_path)) // Return error string
        .and_then(|os_str| {
            // Convert OsStr to &str, return error if invalid Unicode
            os_str.to_str()
                .ok_or_else(|| format!("Path '{}' contains invalid Unicode characters", full_path))
        })
        .unwrap() // Panic on error (simple error handling for this example)
        .to_string();

    // 2. Extract the parent directory of the file with error handling
    let directory = path
        .parent()
        .ok_or_else(|| format!("Failed to extract directory from path '{}' (may be root directory)", full_path))
        .and_then(|path| {
            // Convert Path to &str, return error if invalid Unicode
            path.to_str()
                .ok_or_else(|| format!("Directory path '{}' contains invalid Unicode characters", full_path))
        })
        .unwrap() // Panic on error (simple error handling for this example)
        .to_string();

    (file_name, directory)
}


fn setup_env(app: &AppHandle) -> Config {
    let mut config = Config {
        fh_loader_path: String::new(),
        sahara_server_path: String::new(),
        port_conn_str: String::new(),
        port_str: String::new(),
        current_dir: PathBuf::new(),
        is_connect: false,
    };
    let (port_path, _port_info) = update_port();
    if port_path == "Not found" {
        let _ = app.emit("log_event", &format!("Port not available"));
        return config;
    }
    let port_str = r"\\.\".to_owned() + &port_path;
    let port_conn_str = r"--port=\\.\".to_owned() + &port_path;
    let current_exe = match env::current_exe() {
        Ok(path) => path,
        Err(_e) => return config,
    };
    let parent_dir: PathBuf = current_exe.parent().unwrap_or(Path::new(".")).to_path_buf();
    let tools_dir = parent_dir.join("tools");
    let fhloader_path = tools_dir.join("fh_loader.exe");
    let sahara_server_path = tools_dir.join("QSaharaServer.exe");
    
    config.current_dir = parent_dir;
    config.port_conn_str = port_conn_str;
    config.port_str = port_str;
    config.fh_loader_path = fhloader_path.to_str().unwrap_or("fh_loader.exe").to_string();
    config.sahara_server_path = sahara_server_path.to_str().unwrap_or("QSaharaServer.exe").to_string();
    config.is_connect = !config.port_conn_str.is_empty();
    return config;
}

fn create_dir_if_not_exists(path: &str) -> io::Result<()> {
    let dir_path = Path::new(path);
    if !dir_path.exists() {
        fs::create_dir_all(dir_path)?;
    }
    Ok(())
}

fn write_to_file(path: &str, output_dir: &str, content: &str) {
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

fn exec_cmd(app: &AppHandle, cmd: &[&str], current_dir:&Path) {
    if cmd.is_empty() {
        let _ = app.emit("log_event", "[Error]");
        return;
    }
    let mut exe_cmd = Command::new(cmd[0]);
    let mut cmd_str = format!("{} ", cmd[0]);
    for (_index, s) in cmd.iter().enumerate() {
        if _index != 0 {
            exe_cmd.arg(s);
            cmd_str = format!("{} {}", cmd_str, s);
        }
    }
    let _ = app.emit("log_event", &format!("{}", cmd_str));
    let output = exe_cmd.current_dir(current_dir).output();
    
    let _result = match output {
        Ok(output) => {
            if output.status.success() {
                let _ = app.emit("log_event", "[OK]");
                println!("{}",String::from_utf8_lossy(&output.stdout).to_string());
                String::from_utf8_lossy(&output.stdout).to_string()
            } else {
                let _ = app.emit("log_event", "[Error]");
                let err_msg = String::from_utf8_lossy(&output.stderr).to_string();
                println!("Error: {}", err_msg);
                err_msg
            }
        }
        Err(e) => {
            let _ = app.emit("log_event", "[Error]");
            let err_msg = format!("Execution failed: {}", e);
            println!("Error: {}", err_msg);
            err_msg
        }
    };
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn update_port() -> (String, String) {
    let ports = available_ports().expect("Not found");
    let mut port = String::new();
    let mut product = String::new();
    for p in ports {
        match p.port_type {
             SerialPortType::UsbPort(info) => {
                port = p.port_name;
                if let Some(pinfo) = info.product {
                    println!("product : {}", pinfo);
                    product = pinfo;
                }
             },
             SerialPortType::PciPort | SerialPortType::BluetoothPort | SerialPortType::Unknown => {}
        }
    }
    if port.is_empty() {
        ("Not found".to_string(), "N/A".to_string())
    } else {
        (port, product)
    }
}

#[tauri::command]
fn reboot_to_system(app: AppHandle) {
    let config = setup_env(&app);
    if config.is_connect == false {
        return ();
    }
    let cmd = "<?xml version=\"1.0\" ?><data><power DelayInSeconds=\"0\" value=\"reset\" /></data>";
    write_to_file("cmd.xml", "res", &cmd);
    let _ = app.emit("log_event", &format!("Reboot to system..."));
    let cmds = ["cmd", "/c", &config.fh_loader_path, &config.port_conn_str, "--memoryname=ufs", "--sendxml=res/cmd.xml", "--noprompt", "--skip_configure", "--mainoutputdir=res"];
    exec_cmd(&app, &cmds, PathBuf::from(".").as_path());
}

#[tauri::command]
fn reboot_to_recovery(app: AppHandle, xml: &str) {
    let config = setup_env(&app);
    if config.is_connect == false {
        return ();
    }
    // flash misc partition
    write_to_file("cmd1.xml", "res", &xml);
    let _ = app.emit("log_event", &format!("Writ misc partition ..."));
    let cmds = ["cmd", "/c", &config.fh_loader_path, &config.port_conn_str, 
    "--memoryname=ufs", "--search_path=res", "--showpercentagecomplete", "--sendxml=res/cmd1.xml", 
    "--noprompt", "--skip_configure", "--mainoutputdir=res"];
    exec_cmd(&app, &cmds, PathBuf::from(".").as_path());
    // send reboot command
    let cmd = "<?xml version=\"1.0\" ?><data><power DelayInSeconds=\"0\" value=\"reset\" /></data>";
    write_to_file("cmd.xml", "res", &cmd);
    let _ = app.emit("log_event", &format!("Reboot to recovery..."));
    let cmds = ["cmd", "/c", &config.fh_loader_path, &config.port_conn_str, "--memoryname=ufs", "--sendxml=res/cmd.xml", "--noprompt", "--skip_configure", "--mainoutputdir=res"];
    exec_cmd(&app, &cmds, PathBuf::from(".").as_path());
}

#[tauri::command]
fn reboot_to_fastboot(app: AppHandle, xml: &str) {
    let config = setup_env(&app);
    if config.is_connect == false {
        return ();
    }
    // flash misc partition
    write_to_file("cmd.xml", "res", &xml);
    let _ = app.emit("log_event", &format!("Writ misc partition ..."));
    let cmds = ["cmd", "/c", &config.fh_loader_path, &config.port_conn_str, 
    "--memoryname=ufs", "--search_path=res", "--showpercentagecomplete", "--sendxml=res/cmd.xml", 
    "--noprompt", "--skip_configure", "--mainoutputdir=res"];
    exec_cmd(&app, &cmds, PathBuf::from(".").as_path());
    // send reboot command
    let cmd = "<?xml version=\"1.0\" ?><data><power DelayInSeconds=\"0\" value=\"reset\" /></data>";
    write_to_file("cmd.xml", "res", &cmd);
    let _ = app.emit("log_event", &format!("Reboot to fastbootD..."));
    let cmds = ["cmd", "/c", &config.fh_loader_path, &config.port_conn_str, "--memoryname=ufs", "--sendxml=res/cmd.xml", "--noprompt", "--skip_configure", "--mainoutputdir=res"];
    exec_cmd(&app, &cmds, PathBuf::from(".").as_path());
}

#[tauri::command]
fn reboot_to_edl(app: AppHandle) {
    let config = setup_env(&app);
    if config.is_connect == false {
        return ();
    }
    let cmd = "<?xml version=\"1.0\" ?><data><power DelayInSeconds=\"0\" value=\"reset_to_edl\" /></data>";
    write_to_file("cmd.xml", "res", &cmd);
    let _ = app.emit("log_event", &format!("Reboot to EDL..."));
    let cmds = ["cmd", "/c", &config.fh_loader_path, &config.port_conn_str, "--memoryname=ufs", "--sendxml=res/cmd.xml", "--noprompt", "--skip_configure", "--mainoutputdir=res"];
    exec_cmd(&app, &cmds, PathBuf::from(".").as_path());
}

#[tauri::command]
fn save_to_xml(app: AppHandle, path: &str, xml: &str) {
    if let Err(e) = fs::write(&path, xml) {
        let _ = app.emit("log_event", &format!("Save file{} failed:{}", path, e));
        eprintln!("file{}failed:{}", path, e);
    } else {
        let _ = app.emit("log_event", &format!("success:{}", path));
        println!("success:{}", path);
    }
}

#[tauri::command]
fn write_part(app: AppHandle, xml: &str)  -> String {
// Call the parsing function
    match from_str::<DataRoot>(&xml) {
        Ok(data_root) => {
            let output_dir = "res";
            if let Err(e) = create_dir_if_not_exists(output_dir) {
                return format!("create res dir failed:{}", e);
            }
            // Iterate and print each program
            for mut program in data_root.programs {
               let (file_name, dir_path) = parse_file_path(&program.filename);
               println!("Test {}, {}, {}", &program.filename, &file_name, &dir_path);
               program.filename = file_name;
               let program_xml = match to_string(&program) {
                    Ok(xml) => xml,
                    Err(_e) => {
                        continue;
                    }
                };
               let xml_content = format!("<?xml version=\"1.0\" ?>\n<data>\n{}\n</data>\n", program_xml);
               let file_name = format!("{}\\cmd.xml", &output_dir);
               println!("file:{}", &file_name);
               if let Err(e) = fs::write(&file_name, xml_content) {
                   eprintln!("file{}failed:{}", file_name, e);
               } else {
                   println!("success:{}", file_name);
               }

               let config = setup_env(&app);
               if config.is_connect == false {
                    return format!("port not available");
               }
               let dir_str = format!("--search_path={}", &dir_path);
               let _ = app.emit("log_event", &format!("Writ partition {}...", program.label));
               let cmds = ["cmd", "/c", &config.fh_loader_path, &config.port_conn_str, 
               "--memoryname=ufs", &dir_str, "--showpercentagecomplete", "--sendxml=res/cmd.xml", 
               "--noprompt", "--skip_configure", "--mainoutputdir=res"];
               exec_cmd(&app, &cmds, config.current_dir.as_path());
            }
            format!("")
        }
        Err(e) => {
            let _ = app.emit("log_event", e.to_string());
            format!("XML parsing failed: {}", e)
        }
    }
}

#[tauri::command]
fn read_part(app: AppHandle, xml: &str)  -> String {
    // Call the parsing function
    match from_str::<DataRoot>(&xml) {
        Ok(data_root) => {
            let output_dir = "res";
            if let Err(e) = create_dir_if_not_exists(output_dir) {
                return format!("create res dir failed:{}", e);
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
               let file_name = format!("{}\\cmd.xml", &output_dir);
               println!("file:{}", &file_name);
               if let Err(e) = fs::write(&file_name, xml_content) {
                   eprintln!("file{}failed:{}", file_name, e);
               } else {
                   println!("success:{}", file_name);
               }

               let config = setup_env(&app);/*
               if config.is_connect == false {
                    return format!("port not available");
               }*/
               let _ = app.emit("log_event", &format!("Read partition {}...", read.label));
               let cmds = ["cmd", "/c", &config.fh_loader_path, &config.port_conn_str, "--memoryname=ufs", 
               "--convertprogram2read", "--showpercentagecomplete", "--sendxml=res/cmd.xml", "--noprompt", "--skip_configure", "--mainoutputdir=img"];
               exec_cmd(&app, &cmds, PathBuf::from(".").as_path());
            }
            format!("")
        }
        Err(e) => {
            let _ = app.emit("log_event", e.to_string());
            format!("XML parsing failed: {}", e)
        }
    }
}

#[tauri::command]
fn send_loader(app: AppHandle, loader: &str, digest: &str, sig: &str)  -> String {
    let config = setup_env(&app);
    if config.is_connect == false {
        return format!("port not available");
    }
    let loader_str = r"13:".to_owned() + loader;
    let digest_str = r"--signeddigests=".to_owned() + digest;
    let sig_str = r"--signeddigests=".to_owned() + sig;
    let _ = app.emit("log_event", &format!("Send Loader..."));
    let cmds = ["cmd", "/c", &config.sahara_server_path, "-p", &config.port_str, "-s", &loader_str];
    exec_cmd(&app, &cmds, PathBuf::from(".").as_path());
    
    let _ = app.emit("log_event", &format!("Send Digest..."));
    let cmds = ["cmd", "/c", &config.fh_loader_path, &config.port_conn_str, &digest_str, "--testvipimpact", "--noprompt", "--skip_configure", "--mainoutputdir=res"];
    exec_cmd(&app, &cmds, PathBuf::from(".").as_path());

    let _ = app.emit("log_event", &format!("Send Transfer Config..."));
    let cmds = ["cmd", "/c", &config.fh_loader_path, &config.port_conn_str, "--sendxml=res/transfercfg.xml", "--noprompt", "--skip_configure", "--mainoutputdir=res"];
    exec_cmd(&app, &cmds, PathBuf::from(".").as_path());

    let _ = app.emit("log_event", &format!("Send Verify..."));
    let cmds = ["cmd", "/c", &config.fh_loader_path, &config.port_conn_str, "--sendxml=res/verify.xml", "--noprompt", "--skip_configure", "--mainoutputdir=res"];
    exec_cmd(&app, &cmds, PathBuf::from(".").as_path());

    let _ = app.emit("log_event", &format!("Send Sig..."));
    let cmds = ["cmd", "/c", &config.fh_loader_path, &config.port_conn_str, &sig_str, "--testvipimpact", "--noprompt", "--skip_configure", "--mainoutputdir=res"];
    exec_cmd(&app, &cmds, PathBuf::from(".").as_path());

    let _ = app.emit("log_event", &format!("Send SHA256 init..."));
    let cmds = ["cmd", "/c", &config.fh_loader_path, &config.port_conn_str, "--sendxml=res/sha256init.xml", "--noprompt", "--skip_configure", "--mainoutputdir=res"];
    exec_cmd(&app, &cmds, PathBuf::from(".").as_path());

    let _ = app.emit("log_event", &format!("Send Memory Config..."));
    let cmds = ["cmd", "/c", &config.fh_loader_path, &config.port_conn_str, "--memoryname=ufs", "--sendxml=res/cfg.xml", "--search_path=res", "--noprompt", "--mainoutputdir=res"];
    exec_cmd(&app, &cmds, PathBuf::from(".").as_path());

    format!("OK")
}

#[tauri::command]
fn write_from_xml(app: AppHandle, file_path:&str) -> String {
    let config = setup_env(&app);
    if config.is_connect == false {
        return format!("Port not available");
    }
    let xml = match read_text_file(file_path) {
        Ok(content) => content,
        Err(e) => format!("Error reading file: {}", e),
    };
    let (_file_name, dir_path) = parse_file_path(file_path);
    let dir_str = format!("--search_path={}", &dir_path);

    match from_str::<DataRoot>(&xml) {
        Ok(data_root) => {
            let output_dir = "res";
            if let Err(e) = create_dir_if_not_exists(output_dir) {
                return format!("create res dir failed:{}", e);
            }
            // Iterate and print each program
            for program in data_root.programs {
               let program_xml = match to_string(&program) {
                    Ok(xml) => xml,
                    Err(_e) => {
                        continue;
                    }
                };
               let xml_content = format!("<?xml version=\"1.0\" ?>\n<data>\n{}\n</data>\n", program_xml);
               let file_name = format!("{}\\cmd.xml", &output_dir);
               println!("file:{}", &file_name);
               if let Err(e) = fs::write(&file_name, xml_content) {
                   eprintln!("file{}failed:{}", file_name, e);
               } else {
                   println!("success:{}", file_name);
               }

               let config = setup_env(&app);
               if config.is_connect == false {
                    return format!("port not available");
               }
               
               let _ = app.emit("log_event", &format!("Writ partition {}...", program.label));
               let cmds = ["cmd", "/c", &config.fh_loader_path, &config.port_conn_str, 
               "--memoryname=ufs", &dir_str, "--showpercentagecomplete", "--sendxml=res/cmd.xml", 
               "--noprompt", "--skip_configure", "--mainoutputdir=res"];
               exec_cmd(&app, &cmds, PathBuf::from(".").as_path());
            }
            format!("")
        }
        Err(e) => {
            let _ = app.emit("log_event", e.to_string());
            format!("XML parsing failed: {}", e)
        }
    }
}

#[tauri::command]
fn read_gpt(app: AppHandle) {
    let mut root = DataRoot{programs: Vec::new(), read_tags: Vec::new(),};

    let config = setup_env(&app);
    if config.is_connect == false {
        return ();
    }
    
    for i in 0..6 {
        let _ = app.emit("log_event", format!("read lun {}", i));
        let read_tag = ReadTag {
            filename: format!("gpt_main{}.bin", i).to_string(),
            physical_partition_number: i,
            label: "PrimaryGPT".to_string(),
            start_sector: 0,
            num_partition_sectors: 6,
            sector_size_in_bytes: 4096,
            sparse: false,
        };

        let read_xml = match to_string(&read_tag) {
             Ok(xml) => xml,
             Err(_e) => {
                "".to_string()
             }
        };
        let xml_content = format!("<?xml version=\"1.0\" ?>\n<data>\n{}\n</data>\n", read_xml);
        write_to_file("cmd.xml", "res", &xml_content);
        let cmds = ["cmd", "/c", &config.fh_loader_path, &config.port_conn_str, "--memoryname=ufs", 
            "--sendxml=res/cmd.xml", "--convertprogram2read", "--mainoutputdir=img", "--skip_configure", "--showpercentagecomplete", "--noprompt"];
        exec_cmd(&app, &cmds, PathBuf::from(".").as_path());
        
        //parser gpt
        let file_path = format!("img/gpt_main{}.bin", i).to_string();
        let mut parser = gpt_parser::GptParser::new();
        if check_exist(&file_path) == false {
            println!("error");
            return;
        } else {
            match parser.parse_file(file_path, 4096) {
                Ok(_) => {
                    for (_i, partition) in parser.partitions().iter().enumerate() {
                        //let mut p_tag = Program::new();
                        let program = create_program_dynamic(i, 
                            partition.first_lba, 
                            partition.size_in_sectors(), 
                            &partition.name);
                        root.programs.push(program);
                    }
                }
                Err(_e) => {

                }
            }
        }
    }

    let read_xml = match to_string(&root) {
        Ok(xml) => xml,
        Err(_e) => {
        "".to_string()
        }
    };
    let _ = app.emit("update_partition_table", &read_xml);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![update_port, send_loader, read_part, write_part, read_gpt,
        reboot_to_system, reboot_to_recovery, reboot_to_fastboot, reboot_to_edl, save_to_xml, write_from_xml])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
