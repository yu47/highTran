use crate::lark_drive::LarkCredential;
use crate::lark_drive::LarkDriveClient;
use chrono::Utc;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeedTestResult {
    pub filename: String,
    pub file_size: u64,
    pub upload_secs: f64,
    pub download_secs: f64,
    pub upload_bps: f64,
    pub download_bps: f64,
}

#[tauri::command]
pub async fn run_speed_test(
    credential: LarkCredential,
    test_size_mb: u64,
) -> Result<SpeedTestResult, String> {
    let size_bytes = (test_size_mb as usize) * 1024 * 1024;
    let client = LarkDriveClient::new(credential);
    let filename = format!("hightran-speed-test-{}mb.bin", test_size_mb);
    let file_data = generate_test_data(size_bytes);
    let file_size = file_data.len() as u64;
    let test_id = format!("speed_{}_{}", Utc::now().timestamp(), random_suffix());
    let test_dir = format!("speed_tests/{}", test_id);
    let remote_path = format!("{}/{}", test_dir, filename);

    client.ensure_path(&test_dir).await?;

    let upload_start = Instant::now();
    client.upload_file(&remote_path, file_data.clone()).await?;
    let upload_secs = upload_start.elapsed().as_secs_f64();

    let download_start = Instant::now();
    let downloaded = client.download_file(&remote_path).await?;
    let download_secs = download_start.elapsed().as_secs_f64();

    let _ = client.delete_item(&test_dir).await;

    if downloaded.len() != file_data.len() {
        return Err(format!(
            "Downloaded size mismatch: expected {}, got {}",
            file_data.len(),
            downloaded.len()
        ));
    }

    Ok(SpeedTestResult {
        filename,
        file_size,
        upload_secs,
        download_secs,
        upload_bps: bytes_per_second(file_size, upload_secs),
        download_bps: bytes_per_second(file_size, download_secs),
    })
}

fn bytes_per_second(bytes: u64, secs: f64) -> f64 {
    if secs > 0.0 {
        bytes as f64 / secs
    } else {
        0.0
    }
}

fn random_suffix() -> String {
    let mut rng = rand::thread_rng();
    let chars: Vec<char> = "abcdefghijklmnopqrstuvwxyz0123456789".chars().collect();
    (0..6)
        .map(|_| chars[rng.gen_range(0..chars.len())])
        .collect()
}

fn generate_test_data(size: usize) -> Vec<u8> {
    let mut data = vec![0u8; size];
    for (index, byte) in data.iter_mut().enumerate() {
        *byte = (index % 251) as u8;
    }
    data
}
