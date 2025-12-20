use lazy_static::lazy_static;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Condvar, Mutex, mpsc};
use std::thread;

// Export
pub use self::inner::{CommandItem, CommandQueue, QueueManager};

// inner module
mod inner {
    use super::*;

    #[derive(Debug, Clone)]
    pub struct CommandItem {
        pub msg: String,
        pub cmd: String,
        pub args: Vec<String>,
        pub is_finish: bool,
        pub is_success: bool,
        pub exec_result: String,
    }

    #[derive(Debug, Clone)]
    pub struct CommandQueue {
        commands: Vec<CommandItem>,
    }

    impl CommandQueue {
        pub fn new() -> Self {
            CommandQueue { commands: Vec::new() }
        }

        pub fn add_command(&mut self, item: CommandItem) {
            self.commands.push(item);
        }

        pub fn clear(&mut self) {
            self.commands.clear();
        }

        // Process single command + send result
        pub fn process_single_command(&mut self, index: usize, sender: &mpsc::Sender<CommandItem>) {
            if index < self.commands.len() {
                let mut cmd = &mut self.commands[index];
                if !cmd.is_finish {
                    // command execution logic
                    let mut exe_cmd = Command::new(cmd.cmd.clone());
                    #[cfg(target_os = "windows")]
                    {
                      use std::os::windows::process::CommandExt;
                      exe_cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW constant
                    }
                    for (_index, s) in cmd.args.iter().enumerate() {
                            exe_cmd.arg(s);
                    }
                    let output = exe_cmd.current_dir(PathBuf::from(".").as_path()).output();
                    let result = match output {
                        Ok(output) => {
                            if output.status.success() {
                                cmd.is_success = true;
                                String::from_utf8_lossy(&output.stdout).to_string()
                            } else {
                                cmd.is_success = false;
                                let err_msg = String::from_utf8_lossy(&output.stderr).to_string();
                                format!("[Error]: {}", err_msg)
                            }
                        }
                        Err(e) => {
                            cmd.is_success = false;
                            let err_msg = format!("Execution failed: {}", e);
                            format!("[Error]: {}", err_msg)
                        }
                    };

                    cmd.exec_result = format!("Executed: {} {:?} Result: {}", cmd.cmd, cmd.args, result);
                    cmd.is_finish = true;
                    // handle error message
                    if let Err(e) = sender.send(cmd.clone()) {
                        eprintln!("[Worker] Failed to send result: {}", e);
                    }
                }
            }
        }

        pub fn is_empty(&self) -> bool {
            self.commands.is_empty()
        }

        pub fn len(&self) -> usize {
            self.commands.len()
        }
    }

    // QueueManager with graceful shutdown flag
    pub struct QueueManager {
        pub queue: Mutex<CommandQueue>,
        pub condvar: Condvar,
        sender: mpsc::Sender<CommandItem>,
        is_running: Mutex<bool>,
    }

    impl QueueManager {
        pub fn new(sender: mpsc::Sender<CommandItem>) -> Self {
            QueueManager {
                queue: Mutex::new(CommandQueue::new()),
                condvar: Condvar::new(),
                sender,
                is_running: Mutex::new(true),
            }
        }

        // Set exit flag
        pub fn stop(&self) {
            let mut running = self.is_running.lock().unwrap();
            *running = false;
        }

        // Get running status
        pub fn is_running(&self) -> bool {
            *self.is_running.lock().unwrap()
        }

        // Get sender for result transmission
        pub fn sender(&self) -> &mpsc::Sender<CommandItem> {
            &self.sender
        }
    }
}

lazy_static! {
    static ref GLOBAL_SENDER: Mutex<Option<mpsc::Sender<inner::CommandItem>>> = Mutex::new(None);
    static ref GLOBAL_QUEUE_MANAGER: Mutex<Option<Arc<inner::QueueManager>>> = Mutex::new(None);
    static ref GLOBAL_WORKER: Mutex<Option<thread::JoinHandle<()>>> = Mutex::new(None);
}

pub fn init_worker() -> mpsc::Receiver<inner::CommandItem> {
    // 1. Create channel
    let (sender, receiver) = mpsc::channel::<inner::CommandItem>();
    
    // 2. update Sender
    *GLOBAL_SENDER.lock().unwrap() = Some(sender.clone());

    // 3. init QueueManager
    let mut manager_guard = GLOBAL_QUEUE_MANAGER.lock().unwrap();
    if manager_guard.is_none() {
        let manager = Arc::new(inner::QueueManager::new(sender));
        *manager_guard = Some(manager);
    }
    drop(manager_guard); // release

    // 4. create worker thread
    let mut worker_handle = GLOBAL_WORKER.lock().unwrap();
    if worker_handle.is_none() {
        // QueueManager Arc clone
        let manager_clone = GLOBAL_QUEUE_MANAGER.lock().unwrap().as_ref().unwrap().clone();
        let handle = thread::spawn(move || {
            while manager_clone.is_running() {
                let mut queue = manager_clone.queue.lock().unwrap();
                // Wait for commands or exit signal
                while queue.is_empty() && manager_clone.is_running() {
                    queue = manager_clone.condvar.wait(queue).unwrap();
                }

                // Exit if stop signal is received
                if !manager_clone.is_running() {
                    break;
                }

                // Process all commands
                for i in 0..queue.len() {
                    queue.process_single_command(i, manager_clone.sender());
                }
                queue.clear();
            }
            println!("[Worker Module] Worker thread gracefully exited");
        });
        *worker_handle = Some(handle);
    }

    receiver
}

// Public API: Shutdown global worker thread
pub fn shutdown_worker() {
    if let Some(manager) = GLOBAL_QUEUE_MANAGER.lock().unwrap().as_ref() {
        manager.stop();
        manager.condvar.notify_one();
    }

    if let Some(handle) = GLOBAL_WORKER.lock().unwrap().take() {
        if let Err(e) = handle.join() {
            eprintln!("[Worker] Failed to join thread: {:?}", e);
        }
        println!("[Worker Module] Worker thread joined");
    }
}

// Public API: Get global QueueManager instance
pub fn get_global_manager() -> Arc<inner::QueueManager> {
    let manager_guard = GLOBAL_QUEUE_MANAGER.lock().unwrap();
    manager_guard.as_ref().expect("QueueManager not initialized! Call init_worker() first.").clone()
}

pub fn add_command(msg: &str, cmd: &str, args: Vec<&str>) {
    let string_vec: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    
    let manager = get_global_manager();
    let item = CommandItem {
        msg: msg.to_string(),
        cmd: cmd.to_string(),
        args: string_vec,
        is_finish: false,
        is_success: false,
        exec_result: String::new(),
    };
    let mut queue = manager.queue.lock().unwrap();
    queue.add_command(item);
    drop(queue);
    manager.condvar.notify_one();
}

pub fn add_command_without_notify(msg: &str, cmd: &str, args: Vec<&str>) {
    let string_vec: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    
    let manager = get_global_manager();
    let item = CommandItem {
        msg: msg.to_string(),
        cmd: cmd.to_string(),
        args: string_vec,
        is_finish: false,
        is_success: false,
        exec_result: String::new(),
    };
    let mut queue = manager.queue.lock().unwrap();
    queue.add_command(item);
    drop(queue);
}
