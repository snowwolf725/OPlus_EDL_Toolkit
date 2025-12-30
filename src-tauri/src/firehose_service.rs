use std::fs;
use tauri::{AppHandle, Emitter};
use crate::command_util;
use crate::command_util::Config;
use crate::file_util;

pub async fn erase_part(app: &AppHandle, part: &str, xml_content: &str, config: &Config) {
    let file_name = "res/cmd.xml";
    println!("file:{}", &file_name);
    if let Err(e) = fs::write(&file_name, xml_content) {
        let _ = app.emit("log_event", format!("Write file {} failed: {}", file_name, e));
        eprintln!("Write file {} failed: {}", file_name, e);
        return;
    } else {
        println!("success:{}", file_name);
    }
    
    #[cfg(target_os = "windows")] {
        let cmd = ["cmd", "/c", &config.fh_loader_path, &config.fh_port_conn_str, "--memoryname=ufs", "--convertprogram2read",
        "--showpercentagecomplete", "--sendxml=res/cmd.xml", "--noprompt", "--skip_configure", "--mainoutputdir=res"];
        let _ = command_util::exec_cmd_with_msg(&format!("Erase partition {}", part), &app, &config, &cmd).await;
    }
    #[cfg(target_os = "linux")] {
        let cmd = [&config.fh_loader_path_linux, &config.fh_port_conn_str_linux, "--memoryname=ufs", "--convertprogram2read",
        "--showpercentagecomplete", "--sendxml=res/cmd.xml", "--noprompt", "--zlpawarehost=1", "--mainoutputdir=res"];
        let _ = command_util::exec_cmd_with_msg(&format!("Erase partition {}", part), &app, &config, &cmd).await;
    }
}

pub async fn flash_patch_xml(app: &AppHandle, folder: &str, file_name: &str, config: &Config) -> bool {
    let sendxml_str = format!("--sendxml={}", &file_name);
    let dir_str = format!("--search_path={}", &folder);
    
    #[cfg(target_os = "windows")] {
        let cmd = ["cmd", "/c", &config.fh_loader_path, &config.fh_port_conn_str, 
        "--memoryname=ufs", &dir_str, "--showpercentagecomplete", &sendxml_str, 
        "--noprompt", "--skip_configure", "--mainoutputdir=res"];
        match command_util::exec_cmd_with_msg(&format!("Flash patch file: {}...", file_name), &app, &config, &cmd).await {
            Ok(_result) => return true,
            Err(_e) => return false,
        };
    }
    #[cfg(target_os = "linux")] {
        let cmd = [&*config.fh_loader_path_linux, &*config.fh_port_conn_str_linux, "--memoryname=ufs", &dir_str, "--showpercentagecomplete", 
        &sendxml_str, "--noprompt", "--zlpawarehost=1", "--mainoutputdir=res"];
        match command_util::exec_cmd_with_msg(&format!("Flash patch file: {}...", file_name), &app, &config, &cmd).await {
            Ok(_result) => return true,
            Err(_e) => return false,
        };
    }
}

pub async fn flash_part(app: &AppHandle, part: &str, xml_content: &str, dir_path: &str, config: &Config) -> bool {
    let dir_str = format!("--search_path={}", &dir_path);
    let file_name = "res/cmd.xml";
    println!("file:{}", &file_name);
    if let Err(e) = fs::write(&file_name, xml_content) {
        eprintln!("file{}failed:{}", file_name, e);
        let _ = app.emit("log_event", &format!("Write file {} failed: {}", file_name, e));
        return false;
    } else {
        println!("success:{}", file_name);
    }

    #[cfg(target_os = "windows")] {
        let cmd = ["cmd", "/c", &config.fh_loader_path, &config.fh_port_conn_str, 
            "--memoryname=ufs", &dir_str, "--showpercentagecomplete", "--sendxml=res/cmd.xml", 
            "--noprompt", "--skip_configure", "--mainoutputdir=res"];
        match command_util::exec_cmd_with_msg(&format!("Writ partition {}", part), &app, &config, &cmd).await {
            Ok(_result) => return true,
            Err(_e) => return false,
        };
    }
    #[cfg(target_os = "linux")] {
        let cmd = [&config.fh_loader_path_linux, &config.fh_port_conn_str_linux, "--memoryname=ufs", &dir_str, "--showpercentagecomplete",
                "--sendxml=res/cmd.xml","--noprompt", "--zlpawarehost=1", "--mainoutputdir=res"];
        match command_util::exec_cmd_with_msg(&format!("Writ partition {}", part), &app, &config, &cmd).await {
            Ok(_result) => return true,
            Err(_e) => return false,
        };
    }
}

pub async fn read_part(app: &AppHandle, part: &str, xml_content: &str, folder: &str, config: &Config) {
    let file_name = "res/cmd.xml";
    println!("file:{}", &file_name);
    if let Err(e) = fs::write(&file_name, xml_content) {
        let _ = app.emit("log_event", format!("Write file {} failed: {}", file_name, e));
        eprintln!("file{}failed:{}", file_name, e);
        return;
    } else {
        println!("success:{}", file_name);
    }
    
    let dir_str = format!("--mainoutputdir={}", &folder);
    #[cfg(target_os = "windows")] {
        let cmd = ["cmd", "/c", &config.fh_loader_path, &config.fh_port_conn_str, "--memoryname=ufs", "--convertprogram2read",
        "--showpercentagecomplete", "--sendxml=res/cmd.xml", "--noprompt", "--skip_configure", &dir_str];
        let _ = command_util::exec_cmd_with_msg(&format!("Read partition {}...", part), &app, &config, &cmd).await;
    }
    #[cfg(target_os = "linux")] {
        let cmd = [&config.fh_loader_path_linux, &config.fh_port_conn_str_linux, "--memoryname=ufs", "--convertprogram2read",
        "--showpercentagecomplete", "--sendxml=res/cmd.xml", "--noprompt", "--zlpawarehost=1", &dir_str];
        let _ = command_util::exec_cmd_with_msg(&format!("Read partition {}...", part), &app, &config, &cmd).await;
    }
}

pub async fn read_storage_info(app: &AppHandle, config: &Config) -> Result< String, String> {
    let cmd = "<?xml version=\"1.0\" ?><data><getstorageinfo physical_partition_number=\"0\" /></data>";
    file_util::write_to_file("cmd.xml", "res", &cmd);
    #[cfg(target_os = "windows")] {
        let cmd = ["cmd", "/c", &config.fh_loader_path, &config.fh_port_conn_str, "--memoryname=ufs", 
                   "--sendxml=res/cmd.xml", "--noprompt", "--skip_configure", "--mainoutputdir=res"];
        return command_util::exec_cmd_with_msg("Read storage info", &app, &config, &cmd).await;
    }
    #[cfg(target_os = "linux")] {
        let cmd = [&*config.fh_loader_path_linux, &*config.fh_port_conn_str_linux, "--memoryname=ufs", 
                   "--sendxml=res/cmd.xml", "--noprompt", "--zlpawarehost=1", "--mainoutputdir=res"];
        return command_util::exec_cmd_with_msg("Read storage info", &app, &config, &cmd).await;
    }
}

pub async fn reboot_to(app: &AppHandle, msg: &str, xml: &str, config: &Config) {
    file_util::write_to_file("cmd1.xml", "res", &xml);
    let cmd = "<?xml version=\"1.0\" ?><data><power DelayInSeconds=\"0\" value=\"reset\" /></data>";
    file_util::write_to_file("cmd.xml", "res", &cmd);

    #[cfg(target_os = "windows")] {
        // flash misc partition
        let cmd = ["cmd", "/c", &config.fh_loader_path, &config.fh_port_conn_str, 
            "--memoryname=ufs", "--search_path=res", "--showpercentagecomplete", "--sendxml=res/cmd1.xml", 
            "--noprompt", "--skip_configure", "--mainoutputdir=res"];
        let _ = command_util::exec_cmd_with_msg("Writ misc partition", &app, &config, &cmd).await;
        // send reboot command
        let cmd = ["cmd", "/c", &config.fh_loader_path, &config.fh_port_conn_str, "--memoryname=ufs", 
            "--sendxml=res/cmd.xml", "--noprompt", "--skip_configure", "--mainoutputdir=res"];
        let _ = command_util::exec_cmd_with_msg(&msg, &app, &config, &cmd).await;
    }
    #[cfg(target_os = "linux")] {
        let cmd = [&config.fh_loader_path_linux, &config.fh_port_conn_str_linux, "--memoryname=ufs", "--search_path=res", 
            "--showpercentagecomplete", "--sendxml=res/cmd1.xml", "--noprompt", 
            "--zlpawarehost=1", "--mainoutputdir=res"];
        let _ = command_util::exec_cmd_with_msg("Writ misc partition", &app, &config, &cmd).await;
        // send reboot command
        let cmd = [&config.fh_loader_path_linux, &config.fh_port_conn_str_linux, "--memoryname=ufs", 
            "--sendxml=res/cmd.xml", "--noprompt", "--zlpawarehost=1", "--mainoutputdir=res"];
        let _ = command_util::exec_cmd_with_msg(&msg, &app, &config, &cmd).await;
    }
}

pub async fn reboot_to_edl(app: &AppHandle, config: &Config) {
    let cmd = "<?xml version=\"1.0\" ?><data><power DelayInSeconds=\"0\" value=\"reset_to_edl\" /></data>";
    file_util::write_to_file("cmd.xml", "res", &cmd);
    #[cfg(target_os = "windows")] {
        let cmd = ["cmd", "/c", &config.fh_loader_path, &config.fh_port_conn_str, "--memoryname=ufs", 
            "--sendxml=res/cmd.xml", "--noprompt", "--skip_configure", "--mainoutputdir=res"];
        let _ = command_util::exec_cmd_with_msg("Reboot to EDL", &app, &config, &cmd).await;
    }
    #[cfg(target_os = "linux")] {
        let cmd = [&config.fh_loader_path_linux, &config.fh_port_conn_str_linux, "--memoryname=ufs", "--sendxml=res/cmd.xml", "--noprompt", 
            "--zlpawarehost=1", "--mainoutputdir=res"];
        let _ = command_util::exec_cmd_with_msg("Reboot to EDL", &app, &config, &cmd).await;
    }
}

pub async fn reboot_to_system(app: &AppHandle, config: &Config) {
    let cmd = "<?xml version=\"1.0\" ?><data><power DelayInSeconds=\"0\" value=\"reset\" /></data>";
    file_util::write_to_file("cmd.xml", "res", &cmd);
    #[cfg(target_os = "windows")] {
        let cmd = ["cmd", "/c", &config.fh_loader_path, &config.fh_port_conn_str, "--memoryname=ufs", 
            "--sendxml=res/cmd.xml", "--noprompt", "--skip_configure", "--mainoutputdir=res"];
        let _ = command_util::exec_cmd_with_msg("Reboot to EDL", &app, &config, &cmd).await;
    }
    #[cfg(target_os = "linux")] {
        let cmd = [&config.fh_loader_path_linux, &config.fh_port_conn_str_linux, "--memoryname=ufs", "--sendxml=res/cmd.xml", "--noprompt", 
            "--zlpawarehost=1", "--mainoutputdir=res"];
        let _ = command_util::exec_cmd_with_msg("Reboot to EDL", &app, &config, &cmd).await;
    }
}

pub async fn send_loader(app: &AppHandle, loader: &str, digest: &str, sig: &str, config: &Config) {
	#[cfg(target_os = "windows")] {
        let cmd = ["cmd", "/c", &config.sahara_server_path, "-p", &config.sahara_port_conn_str, "-s", &loader];
        let _ = command_util::exec_cmd_with_msg("Send Loader", &app, &config, &cmd).await;
        
        let cmd = ["cmd", "/c", &config.fh_loader_path, &config.fh_port_conn_str, &digest, "--testvipimpact", "--noprompt", "--skip_configure", "--mainoutputdir=res"];
        let _ = command_util::exec_cmd_with_msg("Send Digest", &app, &config, &cmd).await;

        let cmd = ["cmd", "/c", &config.fh_loader_path, &config.fh_port_conn_str, "--sendxml=res/transfercfg.xml", "--noprompt", "--skip_configure", "--mainoutputdir=res"];
        let _ = command_util::exec_cmd_with_msg("Send Transfer Config", &app, &config, &cmd).await;
        
        let cmd = ["cmd", "/c", &config.fh_loader_path, &config.fh_port_conn_str, "--sendxml=res/verify.xml", "--noprompt", "--skip_configure", "--mainoutputdir=res"];
        let _ = command_util::exec_cmd_with_msg("Send Verify", &app, &config, &cmd).await;
        
        let cmd = ["cmd", "/c", &config.fh_loader_path, &config.fh_port_conn_str, &sig, "--testvipimpact", "--noprompt", "--skip_configure", "--mainoutputdir=res"];
        let _ = command_util::exec_cmd_with_msg("Send Sig", &app, &config, &cmd).await;

        let cmd = ["cmd", "/c", &config.fh_loader_path, &config.fh_port_conn_str, "--sendxml=res/sha256init.xml", "--noprompt", "--skip_configure", "--mainoutputdir=res"];
        let _ = command_util::exec_cmd_with_msg("Send SHA256 init", &app, &config, &cmd).await;

        let cmd = ["cmd", "/c", &config.fh_loader_path, &config.fh_port_conn_str, "--memoryname=ufs", "--sendxml=res/cfg.xml", "--search_path=res", "--noprompt", "--mainoutputdir=res"];
        let _ = command_util::exec_cmd_with_msg("Send Storage Config", &app, &config, &cmd).await;
    }
    #[cfg(target_os = "linux")] {
        let cmd = [&config.sahara_server_path_linux, "-p", &config.sahara_port_conn_str_linux, "-s", &loader];
        let _ = command_util::exec_cmd_with_msg("Send Loader", &app, &config, &cmd).await;

        let cmd = [&*config.sahara_server_path_linux, &*config.fh_port_conn_str_linux, &digest, "--testvipimpact", "--noprompt", "--mainoutputdir=res"];
        let _ = command_util::exec_cmd_with_msg("Send Digest", &app, &config, &cmd).await;

        let cmd = [&config.sahara_server_path_linux, &config.fh_port_conn_str_linux, "--sendxml=res/transfercfg.xml", "--noprompt", "--mainoutputdir=res"];
        let _ = command_util::exec_cmd_with_msg("Send Transfer Config", &app, &config, &cmd).await;

        let cmd = [&config.sahara_server_path_linux, &config.fh_port_conn_str_linux, "--sendxml=res/verify.xml", "--noprompt", "--mainoutputdir=res"];
        let _ = command_util::exec_cmd_with_msg("Send Verify", &app, &config, &cmd).await;

        let cmd = [&*config.sahara_server_path_linux, &*config.fh_port_conn_str_linux, &sig, "--testvipimpact", "--noprompt", "--mainoutputdir=res"];
        let _ = command_util::exec_cmd_with_msg("Send Sig", &app, &config, &cmd).await;

        let cmd = [&config.sahara_server_path_linux, &config.fh_port_conn_str_linux, "--sendxml=res/sha256init.xml", "--memoryname=ufs", "--zlpawarehost=1", "--noprompt", "--mainoutputdir=res"];
        let _ = command_util::exec_cmd_with_msg("Send SHA256 init", &app, &config, &cmd).await;
    }
}

pub async fn send_nop(app: &AppHandle, config: &Config) {
    let cmd = "<?xml version=\"1.0\" ?><data><nop verbose=\"0\" value=\"ping\"/></data>".to_string();
    file_util::write_to_file("cmd.xml", "res", &cmd);
    #[cfg(target_os = "windows")] {
        let cmd = ["cmd", "/c", &config.fh_loader_path, &config.fh_port_conn_str, "--memoryname=ufs", 
                   "--sendxml=res/cmd.xml", "--noprompt", "--skip_configure", "--mainoutputdir=res"];
        let _ = command_util::exec_cmd_with_msg("Send Ping Command", &app, &config, &cmd).await;
    }
    #[cfg(target_os = "linux")] {
        let cmd = [&*config.fh_loader_path_linux, &*config.fh_port_conn_str_linux, "--memoryname=ufs", 
                   "--sendxml=res/cmd.xml", "--noprompt", "--zlpawarehost=1", "--mainoutputdir=res"];
        let _ = command_util::exec_cmd_with_msg("Send Ping Command", &app, &config, &cmd).await;
    }
}

pub async fn switch_slot(app: &AppHandle, slot: &str, config: &Config) -> bool {
    let cmd = if slot == "A" {
        "<?xml version=\"1.0\" ?><data><setbootablestoragedrive value=\"1\" /></data>".to_string()
    } else {
        "<?xml version=\"1.0\" ?><data><setbootablestoragedrive value=\"2\" /></data>".to_string()
    };
    file_util::write_to_file("cmd.xml", "res", &cmd);
    #[cfg(target_os = "windows")] {
        let cmd = ["cmd", "/c", &config.fh_loader_path, &config.fh_port_conn_str, 
        "--memoryname=ufs", "--sendxml=res/cmd.xml",
        "--noprompt", "--skip_configure", "--mainoutputdir=res"];
        let _ = command_util::exec_cmd_with_msg("Set active slot", &app, &config, &cmd).await;
    }
    #[cfg(target_os = "linux")] {
        let cmd = [&*config.fh_loader_path_linux, &*config.fh_port_conn_str_linux, "--memoryname=ufs", "--sendxml=res/cmd.xml", 
        "--noprompt", "--zlpawarehost=1", "--mainoutputdir=res"];
        let _ = command_util::exec_cmd_with_msg("Set active slot", &app, &config, &cmd).await;
    }
    return true;
}
