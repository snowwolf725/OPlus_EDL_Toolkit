use crate::file_util;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn exec_cmd(cmd: &str, args: Vec<&str>, current_dir:&Path) -> bool {
    if cmd.is_empty() {
        return false;
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
    
    let _ = match output {
        Ok(output) => {
            if output.status.success() {
                println!("{}",String::from_utf8_lossy(&output.stdout).to_string());
                String::from_utf8_lossy(&output.stdout).to_string()
            } else {
                let err_msg = String::from_utf8_lossy(&output.stderr).to_string();
                println!("[Error]: {}", err_msg);
                return false;
            }
        }
        Err(e) => {
            let err_msg = format!("Execution failed: {}", e);
            println!("[Error]: {}", err_msg);
            return false;
        }
    };
    return true;
}

pub fn flash_part(port_path: &str, folder: &str, xml_content: &str) -> bool {
    let current_exe = match env::current_exe() {
        Ok(path) => path,
        Err(_e) => return false,
    };
    let parent_dir: PathBuf = current_exe.parent().unwrap_or(Path::new(".")).to_path_buf();
    let tools_dir = parent_dir.join("tools");
    
    let file_name = format!("res/cmd.xml");
    if let Err(e) = fs::write(&file_name, xml_content) {
        eprintln!("write file {} failed:{}", file_name, e);
        return false;
    } else {
        println!("write file success:{}", file_name);
    }
        
    let dir_str = format!("--search_path={}", &folder);
    #[cfg(target_os = "windows")] {
        let fh_loader_path = tools_dir.join("fh_loader.exe").to_str().unwrap_or("fh_loader.exe").to_string();
        let port_conn_str = r"--port=\\.\".to_owned() + &port_path;
        return exec_cmd("cmd", 
        vec!["/c", &fh_loader_path, &port_conn_str, 
        "--memoryname=ufs", &dir_str, "--showpercentagecomplete", "--sendxml=res/cmd.xml", 
        "--noprompt", "--skip_configure", "--mainoutputdir=res"], Path::new(&folder));
    }
    #[cfg(target_os = "linux")] {
        let fh_loader_path_linux = tools_dir.join("fh_loader").to_str().unwrap_or("fh_loader").to_string();
        let port_conn_str_linux = r"--port=".to_owned() + &port_path;
        return exec_cmd(&fh_loader_path_linux,
        vec![&port_conn_str_linux, "--memoryname=ufs", &dir_str, "--showpercentagecomplete", 
        "--sendxml=res/cmd.xml", "--noprompt", "--zlpawarehost=1", "--mainoutputdir=res"], Path::new(&folder));
    }
}

pub fn flash_patch_xml(port_path: &str, folder: &str, file_name: &str) -> bool {
    let current_exe = match env::current_exe() {
        Ok(path) => path,
        Err(_e) => return false,
    };
    let parent_dir: PathBuf = current_exe.parent().unwrap_or(Path::new(".")).to_path_buf();
    let tools_dir = parent_dir.join("tools");
            
    let sendxml_str = format!("--sendxml={}", &file_name);
    let dir_str = format!("--search_path={}", &folder);
    
    #[cfg(target_os = "windows")] {
        let fh_loader_path = tools_dir.join("fh_loader.exe").to_str().unwrap_or("fh_loader.exe").to_string();
        let port_conn_str = r"--port=\\.\".to_owned() + &port_path;
        return exec_cmd("cmd", 
        vec!["/c", &fh_loader_path, &port_conn_str, 
        "--memoryname=ufs", &dir_str, "--showpercentagecomplete", &sendxml_str, 
        "--noprompt", "--skip_configure", "--mainoutputdir=res"], Path::new(&folder));
    }
    #[cfg(target_os = "linux")] {
        let fh_loader_path_linux = tools_dir.join("fh_loader").to_str().unwrap_or("fh_loader").to_string();
        let port_conn_str_linux = r"--port=".to_owned() + &port_path;
        return exec_cmd(&fh_loader_path_linux,
        vec![&port_conn_str_linux, "--memoryname=ufs", &dir_str, "--showpercentagecomplete", 
        &sendxml_str, "--noprompt", "--zlpawarehost=1", "--mainoutputdir=res"], Path::new(&folder));
    }
}

pub fn switch_slot(port_path: &str, slot: &str) -> bool {
    let current_exe = match env::current_exe() {
        Ok(path) => path,
        Err(_e) => return false,
    };
    let parent_dir: PathBuf = current_exe.parent().unwrap_or(Path::new(".")).to_path_buf();
    let tools_dir = parent_dir.join("tools");
    
    let cmd = if slot == "A" {
        "<?xml version=\"1.0\" ?><data><setbootablestoragedrive value=\"1\" /></data>".to_string()
    } else {
        "<?xml version=\"1.0\" ?><data><setbootablestoragedrive value=\"2\" /></data>".to_string()
    };
    file_util::write_to_file("cmd.xml", "res", &cmd);
    #[cfg(target_os = "windows")] {
        let fh_loader_path = tools_dir.join("fh_loader.exe").to_str().unwrap_or("fh_loader.exe").to_string();
        let port_conn_str = r"--port=\\.\".to_owned() + &port_path;
        exec_cmd("cmd", 
        vec!["/c", &fh_loader_path, &port_conn_str, 
        "--memoryname=ufs", "--sendxml=res/cmd.xml",
        "--noprompt", "--skip_configure", "--mainoutputdir=res"], PathBuf::from(".").as_path());
    }
    #[cfg(target_os = "linux")] {
        let fh_loader_path_linux = tools_dir.join("fh_loader").to_str().unwrap_or("fh_loader").to_string();
        let port_conn_str_linux = r"--port=".to_owned() + &port_path;
        exec_cmd(&fh_loader_path_linux,
        vec![&port_conn_str_linux, "--memoryname=ufs", "--sendxml=res/cmd.xml", 
        "--noprompt", "--zlpawarehost=1", "--mainoutputdir=res"], PathBuf::from(".").as_path());
    }
    return true;
}
