mod command_util;
mod edl_loader_util;
mod file_util;
mod firehose_service;
mod gpt_parser;
mod qdl;
mod super_image_creater;
mod xml_file_util;

use crate::xml_file_util::DataRoot;
use serialport::{SerialPortType, available_ports};
use std::env;
use std::fs;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Error, State};
use tokio::runtime::Runtime;

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

fn flash_patch_xml(
    state: &Arc<std::sync::Mutex<ThreadState>>,
    app: &AppHandle,
    folder: &str,
    files: Vec<String>,
    config: &command_util::Config,
    runtime: &Runtime,
) -> bool {
    let total = files.len();
    let mut count = 0;

    for file in files {
        if state.lock().unwrap().running.load(Ordering::SeqCst) == false {
            let _ = app.emit("log_event", "Operation canceled by user");
            return false;
        }
        count += 1;
        thread::sleep(Duration::from_secs(1));

        let result = runtime.block_on(firehose_service::flash_patch_xml(
            &app, &folder, &file, &config,
        ));
        if result == false {
            let _ = app.emit("log_event", format!("Failed to flash patch: {}", &file));
            let _ = app.emit("stop_edl_flashing", "");
            return false;
        } else {
            println!("Flash patch:{} / {}", (count * 15) / total, total);
            let _ = app.emit("log_event", format!("Flash patch file: {}", &file));
            let _ = app.emit("update_percentage", 80 + (count * 15) / total);
        }
    }
    return true;
}

fn flash_program_xml(
    state: &Arc<std::sync::Mutex<ThreadState>>,
    app: &AppHandle,
    folder: &str,
    programs: Vec<(String, String)>,
    config: &command_util::Config,
    runtime: &Runtime,
) -> bool {
    let total = programs.len();
    let mut count = 0;

    for (label, program) in programs {
        if state.lock().unwrap().running.load(Ordering::SeqCst) == false {
            let _ = app.emit("log_event", "Operation canceled by user");
            return false;
        }
        count += 1;
        thread::sleep(Duration::from_secs(1));

        let result = runtime.block_on(
            firehose_service::flash_part(&app, &label, &program, &folder, &config,
        ));
        match result {
            Ok(_output) => {
                println!("Flash program:{} / {}", (count * 60) / total, total);
                let _ = app.emit("log_event", format!("Flash partition: {}", label));
                let _ = app.emit("update_percentage", 20 + (count * 60) / total);
            },
            Err(_e) => {
                let _ = app.emit("log_event", format!("Failed to flash partition: {}", label));
                let _ = app.emit("stop_edl_flashing", "");
                return false;
            },
        };
    }
    return true;
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
async fn erase_part(app: AppHandle, xml: &str, is_debug: bool) -> Result<(), Error> {
    let config = command_util::Config::setup_env(is_debug);
    if config.is_connect == false {
        let _ = app.emit("log_event", "port not available");
        return Err(tauri::Error::AssetNotFound(
            "port not available".to_string(),
        ));
    }
    let _ = app.emit("update_command_running_status", true);
    // Call the parsing function
    let items = xml_file_util::parser_erase_xml(xml);
    for (part, xml_content) in items {
        if config.is_connect == false {
            return Err(tauri::Error::AssetNotFound(
                "port not available".to_string(),
            ));
        }
        firehose_service::erase_part(&app, &part, &xml_content, &config).await;
    }
    let _ = app.emit("update_command_running_status", false);
    Ok(())
}

#[tauri::command]
async fn identify_loader(app: AppHandle, path: String) {
    let result = edl_loader_util::identify_loader(&path);
    let _ = app.emit("log_event", format!("Select EDL Loader: {}", result));

    match edl_loader_util::parser_key_hash(&path) {
        Ok(results) => {
            let mut count = 0;
            for hash in &results {
                count += 1;
                let _ = app.emit("log_event", format!("Key {} SHA384: {}", count, hash));
            }
        }
        Err(_e) => {}
    }
}

#[tauri::command]
async fn read_device_info(app: AppHandle, is_debug: bool) -> String {
    let config = command_util::Config::setup_env(is_debug);
    if config.is_connect == false {
        return "Device not found".to_string();
    }

    let _ = app.emit("update_command_running_status", true);
    let output = match firehose_service::read_storage_info(&app, &config).await {
        Ok(result) => result,
        Err(_e) => "".to_string(),
    };
    let _ = app.emit("update_command_running_status", false);
    return file_util::analysis_info(&output);
}

#[tauri::command]
async fn read_gpt(app: AppHandle, is_debug: bool) {
    let config = command_util::Config::setup_env(is_debug);
    if config.is_connect == false {
        let _ = app.emit("log_event", "port not available");
        return ();
    }

    let _ = app.emit("update_command_running_status", true);
    let mut root = DataRoot {
        programs: Vec::new(),
        read_tags: Vec::new(),
        erase_tags: Vec::new(),
    };
    for i in 0..6 {
        let read_tag = xml_file_util::create_read_tag_dynamic(
            &format!("gpt_main{}.bin", i),
            i,
            0,
            6,
            "PrimaryGPT",
        );

        let read_xml = xml_file_util::to_xml(&read_tag);
        let xml_content = format!("<?xml version=\"1.0\" ?>\n<data>\n{}\n</data>\n", read_xml);
        let _ = firehose_service::read_part(&app, &format!("LUN {}", i), &xml_content, "img", &config).await;

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
                        let program = xml_file_util::create_program_dynamic(
                            i,
                            partition.first_lba,
                            partition.size_in_sectors(),
                            &partition.name,
                        );
                        root.programs.push(program);
                    }
                }
                Err(_e) => {}
            }
        }
    }

    let read_xml = xml_file_util::to_xml(&root);
    let _ = app.emit("update_partition_table", &read_xml);
    let _ = app.emit("update_command_running_status", false);
}

#[tauri::command]
async fn read_part(app: AppHandle, xml: &str, folder: &str, is_debug: bool) -> Result<(), Error> {
    // Call the parsing function
    let config = command_util::Config::setup_env(is_debug);
    if config.is_connect == false {
        let _ = app.emit("log_event", "Device not found");
        return Err(tauri::Error::AssetNotFound(
            "port not available".to_string(),
        ));
    }
    let _ = app.emit("update_command_running_status", true);
    let items = xml_file_util::parser_read_xml(xml);
    for (part, xml_content) in items {
        if config.is_connect == false {
            return Err(tauri::Error::AssetNotFound(
                "port not available".to_string(),
            ));
        }

        let _ = firehose_service::read_part(&app, &part, &xml_content, &folder, &config).await;
    }
    let _ = app.emit("update_command_running_status", false);
    Ok(())
}

#[tauri::command]
async fn reboot_to_edl(app: AppHandle, is_debug: bool) {
    let config = command_util::Config::setup_env(is_debug);
    if config.is_connect == false {
        let _ = app.emit("log_event", "port not available");
        return ();
    }
    firehose_service::reboot_to_edl(&app, &config).await;
    let _ = app.emit("update_loader_status", false);
}

#[tauri::command]
async fn reboot_to_fastboot(app: AppHandle, xml: &str, is_debug: bool) -> Result<(), Error> {
    let config = command_util::Config::setup_env(is_debug);
    if config.is_connect == false {
        let _ = app.emit("log_event", "port not available");
        return Err(tauri::Error::AssetNotFound(
            "port not available".to_string(),
        ));
    }
    firehose_service::reboot_to(&app, "Reboot to fastbootD", &xml, &config).await;
    let _ = app.emit("update_loader_status", false);
    Ok(())
}

#[tauri::command]
async fn reboot_to_recovery(app: AppHandle, xml: &str, is_debug: bool) -> Result<(), Error> {
    let config = command_util::Config::setup_env(is_debug);
    if config.is_connect == false {
        let _ = app.emit("log_event", "port not available");
        return Err(tauri::Error::AssetNotFound(
            "port not available".to_string(),
        ));
    }
    firehose_service::reboot_to(&app, "Reboot to recovery", &xml, &config).await;
    let _ = app.emit("update_loader_status", false);
    Ok(())
}

#[tauri::command]
async fn reboot_to_system(app: AppHandle, is_debug: bool) {
    let config = command_util::Config::setup_env(is_debug);
    if config.is_connect == false {
        let _ = app.emit("log_event", "port not available");
        return ();
    }
    firehose_service::reboot_to_system(&app, &config).await;
    let _ = app.emit("update_loader_status", false);
}

#[tauri::command]
async fn run_command(app: AppHandle, cmd_type: String, path: String, content: String, is_debug: bool) -> String {
    let mut result = String::new();
    let config = command_util::Config::setup_env(is_debug);
    if config.is_connect == false {
        let _ = app.emit("log_event", "port not available");
        return result;
    }
    let _ = app.emit("update_command_running_status", true);
    let output;
    if cmd_type == "read" {
        output = firehose_service::read_part(&app, "", &content, &path, &config).await;
    } else if cmd_type == "program" {
        output = firehose_service::flash_part(&app, "", &content, &path, &config).await;
    } else {
        output = firehose_service::exec_xml_cmd(&app, &content, &config).await;
    }
    
    let _ = app.emit("update_command_running_status", false);
    result = match output {
        Ok(result) => result,
        Err(_e) => _e,
    };
    return result;
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
async fn send_loader(
    app: AppHandle,
    loader: String,
    digest: String,
    sig: String,
    native: bool,
    is_debug: bool,
) -> String {
    let config = command_util::Config::setup_env(is_debug);
    if config.is_connect == false {
        let _ = app.emit("log_event", "port not available");
        return format!("port not available");
    }
    if native {
        let mut client = match qdl::SaharaClient::new(Some(config.port_path.clone())) {
            Ok(client) => client,
            Err(_e) => return format!("Sahara connect error: {}", _e),
        };
        let _ = app.emit("log_event", &format!("Chip serial number: {}", client.get_chip_sn()));
        let _ = app.emit("log_event", &format!("OEM Key hash: {}", client.get_oem_key_hash()));
        client.send_loader(&loader);
    } else {
        let loader_str = r"13:".to_owned() + &loader;
        let digest_str = r"--signeddigests=".to_owned() + &digest;
        let sig_str = r"--signeddigests=".to_owned() + &sig;
        let _ = firehose_service::send_loader(&app, &loader_str, &digest_str, &sig_str, &config).await;
        let _ = app.emit("update_loader_status", true);
        let _ = app.emit("update_command_running_status", false);
    }
    format!("OK")
}

#[tauri::command]
async fn send_ping(app: AppHandle, is_debug: bool) {
    let config = command_util::Config::setup_env(is_debug);
    if config.is_connect == false {
        let _ = app.emit("log_event", "port not found");
        return;
    }
    firehose_service::send_nop(&app, &config).await;
}

#[tauri::command]
fn start_flashing(
    app: AppHandle,
    path: String,
    is_protect_lun5: bool,
    is_debug: bool,
    thread_state: State<Arc<Mutex<ThreadState>>>,
) -> Result<(), String> {
    // lock thread state
    let mut state_guard = thread_state
        .lock()
        .map_err(|e| format!("lock thread state faild: {}", e))?;

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
                    let config = command_util::Config::setup_env(is_debug);
                    if config.is_connect == false {
                        let _ = app_clone.emit("log_event", "port not available");
                        return;
                    }
                    let rt = Runtime::new().unwrap();
                    let (_file_name, dir_path) =
                        file_util::parse_file_path("", &package.patch_files[0]);
                    if flash_program_xml(
                        &state_clone,
                        &app_clone,
                        &dir_path,
                        package.raw_programs,
                        &config,
                        &rt,
                    ) == false
                    {
                        let _ = app_clone.emit("log_event", "The flashing operation has been stopped");
                        state_clone.lock().unwrap().running.store(false, Ordering::SeqCst);
                        let _ = app_clone.emit("update_command_running_status", false);
                        return;
                    }
                    let _ = app_clone.emit("update_percentage", 80);
                    if flash_patch_xml(
                        &state_clone,
                        &app_clone,
                        &dir_path,
                        package.patch_files,
                        &config,
                        &rt,
                    ) == false
                    {
                        let _ = app_clone.emit("log_event", "The flashing operation has been stopped");
                        state_clone.lock().unwrap().running.store(false, Ordering::SeqCst);
                        let _ = app_clone.emit("update_command_running_status", false);
                        return;
                    }
                    let _ = app_clone.emit("update_percentage", 95);
                    let result = rt.block_on(firehose_service::switch_slot(&app_clone, "A", &config));
                    if result == false {
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
            }
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
fn stop_flashing(
    app: AppHandle,
    thread_state: State<Arc<Mutex<ThreadState>>>,
) -> Result<(), String> {
    // lock thread state
    let state_guard = thread_state.lock().map_err(|e| format!("lock thread state faild: {}", e))?;

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
async fn switch_slot(app: AppHandle, slot: &str, is_debug: bool) -> Result<(), Error> {
    let config = command_util::Config::setup_env(is_debug);
    if config.is_connect == false {
        let _ = app.emit("log_event", "Device not found");
        return Err(tauri::Error::AssetNotFound("Device not found".to_string()));
    }
    let _ = app.emit("update_command_running_status", true);
    firehose_service::switch_slot(&app, &slot, &config).await;
    let _ = app.emit("update_command_running_status", false);
    return Ok(());
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

#[tauri::command]
async fn write_from_xml(app: AppHandle, file_path: &str, is_debug: bool) -> Result<(), Error> {
    let config = command_util::Config::setup_env(is_debug);
    if config.is_connect == false {
        let _ = app.emit("log_event", "port not found");
        return Err(tauri::Error::AssetNotFound(
            "port not available".to_string(),
        ));
    }
    let _ = app.emit("update_command_running_status", true);
    let xml = match file_util::read_text_file(file_path) {
        Ok(content) => content,
        Err(e) => format!("Error reading file: {}", e),
    };
    let (_file_name, dir_path) = file_util::parse_file_path("", file_path);

    let items = xml_file_util::parser_program_xml(&dir_path, &xml);
    for (part, xml_content, _dir_path) in items {
        if config.is_connect == false {
            return Err(tauri::Error::AssetNotFound(
                "port not available".to_string(),
            ));
        }
        let _ = firehose_service::flash_part(&app, &part, &xml_content, &dir_path, &config).await;
    }
    let _ = app.emit("update_command_running_status", false);
    Ok(())
}

#[tauri::command]
async fn write_part(app: AppHandle, xml: &str, is_debug: bool) -> Result<(), Error> {
    let config = command_util::Config::setup_env(is_debug);
    if config.is_connect == false {
        let _ = app.emit("log_event", "port not found");
        return Err(tauri::Error::AssetNotFound(
            "port not available".to_string(),
        ));
    }
    let _ = app.emit("update_command_running_status", true);
    let items = xml_file_util::parser_program_xml("", xml);
    for (part, xml_content, dir_path) in items {
        let _ = firehose_service::flash_part(&app, &part, &xml_content, &dir_path, &config).await;
    }
    let _ = app.emit("update_command_running_status", false);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(Arc::new(Mutex::new(ThreadState::default())))
        .invoke_handler(tauri::generate_handler![
            erase_part,
            identify_loader,
            read_device_info,
            read_gpt,
            read_part,
            reboot_to_edl,
            reboot_to_fastboot,
            reboot_to_recovery,
            reboot_to_system,
            run_command,
            save_to_xml,
            send_ping,
            send_loader,
            start_flashing,
            stop_flashing,
            switch_slot,
            update_port,
            write_from_xml,
            write_part
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
