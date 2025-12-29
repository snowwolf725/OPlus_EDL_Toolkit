use serialport::available_ports;
use serialport::SerialPortType;
use std::env;
use std::path::Path;
use std::path::PathBuf;
use tokio::process::Command;
use tauri::AppHandle;
use tauri::Emitter;


#[derive(Debug, Clone)]
pub struct Config {
    pub fh_loader_path: String,
    
    pub sahara_server_path: String,

    pub fh_loader_path_linux: String,
    
    pub sahara_server_path_linux: String,

    pub fh_port_conn_str: String,

    pub sahara_port_conn_str: String,

    pub fh_port_conn_str_linux: String,

    pub sahara_port_conn_str_linux: String,

    pub current_dir: PathBuf,

    pub is_connect: bool,

    pub log_level: LogLevel,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LogLevel {
    Info,
    Debug,
}

impl Config {
    
    pub fn setup_env(debug: bool) -> Self {
        let mut config = Self {
            fh_loader_path: String::new(),
            sahara_server_path: String::new(),
            fh_loader_path_linux: String::new(),
            sahara_server_path_linux: String::new(),
            fh_port_conn_str: String::new(),
            sahara_port_conn_str: String::new(),
            fh_port_conn_str_linux: String::new(),
            sahara_port_conn_str_linux: String::new(),
            current_dir: PathBuf::new(),
            is_connect: false,
            log_level: LogLevel::Info,
        };
        let (port_path, _port_info) = update_port();
        if port_path == "Not found" {
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
        let log_level = match debug {
            true => LogLevel::Debug,
            false => LogLevel::Info,
        };

    
        config.current_dir = parent_dir;
        config.fh_port_conn_str = port_conn_str;
        config.sahara_port_conn_str = port_str;
        config.fh_port_conn_str_linux = port_conn_str_linux;
        config.sahara_port_conn_str_linux = port_path;
        config.fh_loader_path = fhloader_path.to_str().unwrap_or("fh_loader.exe").to_string();
        config.sahara_server_path = sahara_server_path.to_str().unwrap_or("QSaharaServer.exe").to_string();
        config.fh_loader_path_linux = fhloader_path_linux.to_str().unwrap_or("fh_loader").to_string();
        config.sahara_server_path_linux = sahara_server_path_linux.to_str().unwrap_or("QSaharaServer").to_string();
        config.is_connect = !config.fh_port_conn_str.is_empty();
        config.log_level = log_level;
        return config;
    }
}

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

pub async fn exec_cmd_with_msg(msg: &str, app: &AppHandle, config: &Config, cmd: &[&str]) {
    if config.log_level == LogLevel::Debug {
        let mut cmd_str = format!("{} ", cmd[0]);
        for (_index, s) in cmd.iter().enumerate() {
            if _index != 0 {
                cmd_str = format!("{} {}", cmd_str, s);
            }
        }
        let _ = app.emit("log_event", &format!("{}", cmd_str));
    }
    
    let result = exec_cmd(&app, &cmd, None).await;
    if result.contains("[Error]") {
        let _ = app.emit("log_event", &format!("{}...Error", msg));
    } else {
        let _ = app.emit("log_event", &format!("{}...OK", msg));
    }
}

pub async fn exec_cmd(app: &AppHandle, cmd: &[&str], current_dir: Option<&Path>) -> String {
    if cmd.is_empty() {
        let _ = app.emit("log_event", "[Error]");
        return "[Error] cmd is empty".to_string();
    }
    let work_dir = match current_dir {
        Some(current_dir) => current_dir,
        None => Path::new("."),
    };
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
    let output = exe_cmd.current_dir(&work_dir).output().await;
    
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
