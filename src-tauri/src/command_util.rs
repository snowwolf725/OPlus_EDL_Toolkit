use serialport::SerialPortType;
use serialport::available_ports;
use std::env;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::AppHandle;
use tauri::Emitter;
use tokio::io::{AsyncReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::Mutex;
use bytes::BytesMut;
use encoding_rs::GBK;


#[derive(Debug, Clone)]
pub struct Config {
    pub fh_loader_path: String,

    pub sahara_server_path: String,

    pub fh_loader_path_linux: String,

    pub sahara_server_path_linux: String,

    pub port_path: String,

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
            port_path: String::new(),
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
        config.port_path = port_path.clone();
        config.fh_port_conn_str = port_conn_str;
        config.sahara_port_conn_str = port_str;
        config.fh_port_conn_str_linux = port_conn_str_linux;
        config.sahara_port_conn_str_linux = port_path;
        config.fh_loader_path = fhloader_path
            .to_str()
            .unwrap_or("fh_loader.exe")
            .to_string();
        config.sahara_server_path = sahara_server_path
            .to_str()
            .unwrap_or("QSaharaServer.exe")
            .to_string();
        config.fh_loader_path_linux = fhloader_path_linux
            .to_str()
            .unwrap_or("fh_loader")
            .to_string();
        config.sahara_server_path_linux = sahara_server_path_linux
            .to_str()
            .unwrap_or("QSaharaServer")
            .to_string();
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
            }
            SerialPortType::PciPort | SerialPortType::BluetoothPort | SerialPortType::Unknown => {}
        }
    }
    if port.is_empty() {
        ("Not found".to_string(), "N/A".to_string())
    } else {
        (port, product)
    }
}

pub async fn exec_cmd_with_msg(
    msg: &str,
    app: &AppHandle,
    config: &Config,
    cmd: &[&str],
) -> Result<String, String> {
    if config.log_level == LogLevel::Debug {
        let mut cmd_str = String::new();
        for (_index, s) in cmd.iter().enumerate() {
            cmd_str = format!("{} {}", &cmd_str, s);
        }
        let _ = app.emit("log_event", &format!("{}", &cmd_str));
    }
    let result = exec_cmd_with_progress(&app, &cmd, None).await;
    match result {
        Ok(result) => {
            let _ = app.emit("log_event", &format!("{}...OK", msg));
            Ok(result)
        },
        Err(error) => {
            let _ = app.emit("log_event", &format!("{}...Error", msg));
            Err(error)
        },
    }
}

fn decode_bytes(buffer: &[u8]) -> String {
    #[cfg(target_os = "windows")]
    {
        let (decoded, _, _) = GBK.decode(buffer);
        decoded.to_string()
    }
    #[cfg(target_os = "linux")]
    {
        String::from_utf8_lossy(buffer).to_string()
    }
}

pub async fn exec_cmd_with_progress(app: &AppHandle, cmd: &[&str], current_dir: Option<&Path>) -> Result<String, String> {
    if cmd.is_empty() {
        return Err("[Error] cmd is empty".to_string());
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
    exe_cmd.args(&cmd[1..]).current_dir(work_dir);
    let mut child = exe_cmd.stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn().map_err(|e| format!("Failed to spawn command: {}", e))?;

    let stdout = child.stdout.take().ok_or("[Error] Failed to capture stdout".to_string())?;
    let stderr = child.stderr.take().ok_or("[Error] Failed to capture stderr".to_string())?;
    let stdout_str = Arc::new(Mutex::new(String::new()));
    let stderr_str = Arc::new(Mutex::new(String::new()));
    
    let stdout_clone = Arc::clone(&stdout_str);
    let app_clone = app.clone();
    let stdout_task = tokio::spawn(async move {
        let mut reader = BufReader::new(stdout);
        let mut buffer = BytesMut::with_capacity(1024);
        
        while let Ok(n) = reader.read_buf(&mut buffer).await {
            if n == 0 {
                break;
            }

            let decoded = decode_bytes(&buffer);

            let mut s = stdout_clone.lock().await;
            *s += &decoded;
            *s += "\n";
            
            let _ = app_clone.emit("update_working_percentage", "0");
            
            println!("STDOUT: {}", decoded);
            
            buffer.clear();
        }
    });

    let stderr_clone = Arc::clone(&stderr_str);
    let stderr_task = tokio::spawn(async move {
        let mut reader = BufReader::new(stderr);
        let mut buffer = BytesMut::with_capacity(1024);
        
        while let Ok(n) = reader.read_buf(&mut buffer).await {
            if n == 0 {
                break;
            }
            
            let decoded = decode_bytes(&buffer);

            let mut s = stderr_clone.lock().await;
            *s += &decoded;
            *s += "\n";
            
            println!("STDERR: {}", decoded);
            buffer.clear();
        }
    });

    let status = child
        .wait()
        .await
        .map_err(|e| format!("Failed to wait for command: {}", e))?;

    let _ = stdout_task.await;
    let _ = stderr_task.await;

    let final_stdout = stdout_str.lock().await.clone();
    let final_stderr = stderr_str.lock().await.clone();

    if status.success() {
        Ok(final_stdout)
    } else {
        if final_stderr.is_empty() {
            Err(final_stdout)
        } else {
            Err(final_stderr)
        }
    }
}
