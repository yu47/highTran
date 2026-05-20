#[path = "../crypto.rs"]
mod crypto;
#[path = "../lark_drive.rs"]
mod lark_drive;
#[path = "../protocol.rs"]
mod protocol;
#[path = "../speed_test.rs"]
mod speed_test;

use chrono::Utc;
use lark_drive::LarkCredential;
use lark_drive::LarkDriveClient;
use md5::{Digest, Md5};
use protocol::Packet;
use rand::Rng;
use std::env;

const DEFAULT_TOKEN: &str = "u-fi7A1UMqB1mGZ.9fZkEnqMV0nkn6k0cjXGy0qxx00Gkd";

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("[FAIL] {}", err);
        std::process::exit(1);
    }
}

async fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    let command = args.get(1).map(String::as_str).unwrap_or("self-test");
    let credential = if let (Some(app_id), Some(app_secret)) = (
        read_arg_value(&args, "--app-id"),
        read_arg_value(&args, "--app-secret"),
    ) {
        LarkCredential::App { app_id, app_secret }
    } else if let Some(token) = read_arg_value(&args, "--token")
        .or_else(|| env::var("LARK_TOKEN").ok())
    {
        LarkCredential::Token { token }
    } else {
        LarkCredential::Token {
            token: DEFAULT_TOKEN.to_string(),
        }
    };

    match command {
        "self-test" => self_test(credential).await,
        "speed-test" => speed_test_cli(credential).await,
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        other => Err(format!("Unknown command: {}", other)),
    }
}

fn print_help() {
    println!("HighTran CLI");
    println!();
    println!("Usage:");
    println!("  cargo run --bin hightran_cli -- self-test --token <LARK_USER_ACCESS_TOKEN>");
    println!("  cargo run --bin hightran_cli -- speed-test --token <LARK_USER_ACCESS_TOKEN>");
    println!("  cargo run --bin hightran_cli -- self-test --app-id <APP_ID> --app-secret <APP_SECRET>");
    println!("  cargo run --bin hightran_cli -- speed-test --app-id <APP_ID> --app-secret <APP_SECRET>");
    println!("  cargo run --bin hightran_cli -- self-test");
    println!();
    println!("If --app-id and --app-secret are provided, app auth is used.");
    println!("Otherwise, --token is used. If --token is omitted, LARK_TOKEN is used.");
    println!("If LARK_TOKEN is also unset, the token embedded for local testing is used.");
}

async fn speed_test_cli(credential: LarkCredential) -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let test_size_mb: u64 = read_arg_value(&args, "--size")
        .and_then(|s| s.parse().ok())
        .unwrap_or(8);
    println!("HighTran Rust CLI speed test ({} MB)", test_size_mb);
    let result = speed_test::run_speed_test(credential, test_size_mb).await?;
    println!(
        "file: {} ({})",
        result.filename,
        format_size(result.file_size)
    );
    println!(
        "upload: {} in {:.2}s",
        format_speed(result.upload_bps),
        result.upload_secs
    );
    println!(
        "download: {} in {:.2}s",
        format_speed(result.download_bps),
        result.download_secs
    );
    println!("[PASS] Speed test completed.");
    Ok(())
}

fn read_arg_value(args: &[String], name: &str) -> Option<String> {
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].clone())
}

async fn self_test(credential: LarkCredential) -> Result<(), String> {
    let code = format!("rt{}", generate_code());
    let key = crypto::derive_key(&code);
    let client = LarkDriveClient::new(credential);
    let filename = "hightran-cli-test.txt".to_string();
    let file_data = b"hello from hightran rust cli test\n".to_vec();
    let file_size = file_data.len() as u64;
    let md5 = compute_md5(&file_data);
    let chunk_size = 8 * 1024 * 1024u64;

    println!("HighTran Rust CLI relay self-test");
    println!("pickup code: {}", code);
    println!();

    println!("[1/9] create relay folders");
    client
        .ensure_path(&format!("lists/{}/send_r", code))
        .await?;
    client
        .ensure_path(&format!("lists/{}/down_r", code))
        .await?;
    println!("[OK] relay folders ready");

    println!("[2/9] sender upload encrypted meta.enc");
    let info_packet = Packet::FileInfo {
        filename: filename.clone(),
        size: file_size,
        md5: md5.clone(),
        timestamp: Utc::now().timestamp(),
        code: code.clone(),
        chunk_size,
    };
    upload_packet(
        &client,
        &key,
        &format!("lists/{}/down_r/meta.enc", code),
        &info_packet,
    )
    .await?;
    println!("[OK] meta.enc uploaded");

    println!("[3/9] receiver download and decrypt meta.enc by pickup code");
    let meta_data = client
        .download_file(&format!("lists/{}/down_r/meta.enc", code))
        .await?;
    let meta_packet = decrypt_packet(&key, &meta_data)?;
    match meta_packet {
        Packet::FileInfo {
            filename: got_name,
            size,
            md5: got_md5,
            code: got_code,
            ..
        } => {
            if got_name != filename || size != file_size || got_md5 != md5 || got_code != code {
                return Err("meta packet content mismatch".into());
            }
        }
        _ => return Err("meta packet had wrong type".into()),
    }
    println!("[OK] receiver resolved and decrypted meta.enc");

    println!("[4/9] receiver upload start.enc");
    let start_packet = Packet::StartSignal {
        filename: filename.clone(),
        size: file_size,
        md5: md5.clone(),
        timestamp: Utc::now().timestamp(),
        code: code.clone(),
    };
    upload_packet(
        &client,
        &key,
        &format!("lists/{}/send_r/start.enc", code),
        &start_packet,
    )
    .await?;
    println!("[OK] start.enc uploaded");

    println!("[5/9] sender list send_r and decrypt start.enc");
    let send_items = client
        .list_children(&format!("lists/{}/send_r", code))
        .await?;
    if !send_items.iter().any(|item| item.name == "start.enc") {
        return Err("sender could not list start.enc".into());
    }
    let start_data = client
        .download_file(&format!("lists/{}/send_r/start.enc", code))
        .await?;
    match decrypt_packet(&key, &start_data)? {
        Packet::StartSignal { code: got_code, .. } if got_code == code => {}
        _ => return Err("start packet mismatch".into()),
    }
    println!("[OK] sender found and decrypted start.enc");

    println!("[6/9] sender upload chunk_0.enc");
    let chunk_packet = Packet::DataChunk {
        timestamp: Utc::now().timestamp(),
        code: code.clone(),
        chunk_no: 0,
        total_chunks: 1,
        data: file_data.clone(),
    };
    upload_packet(
        &client,
        &key,
        &format!("lists/{}/down_r/chunk_0.enc", code),
        &chunk_packet,
    )
    .await?;
    println!("[OK] chunk_0.enc uploaded");

    println!("[7/9] sender upload complete.enc");
    let complete_packet = Packet::Complete {
        timestamp: Utc::now().timestamp(),
        code: code.clone(),
    };
    upload_packet(
        &client,
        &key,
        &format!("lists/{}/down_r/complete.enc", code),
        &complete_packet,
    )
    .await?;
    println!("[OK] complete.enc uploaded");

    println!("[8/9] receiver download/decrypt chunk and complete");
    let chunk_data = client
        .download_file(&format!("lists/{}/down_r/chunk_0.enc", code))
        .await?;
    match decrypt_packet(&key, &chunk_data)? {
        Packet::DataChunk {
            data,
            code: got_code,
            ..
        } if data == file_data && got_code == code => {}
        _ => return Err("chunk packet mismatch".into()),
    }
    let complete_data = client
        .download_file(&format!("lists/{}/down_r/complete.enc", code))
        .await?;
    match decrypt_packet(&key, &complete_data)? {
        Packet::Complete { code: got_code, .. } if got_code == code => {}
        _ => return Err("complete packet mismatch".into()),
    }
    println!("[OK] receiver downloaded and decrypted payload");

    println!("[9/9] cleanup relay folder");
    let _ = client.delete_item(&format!("lists/{}", code)).await;
    println!("[OK] cleanup requested");

    println!();
    println!("[PASS] Rust CLI relay flow works.");
    Ok(())
}

async fn upload_packet(
    client: &LarkDriveClient,
    key: &[u8; 32],
    path: &str,
    packet: &Packet,
) -> Result<(), String> {
    let bytes = packet.to_bytes()?;
    let encrypted = crypto::encrypt(key, &bytes)?;
    client.upload_file(path, encrypted).await
}

fn decrypt_packet(key: &[u8; 32], encrypted: &[u8]) -> Result<Packet, String> {
    let decrypted = crypto::decrypt(key, encrypted)?;
    Packet::from_bytes(&decrypted)
}

fn generate_code() -> String {
    let mut rng = rand::thread_rng();
    let chars: Vec<char> = "abcdefghijklmnopqrstuvwxyz0123456789".chars().collect();
    (0..6)
        .map(|_| chars[rng.gen_range(0..chars.len())])
        .collect()
}

fn compute_md5(data: &[u8]) -> String {
    let mut hasher = Md5::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

fn format_speed(bps: f64) -> String {
    if bps < 1024.0 {
        format!("{:.0} B/s", bps)
    } else if bps < 1048576.0 {
        format!("{:.1} KB/s", bps / 1024.0)
    } else {
        format!("{:.2} MB/s", bps / 1048576.0)
    }
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1048576 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / 1048576.0)
    }
}
