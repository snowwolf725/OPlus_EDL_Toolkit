use ring::digest::{self, Digest, SHA384};
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs::File;
use std::io::{Read, Result};
use std::path::Path;
use x509_parser::parse_x509_certificate;
use x509_parser::prelude::X509Certificate;

fn compute_sha384(data: &[u8]) -> String {
    let digest: Digest = digest::digest(&SHA384, data);
    digest
        .as_ref()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect()
}

/// Condition: 1. basicConstraints CA=true  2. issuer = subject
fn is_root_ca_cert(cert: &X509Certificate) -> bool {
    let has_ca_constraint = cert
        .basic_constraints()
        .map(|bc| bc.unwrap().value.ca)
        .unwrap_or(false);

    let is_self_signed = cert.issuer() == cert.subject();
    has_ca_constraint && is_self_signed
}

pub fn parser_key_hash(file_path: &str) -> Result<HashSet<String>> {
    let mut buffer = Vec::new();
    let mut matches = HashSet::new();
    File::open(file_path)?.read_to_end(&mut buffer)?;

    let pattern_len = 6;
    let mut i = 0;

    while i <= buffer.len().saturating_sub(pattern_len) {
        let slice = &buffer[i..i + pattern_len];
        // Search pattern 30 82 ?? ?? 30 82
        if slice[0] == 0x30 && slice[1] == 0x82 && slice[4] == 0x30 && slice[5] == 0x82 {
            let mut header = [0u8; 6];
            header.copy_from_slice(slice);

            // DER len
            let len_high = buffer[i + 2] as u32;
            let len_low = buffer[i + 3] as u32;
            let total_der_len = 4 + ((len_high << 8) | len_low) as usize;

            // fetch DER data
            let der_data = if i + total_der_len <= buffer.len() {
                buffer[i..i + total_der_len].to_vec()
            } else {
                buffer[i..].to_vec()
            };

            // check CA, Root CA, cal sha384
            let (is_valid, is_root, hash) = match parse_x509_certificate(&der_data) {
                Ok((_, cert)) => {
                    let hash_str = compute_sha384(&der_data);
                    let root_flag = is_root_ca_cert(&cert);
                    (true, root_flag, Some(hash_str))
                }
                Err(_) => (false, false, None),
            };

            if is_valid && is_root {
                if let Some(key_hash) = hash {
                    matches.insert(key_hash);
                }
            }

            // skip der body
            i += total_der_len;
            continue;
        }
        i += 1;
    }

    Ok(matches)
}

pub fn identify_loader<P: AsRef<Path>>(file_path: P) -> String {
    let mut buffer = Vec::new();
    match File::open(file_path) {
        Ok(mut file) => {
            let _ = file.read_to_end(&mut buffer);
        }
        Err(_e) => {}
    }

    // Search binary pattern: "51 43 4F 4D 00"  b"QCOM\x00"
    let search_pattern = b"QCOM\x00";
    let mut result = String::new();
    let mut i = 0;

    // Search pattern in file
    while i + search_pattern.len() <= buffer.len() {
        if &buffer[i..i + search_pattern.len()] == search_pattern {
            let start_index = i + search_pattern.len();
            let mut end_index = start_index;

            // find 0x00
            while end_index < buffer.len() && buffer[end_index] != 0x00 {
                end_index += 1;
            }

            if end_index > start_index {
                let extracted = &buffer[start_index..end_index];
                if extracted.is_empty() == false {
                    for &byte in extracted {
                        if byte.is_ascii_graphic() || byte == b' ' {
                            result.push(byte as char);
                        } else {
                            result.push('.');
                        }
                    }
                }
            }

            break;
        } else {
            i += 1;
        }
    }

    let mut map = HashMap::new();
    map.insert("SM4250", "Snapdragon 460");
    map.insert("SM4350", "Snapdragon 480");
    map.insert("SM6375", "Snapdragon 695");
    map.insert("SM7475", "Snapdragon 7+ Gen 2");
    map.insert("SM7675", "Snapdragon 7+ Gen 3");
    map.insert("SM8350", "Snapdragon 888");
    map.insert("SM8450", "Snapdragon 8 Gen 1");
    map.insert("SM8475", "Snapdragon 8+ Gen 1");
    map.insert("SM8550", "Snapdragon 8 Gen 2");
    map.insert("SM8650", "Snapdragon 8 Gen 3");
    map.insert("SM8750", "Snapdragon 8 Elite");
    if let Some(model) = map.get(result.as_str()) {
        result = format!("{} ({})", result, model);
    } else {
        result = format!("{} (Unknown)", result);
    }
    return result;
}
