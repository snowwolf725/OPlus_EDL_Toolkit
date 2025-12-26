pub mod types;
pub mod parsers;
pub mod serial;
pub mod firehose;
pub mod sahara;
use itertools::Itertools;
use types::QdlDevice;
use std::fs;
use std::io;
use crate::qdl::serial::setup_serial_device;
use crate::qdl::types::FirehoseConfiguration;
use crate::qdl::firehose::{firehose_read, firehose_configure};
use crate::qdl::parsers::{firehose_parser_ack_nak, firehose_parser_configure_response};
use crate::qdl::serial::QdlSerialConfig;
use crate::qdl::sahara::SaharaMode;
use crate::qdl::sahara::sahara_run;
use crate::qdl::sahara::SaharaCmdModeCmd;

pub struct SaharaClient {
    chip_sn: String,
    oem_key_hash: String,
    channel: QdlDevice<QdlSerialConfig>,
}

impl SaharaClient {
    pub fn new(dev_path: Option<String>) -> Result<Self, String> {
        // Set up the device
        let rw_channel = match setup_serial_device(dev_path) {
            Ok(config) => Box::new(config),
            Err(e) => return Err(format!("Failed to setup serial device: {}", e)),
        };
        let mut qdl_dev = QdlDevice {
            rw: rw_channel,
            fh_cfg: FirehoseConfiguration::default(),
            reset_on_drop: false,
        };
        // Get some info about the device
        let output = sahara_run(
            &mut qdl_dev,
            SaharaMode::Command,
            Some(SaharaCmdModeCmd::ReadSerialNum),
            &mut [],
            vec![],
            true,
        );
        let sn = match output {
            Ok(result) => result,
            Err(e) => return Err(format!("Failed to get serial from device: {}", e)),
        };
        let sn = u32::from_le_bytes([sn[0], sn[1], sn[2], sn[3]]);
        println!("Chip serial number: 0x{sn:x}");

        let key_hash = sahara_run(
            &mut qdl_dev,
            SaharaMode::Command,
            Some(SaharaCmdModeCmd::ReadOemKeyHash),
            &mut [],
            vec![],
            true,
        ).unwrap();
        println!(
            "OEM Private Key hash: 0x{:02x}",
            key_hash[..key_hash.len() / 3].iter().format("")
        );

        Ok(Self {
            chip_sn: format!("0x{sn:x}"),
            oem_key_hash: format!("0x{:02x}", key_hash[..key_hash.len() / 3].iter().format("")),
            channel: qdl_dev,
        })
    }

    pub fn get_chip_sn(&self) -> String {
        return  self.chip_sn.clone();
    }

    pub fn get_oem_key_hash(&self) -> String {
        return  self.oem_key_hash.clone();
    }

    pub fn send_loader(&mut self, loader_path: &str) {
        // Get the MBN loader binary
        let mbn_loader: Result<Vec<u8>, io::Error> = fs::read(loader_path);

        match mbn_loader {
            Ok(data) => {
                // Send the loader (and any other images)
                let _ = sahara_run(
                    &mut self.channel,
                    SaharaMode::WaitingForImage,
                    None,
                    &mut [data],
                    vec![],
                    true,
                );
                
                // If we're past Sahara, activate the Firehose reset-on-drop listener
                //self.channel.reset_on_drop = true;

                // Get any "welcome" logs
                let _ = firehose_read(&mut self.channel, firehose_parser_ack_nak);

                // Send the host capabilities to the device
                let _ = firehose_configure(&mut self.channel, true);

                // Parse some information from the device
                let _ = firehose_read(&mut self.channel, firehose_parser_configure_response);
            }
            Err(e) => {
                eprintln!("Couldn't open the programmer binary: {}", e);
            }
        };

        
    }
}

