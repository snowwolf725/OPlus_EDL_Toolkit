mod command_worker;
mod file_util;
mod gpt_parser;
mod qdl;
mod xml_file_util;
mod super_image_creater;

use command_worker::CommandItem;
use serialport::{available_ports, SerialPortType};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Error, command, State};
use tokio::process::Command;
use crate::xml_file_util::DataRoot;


#[derive(Debug, Clone)]
pub struct Config {
    pub fh_loader_path: String,
    
    pub sahara_server_path: String,

    pub fh_loader_path_linux: String,
    
    pub sahara_server_path_linux: String,

    pub fh_port_conn_str: String,

    pub sahara_port_conn_str: String,

    pub fh_port_conn_str_linux: String,

    pub current_dir: PathBuf,

    pub is_connect: bool,
}

struct ThreadState {
    running: AtomicBool,
    thread_handle: Option<thread::JoinHandle<()>>,
}

impl Default for ThreadState {
    fn default() -> Self {
        ThreadState {
            running: AtomicBool::new(false),
            thread_handle: None,
        }
    }
}

async fn exec_cmd(app: &AppHandle, cmd: &[&str], current_dir:&Path) -> String {
    if cmd.is_empty() {
        let _ = app.emit("log_event", "[Error]");
        return "[Error] cmd is empty".to_string();
    }
    let mut exe_cmd = Command::new(cmd[0]);
    #[cfg(target_os = "windows")]
    {
      exe_cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW constant
    }
    let mut cmd_str = format!("{} ", cmd[0]);
    for (_index, s) in cmd.iter().enumerate() {
        if _index != 0 {
            exe_cmd.arg(s);
            cmd_str = format!("{} {}", cmd_str, s);
        }
    }
    let _ = app.emit("log_event", &format!("{}", cmd_str));
    let output = exe_cmd.current_dir(current_dir).output().await;
    
    let result = match output {
        Ok(output) => {
            if output.status.success() {
                let _ = app.emit("log_event", "[OK]");
                println!("{}",String::from_utf8_lossy(&output.stdout).to_string());
                String::from_utf8_lossy(&output.stdout).to_string()
            } else {
                let _ = app.emit("log_event", "[Error]");
                let err_msg = String::from_utf8_lossy(&output.stderr).to_string();
                println!("[Error]: {}", err_msg);
                format!("[Error]: {}", err_msg)
            }
        }
        Err(e) => {
            let _ = app.emit("log_event", "[Error]");
            let err_msg = format!("Execution failed: {}", e);
            println!("[Error]: {}", err_msg);
            format!("[Error]: {}", err_msg)
        }
    };
    return result;
}

fn flash_patch_xml(state: &Arc<std::sync::Mutex<ThreadState>>, app: &AppHandle, 
port_path: &str, folder: &str, files: Vec<String>) -> bool {
    let total = files.len();
    let mut count = 0;

    for file in files {
        if state.lock().unwrap().running.load(Ordering::SeqCst) == false {
            let _ = app.emit("log_event", "Operation canceled by user");
            return false;
        }
        count += 1;
        thread::sleep(Duration::from_secs(1));

        if command_worker::flash_patch_xml(&port_path, &folder, &file) == false {
            let _ = app.emit("log_event", format!("Failed to flash patch: {}", &file));
            let _ = app.emit("stop_edl_flashing", "");
            return false;
        } else {
            println!("Flash patch:{} / {}", (count * 15)/total, total);
            let _ = app.emit("log_event", format!("Flash patch file: {}", &file));
            let _ = app.emit("update_percentage", 80 + (count * 15)/total);
        }
    }
    return true;
}

fn flash_program_xml(state: &Arc<std::sync::Mutex<ThreadState>>, app: &AppHandle, port_path: &str, 
folder: &str, programs: Vec<(String, String)>) -> bool {
    let total = programs.len();
    let mut count = 0;
    
    for (label, program) in programs {
        if state.lock().unwrap().running.load(Ordering::SeqCst) == false {
            let _ = app.emit("log_event", "Operation canceled by user");
            return false;
        }
        count += 1;
        thread::sleep(Duration::from_secs(1));
        
        if command_worker::flash_part(&port_path, &folder, &program) == false {
            let _ = app.emit("log_event", format!("Failed to flash partition: {}", label));
            let _ = app.emit("stop_edl_flashing", "");
            return false;
        } else {
            println!("Flash program:{} / {}", (count * 60)/total, total);
            let _ = app.emit("log_event", format!("Flash partition: {}", label));
            let _ = app.emit("update_percentage", 20 + (count * 60)/total);
        }
    }
    return true;
}

fn print_result(app: &AppHandle, item: CommandItem) {
    println!("[Main/Print] Command: {}, Result: {}", item.cmd, item.exec_result);
    let _ = app.emit("log_event", &format!("Command: {} {:?}", item.cmd, item.args));
    if item.is_success {
        let _ = app.emit("log_event", &format!("{}...OK", item.msg));
    } else {
        let _ = app.emit("log_event", &format!("{}...Error", item.msg));
    }
}

fn setup_env(app: &AppHandle) -> Config {
    let mut config = Config {
        fh_loader_path: String::new(),
        sahara_server_path: String::new(),
        fh_loader_path_linux: String::new(),
        sahara_server_path_linux: String::new(),
        fh_port_conn_str: String::new(),
        sahara_port_conn_str: String::new(),
        fh_port_conn_str_linux: String::new(),
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
    let port_conn_str_linux = r"--port=".to_owned() + &port_path;
    let current_exe = match env::current_exe() {
        Ok(path) => path,
        Err(_e) => return config,
    };
    let parent_dir: PathBuf = current_exe.parent().unwrap_or(Path::new(".")).to_path_buf();
    let tools_dir = parent_dir.join("tools");
    let fhloader_path = tools_dir.join("fh_loader.exe");
    let sahara_server_path = tools_dir.join("QSaharaServer.exe");
    let fhloader_path_linux = tools_dir.join("fh_loader");
    let sahara_server_path_linux = tools_dir.join("QSaharaServer");
    
    config.current_dir = parent_dir;
    config.fh_port_conn_str = port_conn_str;
    config.sahara_port_conn_str = port_str;
    config.fh_port_conn_str_linux = port_conn_str_linux;
    config.fh_loader_path = fhloader_path.to_str().unwrap_or("fh_loader.exe").to_string();
    config.sahara_server_path = sahara_server_path.to_str().unwrap_or("QSaharaServer.exe").to_string();
    config.fh_loader_path_linux = fhloader_path_linux.to_str().unwrap_or("fh_loader").to_string();
    config.sahara_server_path_linux = sahara_server_path_linux.to_str().unwrap_or("QSaharaServer").to_string();
    config.is_connect = !config.fh_port_conn_str.is_empty();
    return config;
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn init(app: AppHandle) {
    let receiver = command_worker::init_worker();
    let value = app.clone();
    std::thread::spawn(move || {
        while let Ok(result_item) = receiver.recv() {
            print_result(&value, result_item);
        }
    });
}

#[tauri::command]
async fn read_device_info(app: AppHandle) -> String {
    let config = setup_env(&app);
    if config.is_connect == false {
        return "Device not found".to_string();
    }

    let _ = app.emit("update_command_running_status", true);
    let cmd = "<?xml version=\"1.0\" ?><data><getstorageinfo physical_partition_number=\"0\" /></data>";
    file_util::write_to_file("cmd.xml", "res", &cmd);
    let _ = app.emit("log_event", &format!("Read Device Info..."));
    #[cfg(target_os = "windows")] {
        let cmds = ["cmd", "/c", &config.fh_loader_path, &config.fh_port_conn_str, "--memoryname=ufs", 
                   "--sendxml=res/cmd.xml", "--noprompt", "--skip_configure", "--mainoutputdir=res"];
        let result = exec_cmd(&app, &cmds, PathBuf::from(".").as_path()).await;
        if result.starts_with("[Error]") == false {
            return file_util::analysis_info(&result);
        }
    }
    #[cfg(target_os = "linux")] {
        let cmds = [&config.fh_loader_path_linux, &config.fh_port_conn_str_linux, "--memoryname=ufs", 
                   "--sendxml=res/cmd.xml", "--noprompt", "--zlpawarehost=1", "--mainoutputdir=res"];
        let result = exec_cmd(&app, &cmds, PathBuf::from(".").as_path()).await;
        if result.starts_with("[Error]") == false {
            return file_util::analysis_info(&result);
        }
    }
    let _ = app.emit("update_command_running_status", false);
    return "".to_string();
}

#[tauri::command]
async fn read_gpt(app: AppHandle) {
    let config = setup_env(&app);
    if config.is_connect == false {
        return ();
    }

    let _ = app.emit("update_command_running_status", true);
    let mut root = DataRoot{programs: Vec::new(), read_tags: Vec::new(),};
    for i in 0..6 {
        let _ = app.emit("log_event", format!("read LUN {}...", i));
        let read_tag = xml_file_util::create_read_tag_dynamic(&format!("gpt_main{}.bin", i), i, 0, 6, "PrimaryGPT");

        let read_xml = xml_file_util::to_xml(&read_tag);
        let xml_content = format!("<?xml version=\"1.0\" ?>\n<data>\n{}\n</data>\n", read_xml);
        file_util::write_to_file("cmd.xml", "res", &xml_content);
        #[cfg(target_os = "windows")] {
            let cmds = ["cmd", "/c", &config.fh_loader_path, &config.fh_port_conn_str, "--memoryname=ufs", 
                "--sendxml=res/cmd.xml", "--convertprogram2read", "--mainoutputdir=img", "--skip_configure", "--showpercentagecomplete", "--noprompt"];
            exec_cmd(&app, &cmds, PathBuf::from(".").as_path()).await;
        }
        #[cfg(target_os = "linux")] {
            let cmds = [&config.fh_loader_path_linux, &config.fh_port_conn_str_linux, "--memoryname=ufs", 
                "--sendxml=res/cmd.xml", "--convertprogram2read", "--mainoutputdir=img", "--zlpawarehost=1", "--showpercentagecomplete", "--noprompt"];
            exec_cmd(&app, &cmds, PathBuf::from(".").as_path()).await;
        }
        
        //parser gpt
        let file_path = format!("img/gpt_main{}.bin", i).to_string();
        let mut parser = gpt_parser::GptParser::new();
        if file_util::check_file_exist(&file_path) == false {
            println!("error");
            return;
        } else {
            match parser.parse_file(file_path, 4096) {
                Ok(_) => {
                    for (_i, partition) in parser.partitions().iter().enumerate() {
                        let program = xml_file_util::create_program_dynamic(i, 
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

    let read_xml = xml_file_util::to_xml(&root);
    let _ = app.emit("update_partition_table", &read_xml);
    let _ = app.emit("update_command_running_status", false);
}

#[tauri::command]
fn read_part(app: AppHandle, xml: &str, folder: &str)  -> String {
    let _ = app.emit("update_command_running_status", true);
    // Call the parsing function
    let config = setup_env(&app);
    let items = xml_file_util::parser_read_xml(xml);
    for (part, xml_content) in items {
        let file_name = "res/cmd.xml";
        println!("file:{}", &file_name);
        if let Err(e) = fs::write(&file_name, xml_content) {
            eprintln!("file{}failed:{}", file_name, e);
            continue;
        } else {
            println!("success:{}", file_name);
        }

        if config.is_connect == false {
            return format!("port not available");
        }
        let dir_str = format!("--mainoutputdir={}", &folder);
        let _ = app.emit("log_event", format!("Start reading partition {}", part));
        #[cfg(target_os = "windows")] {
            command_worker::add_command(&format!("Read partition {}...", part), "cmd", 
            vec!["/c", &config.fh_loader_path, &config.fh_port_conn_str, "--memoryname=ufs", "--convertprogram2read",
            "--showpercentagecomplete", "--sendxml=res/cmd.xml", "--noprompt", "--skip_configure", &dir_str]);
        }
        #[cfg(target_os = "linux")] {
            command_worker::add_command(&format!("Read partition {}...", part), &config.fh_loader_path_linux,
            vec![&config.fh_port_conn_str_linux, "--memoryname=ufs", "--convertprogram2read",
            "--showpercentagecomplete", "--sendxml=res/cmd.xml", "--noprompt", "--zlpawarehost=1", &dir_str]);
        }
    }
    let _ = app.emit("update_command_running_status", false);
    return "".to_string();
}

#[tauri::command]
fn reboot_to_edl(app: AppHandle) {
    let config = setup_env(&app);
    if config.is_connect == false {
        return ();
    }
    let cmd = "<?xml version=\"1.0\" ?><data><power DelayInSeconds=\"0\" value=\"reset_to_edl\" /></data>";
    file_util::write_to_file("cmd.xml", "res", &cmd);
    #[cfg(target_os = "windows")] {
        command_worker::add_command("Reboot to EDL", "cmd", 
            vec!["/c", &config.fh_loader_path, &config.fh_port_conn_str, "--memoryname=ufs", 
            "--sendxml=res/cmd.xml", "--noprompt", "--skip_configure", "--mainoutputdir=res"]);
    }
    #[cfg(target_os = "linux")] {
        command_worker::add_command("Reboot to EDL", &config.fh_loader_path_linux, 
            vec![&config.fh_port_conn_str_linux, "--memoryname=ufs", "--sendxml=res/cmd.xml", "--noprompt", 
            "--zlpawarehost=1", "--mainoutputdir=res"]);
    }
    let _ = app.emit("update_loader_status", false);
}

#[tauri::command]
fn reboot_to_fastboot(app: AppHandle, xml: &str) {
    let config = setup_env(&app);
    if config.is_connect == false {
        return ();
    }
    // flash misc partition
    file_util::write_to_file("cmd.xml", "res", &xml);
    let cmd = "<?xml version=\"1.0\" ?><data><power DelayInSeconds=\"0\" value=\"reset\" /></data>";
    file_util::write_to_file("cmd.xml", "res", &cmd);

    #[cfg(target_os = "windows")] {
        command_worker::add_command("Writ misc partition", "cmd", 
            vec!["/c", &config.fh_loader_path, &config.fh_port_conn_str, 
            "--memoryname=ufs", "--search_path=res", "--showpercentagecomplete", "--sendxml=res/cmd1.xml", 
            "--noprompt", "--skip_configure", "--mainoutputdir=res"]);
        // send reboot command
        command_worker::add_command("Reboot to fastbootD", "cmd", 
            vec!["/c", &config.fh_loader_path, &config.fh_port_conn_str, "--memoryname=ufs", 
            "--sendxml=res/cmd.xml", "--noprompt", "--skip_configure", "--mainoutputdir=res"]);
    }
    #[cfg(target_os = "linux")] {
        command_worker::add_command("Writ misc partition", &config.fh_loader_path_linux, 
            vec![&config.fh_port_conn_str_linux, "--memoryname=ufs", "--search_path=res", 
            "--showpercentagecomplete", "--sendxml=res/cmd1.xml", "--noprompt", 
            "--zlpawarehost=1", "--mainoutputdir=res"]);
        // send reboot command
        command_worker::add_command("Reboot to fastbootD", &config.fh_loader_path_linux, 
            vec![&config.fh_port_conn_str_linux, "--memoryname=ufs", "--sendxml=res/cmd.xml", "--noprompt", 
            "--zlpawarehost=1", "--mainoutputdir=res"]);
    }
    let _ = app.emit("update_loader_status", false);
}

#[tauri::command]
fn reboot_to_recovery(app: AppHandle, xml: &str) {
    let config = setup_env(&app);
    if config.is_connect == false {
        return ();
    }
    // flash misc partition
    file_util::write_to_file("cmd1.xml", "res", &xml);
    let cmd = "<?xml version=\"1.0\" ?><data><power DelayInSeconds=\"0\" value=\"reset\" /></data>";
    file_util::write_to_file("cmd.xml", "res", &cmd);
    
    let _ = app.emit("log_event", &format!("Writ misc partition ..."));
    #[cfg(target_os = "windows")] {
        command_worker::add_command("Writ misc partition", "cmd", 
            vec!["/c", &config.fh_loader_path, &config.fh_port_conn_str, 
            "--memoryname=ufs", "--search_path=res", "--showpercentagecomplete", "--sendxml=res/cmd1.xml", 
            "--noprompt", "--skip_configure", "--mainoutputdir=res"]);
        // send reboot command
        command_worker::add_command("Reboot to recovery", "cmd", 
            vec!["/c", &config.fh_loader_path, &config.fh_port_conn_str, "--memoryname=ufs", 
            "--sendxml=res/cmd.xml", "--noprompt", "--skip_configure", "--mainoutputdir=res"]);
    }
    #[cfg(target_os = "linux")] {
        command_worker::add_command("Writ misc partition", &config.fh_loader_path_linux, 
            vec![&config.fh_port_conn_str_linux, "--memoryname=ufs", "--search_path=res", 
            "--showpercentagecomplete", "--sendxml=res/cmd1.xml", "--noprompt", 
            "--zlpawarehost=1", "--mainoutputdir=res"]);
        // send reboot command
        command_worker::add_command("Reboot to recovery", &config.fh_loader_path_linux, 
            vec![&config.fh_port_conn_str_linux, "--memoryname=ufs", "--sendxml=res/cmd.xml", "--noprompt", 
            "--zlpawarehost=1", "--mainoutputdir=res"]);
    }
    let _ = app.emit("update_loader_status", false);
}

#[tauri::command]
fn reboot_to_system(app: AppHandle) {
    let config = setup_env(&app);
    if config.is_connect == false {
        return ();
    }
    let cmd = "<?xml version=\"1.0\" ?><data><power DelayInSeconds=\"0\" value=\"reset\" /></data>";
    file_util::write_to_file("cmd.xml", "res", &cmd);
    #[cfg(target_os = "windows")] {
        command_worker::add_command("Reboot to system", "cmd", 
        vec!["/c", &config.fh_loader_path, &config.fh_port_conn_str, "--memoryname=ufs", "--sendxml=res/cmd.xml", "--noprompt", "--skip_configure", "--mainoutputdir=res"]);
    }
    #[cfg(target_os = "linux")] {
        command_worker::add_command("Reboot to system", &config.fh_loader_path_linux, 
        vec![&config.fh_port_conn_str_linux, "--memoryname=ufs", "--sendxml=res/cmd.xml", "--noprompt", "--zlpawarehost=1", "--mainoutputdir=res"]);
    }
    let _ = app.emit("update_loader_status", false);
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
fn send_loader(app: AppHandle, loader: &str, digest: &str, sig: &str, native: bool)  -> String {
    let config = setup_env(&app);
    if config.is_connect == false {
        return format!("port not available");
    }
    if native {
        let (port_path, _port_info) = update_port();
        let mut client = match qdl::SaharaClient::new(Some(port_path)) {
            Ok(client) => client,
            Err(_e) => return format!("Sahara connect error: {}", _e),
        };
        let _ = app.emit("log_event", &format!("Chip serial number: {}", client.get_chip_sn()));
        let _ = app.emit("log_event", &format!("OEM Private Key hash: {}", client.get_oem_key_hash()));
        client.send_loader(loader);
    } else {
        let loader_str = r"13:".to_owned() + loader;
        let digest_str = r"--signeddigests=".to_owned() + digest;
        let sig_str = r"--signeddigests=".to_owned() + sig;
        #[cfg(target_os = "windows")] {
            command_worker::add_command_without_notify("Send Loader", "cmd", 
            vec!["/c", &config.sahara_server_path, "-p", &config.sahara_port_conn_str, "-s", &loader_str]);
            
            command_worker::add_command_without_notify("Send Digest", "cmd", 
            vec!["/c", &config.fh_loader_path, &config.fh_port_conn_str, &digest_str, "--testvipimpact", "--noprompt", "--skip_configure", "--mainoutputdir=res"]);

            command_worker::add_command_without_notify("Send Transfer Config", "cmd", 
            vec!["/c", &config.fh_loader_path, &config.fh_port_conn_str, "--sendxml=res/transfercfg.xml", "--noprompt", "--skip_configure", "--mainoutputdir=res"]);

            command_worker::add_command_without_notify("Send Verify", "cmd", 
            vec!["/c", &config.fh_loader_path, &config.fh_port_conn_str, "--sendxml=res/verify.xml", "--noprompt", "--skip_configure", "--mainoutputdir=res"]);

            command_worker::add_command_without_notify("Send Sig", "cmd", 
            vec!["/c", &config.fh_loader_path, &config.fh_port_conn_str, &sig_str, "--testvipimpact", "--noprompt", "--skip_configure", "--mainoutputdir=res"]);

            command_worker::add_command_without_notify("Send SHA256 init", "cmd", 
            vec!["/c", &config.fh_loader_path, &config.fh_port_conn_str, "--sendxml=res/sha256init.xml", "--noprompt", "--skip_configure", "--mainoutputdir=res"]);

            command_worker::add_command("Send Storage Config", "cmd", 
            vec!["/c", &config.fh_loader_path, &config.fh_port_conn_str, "--memoryname=ufs", "--sendxml=res/cfg.xml", "--search_path=res", "--noprompt", "--mainoutputdir=res"]);
        }
        #[cfg(target_os = "linux")] {
            let (port_path, _port_info) = update_port();
            command_worker::add_command_without_notify("Send Loader", &config.sahara_server_path_linux, 
            vec!["-p", &port_path, "-s", &loader_str]);
            
            command_worker::add_command_without_notify("Send Digest", &config.fh_loader_path_linux, 
            vec![&config.fh_port_conn_str_linux, &digest_str, "--testvipimpact", "--noprompt", "--mainoutputdir=res"]);

            command_worker::add_command_without_notify("Send Transfer Config", &config.fh_loader_path_linux, 
            vec![&config.fh_port_conn_str_linux, "--sendxml=res/transfercfg.xml", "--noprompt", "--mainoutputdir=res"]);

            command_worker::add_command_without_notify("Send Verify", &config.fh_loader_path_linux, 
            vec![&config.fh_port_conn_str_linux, "--sendxml=res/verify.xml", "--noprompt", "--mainoutputdir=res"]);

            command_worker::add_command_without_notify("Send Sig", &config.fh_loader_path_linux, 
            vec![&config.fh_port_conn_str_linux, &sig_str, "--testvipimpact", "--noprompt", "--mainoutputdir=res"]);

            command_worker::add_command("Send SHA256 init", &config.fh_loader_path_linux, 
            vec![&config.fh_port_conn_str_linux, "--sendxml=res/sha256init.xml", "--memoryname=ufs", "--zlpawarehost=1", "--noprompt", "--mainoutputdir=res"]);
        }
        let _ = app.emit("update_loader_status", true);
        let _ = app.emit("update_command_running_status", false);
    }
    format!("OK")
}

#[tauri::command]
async fn send_ping(app: AppHandle) {
    let config = setup_env(&app);
    if config.is_connect == false {
        let _ = app.emit("log_event", "Device not found");
        return;
    }
    let cmd = "<?xml version=\"1.0\" ?><data><nop verbose=\"0\" value=\"ping\"/></data>".to_string();
    file_util::write_to_file("cmd.xml", "res", &cmd);
    let _ = app.emit("log_event", "Send Ping Command");
    let mut result = String::new();
    #[cfg(target_os = "windows")] {
        let cmds = ["cmd", "/c", &config.fh_loader_path, &config.fh_port_conn_str, "--memoryname=ufs", 
                   "--sendxml=res/cmd.xml", "--noprompt", "--skip_configure", "--mainoutputdir=res"];
        result = exec_cmd(&app, &cmds, PathBuf::from(".").as_path()).await;
    }
    #[cfg(target_os = "linux")] {
        let cmds = [&config.fh_loader_path_linux, &config.fh_port_conn_str_linux, "--memoryname=ufs", 
                   "--sendxml=res/cmd.xml", "--noprompt", "--zlpawarehost=1", "--mainoutputdir=res"];
        result = exec_cmd(&app, &cmds, PathBuf::from(".").as_path()).await;
    }
}

#[tauri::command] 
fn start_flashing(app: AppHandle, path: String, is_protect_lun5: bool, thread_state: State<Arc<Mutex<ThreadState>>>,) -> Result<(), String> {
    // lock thread state
    let mut state_guard = thread_state.lock().map_err(|e| format!("lock thread state faild: {}", e))?;

    // if running then return
    if state_guard.running.load(Ordering::SeqCst) {
        return Ok(());
    }
    // set status to running
    state_guard.running.store(true, Ordering::SeqCst);

    // clone state, app for thread using
    let state_clone = thread_state.inner().clone();
    let app_clone = app.clone();

    // create thread
    let handle = thread::spawn(move || {
        let _ = app_clone.emit("update_command_running_status", true);
        let _ = app_clone.emit("update_percentage", 5);
        match file_util::check_necessary_files_in_edl_folder(&path, is_protect_lun5) {
            Ok(package) => {
                if package.is_miss_file == false {
                    let _ = app_clone.emit("log_event", &format!("Check necessary files...OK"));
                    let _ = app_clone.emit("update_percentage", 10);
                    let _ = app_clone.emit("log_event", &format!("Merging Super image..."));
                    if super_image_creater::creat_super_image(&package.super_define) == false {
                        let _ = app_clone.emit("stop_edl_flashing", "");
                        let _ = app_clone.emit("log_event", "Failed to create Super image.");
                        let _ = app_clone.emit("log_event", "The flashing operation has been stopped");
                        state_clone.lock().unwrap().running.store(false, Ordering::SeqCst);
                        let _ = app_clone.emit("update_command_running_status", false);
                        return;
                    }
                    if state_clone.lock().unwrap().running.load(Ordering::SeqCst) == false {
                        let _ = app_clone.emit("log_event", "Operation canceled by user");
                        let _ = app_clone.emit("log_event", "The flashing operation has been stopped");
                        let _ = app_clone.emit("update_command_running_status", false);
                        return;
                    }
                    let _ = app_clone.emit("log_event", &format!("Merge Super image...OK"));
                    let _ = app_clone.emit("update_percentage", 20);
                    let (port_path, _port_info) = update_port();
                    if port_path == "Not found" {
                        let _ = app_clone.emit("log_event", &format!("Port not available"));
                        let _ = app_clone.emit("update_command_running_status", false);
                        return;
                    }
                    let (_file_name, dir_path) = file_util::parse_file_path("", &package.patch_files[0]);
                    if flash_program_xml(&state_clone, &app_clone, &port_path, &dir_path, package.raw_programs) == false {
                        let _ = app_clone.emit("log_event", "The flashing operation has been stopped");
                        state_clone.lock().unwrap().running.store(false, Ordering::SeqCst);
                        let _ = app_clone.emit("update_command_running_status", false);
                        return;
                    }
                    let _ = app_clone.emit("update_percentage", 80);
                    if flash_patch_xml(&state_clone, &app_clone, &port_path, &dir_path, package.patch_files) == false {
                        let _ = app_clone.emit("log_event", "The flashing operation has been stopped");
                        state_clone.lock().unwrap().running.store(false, Ordering::SeqCst);
                        let _ = app_clone.emit("update_command_running_status", false);
                        return;
                    }
                    let _ = app_clone.emit("update_percentage", 95);
                    if command_worker::switch_slot(&port_path, "A") == false {
                        let _ = app_clone.emit("log_event", "The flashing operation has been stopped");
                        state_clone.lock().unwrap().running.store(false, Ordering::SeqCst);
                        let _ = app_clone.emit("update_command_running_status", false);
                        return;
                    }
                    let _ = app_clone.emit("update_percentage", 100);
                    let _ = app_clone.emit("update_command_running_status", false);
                } else {
                    let _ = app_clone.emit("log_event", &format!("Check necessary files...Error"));
                }
            },
            Err(_e) => {
                let _ = app_clone.emit("log_event", &format!("Check necessary files...Error"));
            }
        }

        state_clone.lock().unwrap().running.store(false, Ordering::SeqCst);
        let _ = app_clone.emit("log_event", "The flashing operation has been stopped");
    });

    // store handler to global state
    state_guard.thread_handle = Some(handle);
    Ok(())
}

#[tauri::command]
fn stop_flashing(app: AppHandle, thread_state: State<Arc<Mutex<ThreadState>>>,) -> Result<(), String> {
    // lock thread state
    let mut state_guard = thread_state.lock().map_err(|e| format!("lock thread state faild: {}", e))?;

    // if not running then return
    if state_guard.running.load(Ordering::SeqCst) == false {
        return Ok(());
    }

    // set running state to false
    state_guard.running.store(false, Ordering::SeqCst);

    let _ = app.emit("log_event", "Stopping the EDL flashing operation");

    Ok(())
}

#[tauri::command]
async fn switch_slot(app: AppHandle, slot: &str) -> Result<String, Error> {
    let config = setup_env(&app);
    if config.is_connect == false {
        return Err(tauri::Error::AssetNotFound("Device not found".to_string()));
    }
    let _ = app.emit("update_command_running_status", true);
    let cmd = if slot == "A" {
        "<?xml version=\"1.0\" ?><data><setbootablestoragedrive value=\"1\" /></data>".to_string()
    } else {
        "<?xml version=\"1.0\" ?><data><setbootablestoragedrive value=\"2\" /></data>".to_string()
    };
    file_util::write_to_file("cmd.xml", "res", &cmd);
    let _ = app.emit("log_event", &format!("Set slot to {}...", slot));
    let mut result = String::new();
    #[cfg(target_os = "windows")] {
        let cmds = ["cmd", "/c", &config.fh_loader_path, &config.fh_port_conn_str, "--memoryname=ufs", 
                   "--sendxml=res/cmd.xml", "--noprompt", "--skip_configure", "--mainoutputdir=res"];
        result = exec_cmd(&app, &cmds, PathBuf::from(".").as_path()).await;
    }
    #[cfg(target_os = "linux")] {
        let cmds = [&config.fh_loader_path_linux, &config.fh_port_conn_str_linux, "--memoryname=ufs", 
                   "--sendxml=res/cmd.xml", "--noprompt", "--zlpawarehost=1", "--mainoutputdir=res"];
        result = exec_cmd(&app, &cmds, PathBuf::from(".").as_path()).await;
    }
    let _ = app.emit("update_command_running_status", false);
    return Ok(result);
}

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
fn write_from_xml(app: AppHandle, file_path:&str) -> String {
    let config = setup_env(&app);
    if config.is_connect == false {
        return format!("Port not available");
    }
    let _ = app.emit("update_command_running_status", true);
    let xml = match file_util::read_text_file(file_path) {
        Ok(content) => content,
        Err(e) => format!("Error reading file: {}", e),
    };
    let (_file_name, dir_path) = file_util::parse_file_path("", file_path);
    let dir_str = format!("--search_path={}", &dir_path);
    
    let items = xml_file_util::parser_program_xml(&dir_path, &xml);
    for (part, xml_content, _dir_path) in items {
        let file_name = "res/cmd.xml";
        println!("file:{}", &file_name);
        if let Err(e) = fs::write(&file_name, xml_content) {
            eprintln!("file{}failed:{}", file_name, e);
            let _ = app.emit("log_event", &format!("file{}failed:{}", file_name, e));
            continue;
        } else {
            println!("success:{}", file_name);
        }

        if config.is_connect == false {
            return format!("port not available");
        }
        #[cfg(target_os = "windows")] {
            command_worker::add_command(&format!("Writ partition {}", part), "cmd", 
                vec!["/c", &config.fh_loader_path, &config.fh_port_conn_str, 
                "--memoryname=ufs", &dir_str, "--showpercentagecomplete", "--sendxml=res/cmd.xml", 
                "--noprompt", "--skip_configure", "--mainoutputdir=res"]);
        }
        #[cfg(target_os = "linux")] {
            command_worker::add_command(&format!("Writ partition {}", part), &config.fh_loader_path_linux, 
                vec![&config.fh_port_conn_str_linux, "--memoryname=ufs", &dir_str, "--showpercentagecomplete",
                 "--sendxml=res/cmd.xml","--noprompt", "--zlpawarehost=1", "--mainoutputdir=res"]);
        }
    }
    let _ = app.emit("update_command_running_status", false);
    return "".to_string();
}

#[tauri::command]
fn write_part(app: AppHandle, xml: &str)  -> String {
    let config = setup_env(&app);
    if config.is_connect == false {
        return format!("port not available");
    }
    let _ = app.emit("update_command_running_status", true);
    let items = xml_file_util::parser_program_xml("", xml);
    for (part, xml_content, dir_path) in items {
        let file_name = "res/cmd.xml";
        println!("file:{}", &file_name);
        if let Err(e) = fs::write(&file_name, xml_content) {
            eprintln!("file{}failed:{}", file_name, e);
            let _ = app.emit("log_event", &format!("file{}failed:{}", file_name, e));
            continue;
        } else {
            println!("success:{}", file_name);
        }
        
        let dir_str = format!("--search_path={}", &dir_path);
        let _ = app.emit("log_event", format!("Start writing partition {}", part));
        #[cfg(target_os = "windows")] {
            command_worker::add_command(&format!("Writ partition {}...", part), "cmd", 
            vec!["/c", &config.fh_loader_path, &config.fh_port_conn_str, 
            "--memoryname=ufs", &dir_str, "--showpercentagecomplete", "--sendxml=res/cmd.xml", 
            "--noprompt", "--skip_configure", "--mainoutputdir=res"]);
        }
        #[cfg(target_os = "linux")] {
            command_worker::add_command(&format!("Writ partition {}...", part), &config.fh_loader_path_linux,
            vec![&config.fh_port_conn_str_linux, "--memoryname=ufs", &dir_str, "--showpercentagecomplete", 
            "--sendxml=res/cmd.xml", "--noprompt", "--zlpawarehost=1", "--mainoutputdir=res"]);
        }
    }
    let _ = app.emit("update_command_running_status", false);
    return "".to_string();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(Arc::new(Mutex::new(ThreadState::default())))
        .invoke_handler(tauri::generate_handler![init, read_device_info, read_gpt, read_part, 
        reboot_to_edl, reboot_to_fastboot, reboot_to_recovery, reboot_to_system, 
        save_to_xml, send_ping, send_loader, start_flashing, stop_flashing, switch_slot, update_port, write_from_xml, write_part])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
