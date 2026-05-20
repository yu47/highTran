use crate::crypto;
use crate::lark_drive::LarkCredential;
use crate::lark_drive::LarkDriveClient;
use crate::protocol::Packet;
use chrono::Utc;
use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tauri::{AppHandle, Emitter};
use tokio::sync::RwLock;

const CHUNK_SIZE_SMALL: usize = 8 * 1024 * 1024;
const MAX_PENDING_CHUNKS: usize = 5;
const POLL_INTERVAL_MS: u64 = 500;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendStatus {
    pub state: String,
    pub code: String,
    pub progress: f64,
    pub message: String,
    pub filename: String,
    pub file_size: u64,
    pub speed_bps: f64,
    pub eta_secs: f64,
}

pub struct SenderState {
    pub status: Arc<RwLock<Option<SendStatus>>>,
}

impl SenderState {
    pub fn new() -> Self {
        Self {
            status: Arc::new(RwLock::new(None)),
        }
    }
}

fn generate_code() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let chars: Vec<char> = "abcdefghijklmnopqrstuvwxyz0123456789".chars().collect();
    (0..8)
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

fn format_eta(secs: f64) -> String {
    if secs < 0.0 || secs.is_nan() || secs.is_infinite() {
        return "...".into();
    }
    let s = secs as u64;
    if s < 60 {
        format!("{}s", s)
    } else if s < 3600 {
        format!("{}m{}s", s / 60, s % 60)
    } else {
        format!("{}h{}m", s / 3600, (s % 3600) / 60)
    }
}

#[tauri::command]
pub async fn start_send(
    app: AppHandle,
    state: tauri::State<'_, SenderState>,
    credential: LarkCredential,
    file_path: String,
) -> Result<String, String> {
    let code = generate_code();
    let key = crypto::derive_key(&code);
    let client = LarkDriveClient::new(credential);

    let path = Path::new(&file_path);
    let filename = path
        .file_name()
        .ok_or("Invalid file path")?
        .to_string_lossy()
        .to_string();

    let file_data = tokio::fs::read(&file_path)
        .await
        .map_err(|e| format!("Read file failed: {}", e))?;
    let file_size = file_data.len() as u64;
    let md5 = compute_md5(&file_data);
    let chunk_size = CHUNK_SIZE_SMALL;
    let total_chunks = if file_size == 0 {
        1
    } else {
        ((file_size as f64) / (chunk_size as f64)).ceil() as u32
    };

    let status = SendStatus {
        state: "waiting".into(),
        code: code.clone(),
        progress: 0.0,
        message: "Preparing...".into(),
        filename: filename.clone(),
        file_size,
        speed_bps: 0.0,
        eta_secs: -1.0,
    };
    *state.status.write().await = Some(status.clone());
    let _ = app.emit("send-status", &status);

    client
        .ensure_path(&format!("lists/{}/send_r", code))
        .await?;
    client
        .ensure_path(&format!("lists/{}/down_r", code))
        .await?;

    let info_packet = Packet::FileInfo {
        filename: filename.clone(),
        size: file_size,
        md5: md5.clone(),
        timestamp: Utc::now().timestamp(),
        code: code.clone(),
        chunk_size: chunk_size as u64,
    };
    let info_bytes = info_packet.to_bytes()?;
    let encrypted = crypto::encrypt(&key, &info_bytes)?;
    client
        .upload_file(&format!("lists/{}/down_r/meta.enc", code), encrypted)
        .await?;

    {
        let mut s = state.status.write().await;
        if let Some(ref mut st) = *s {
            st.message = format!("Code: {} - waiting...", code);
        }
        let _ = app.emit("send-status", s.as_ref().unwrap());
    }

    let code_clone = code.clone();
    let status_arc = state.status.clone();
    let app_clone = app.clone();

    tokio::spawn(async move {
        if let Err(e) = sender_loop(
            app_clone.clone(),
            status_arc.clone(),
            client,
            code_clone.clone(),
            key,
            file_data,
            file_size,
            chunk_size,
            total_chunks,
        )
        .await
        {
            eprintln!("Sender error: {}", e);
            let mut s = status_arc.write().await;
            if let Some(ref mut st) = *s {
                st.state = "error".into();
                st.message = format!("Error: {}", e);
            }
            let _ = app_clone.emit("send-status", s.as_ref().unwrap());
        }
    });

    Ok(code)
}

async fn sender_loop(
    app: AppHandle,
    status_arc: Arc<RwLock<Option<SendStatus>>>,
    client: LarkDriveClient,
    code: String,
    key: [u8; 32],
    file_data: Vec<u8>,
    file_size: u64,
    chunk_size: usize,
    total_chunks: u32,
) -> Result<(), String> {
    loop {
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

        let items = client
            .list_children(&format!("lists/{}/send_r", code))
            .await?;

        for item in &items {
            if item.name.starts_with("start") {
                let data = client
                    .download_file(&format!("lists/{}/send_r/{}", code, item.name))
                    .await?;
                let decrypted = crypto::decrypt(&key, &data)?;
                let packet = Packet::from_bytes(&decrypted)?;

                if matches!(packet, Packet::StartSignal { .. }) {
                    let _ = client
                        .delete_item(&format!("lists/{}/send_r/{}", code, item.name))
                        .await;

                    {
                        let mut s = status_arc.write().await;
                        if let Some(ref mut st) = *s {
                            st.state = "transferring".into();
                            st.message = "Transferring...".into();
                        }
                        let _ = app.emit("send-status", s.as_ref().unwrap());
                    }

                    upload_chunks(
                        &app,
                        &status_arc,
                        &client,
                        &code,
                        &key,
                        &file_data,
                        file_size,
                        chunk_size,
                        total_chunks,
                    )
                    .await?;

                    return Ok(());
                }
            }
        }
    }
}

async fn upload_chunks(
    app: &AppHandle,
    status_arc: &Arc<RwLock<Option<SendStatus>>>,
    client: &LarkDriveClient,
    code: &str,
    key: &[u8; 32],
    file_data: &[u8],
    file_size: u64,
    chunk_size: usize,
    total_chunks: u32,
) -> Result<(), String> {
    let transfer_start = Instant::now();
    let mut bytes_sent: u64 = 0;

    let pending_count = Arc::new(AtomicUsize::new(0));
    let done_flag = Arc::new(AtomicUsize::new(0));

    let poll_client = client.clone();
    let poll_code = code.to_string();
    let poll_pending = pending_count.clone();
    let poll_done = done_flag.clone();
    let poller = tokio::spawn(async move {
        loop {
            if poll_done.load(Ordering::Relaxed) == 1 {
                break;
            }
            if let Ok(items) = poll_client
                .list_children(&format!("lists/{}/down_r", poll_code))
                .await
            {
                let count = items
                    .iter()
                    .filter(|i| i.name.starts_with("chunk_"))
                    .count();
                poll_pending.store(count, Ordering::Relaxed);
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(POLL_INTERVAL_MS)).await;
        }
    });

    for chunk_no in 0..total_chunks {
        loop {
            let current = pending_count.load(Ordering::Relaxed);
            if current < MAX_PENDING_CHUNKS {
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        let start = (chunk_no as usize) * chunk_size;
        let end = std::cmp::min(start + chunk_size, file_data.len());
        let chunk_data = &file_data[start..end];

        let packet = Packet::DataChunk {
            timestamp: Utc::now().timestamp(),
            code: code.to_string(),
            chunk_no,
            total_chunks,
            data: chunk_data.to_vec(),
        };
        let packet_bytes = packet.to_bytes()?;
        let encrypted = crypto::encrypt(key, &packet_bytes)?;

        client
            .upload_file(
                &format!("lists/{}/down_r/chunk_{}.enc", code, chunk_no),
                encrypted,
            )
            .await?;

        bytes_sent += chunk_data.len() as u64;

        let elapsed = transfer_start.elapsed().as_secs_f64();
        let speed = if elapsed > 0.0 {
            bytes_sent as f64 / elapsed
        } else {
            0.0
        };
        let remaining_bytes = file_size.saturating_sub(bytes_sent) as f64;
        let eta = if speed > 0.0 {
            remaining_bytes / speed
        } else {
            -1.0
        };
        let progress = ((chunk_no + 1) as f64 / total_chunks as f64) * 100.0;

        {
            let mut s = status_arc.write().await;
            if let Some(ref mut st) = *s {
                st.progress = progress;
                st.speed_bps = speed;
                st.eta_secs = eta;
                st.message = format!(
                    "{:.1}% - {} - ETA {}",
                    progress,
                    format_speed(speed),
                    format_eta(eta)
                );
            }
            let _ = app.emit("send-status", s.as_ref().unwrap());
        }
    }

    done_flag.store(1, Ordering::Relaxed);
    let _ = poller.await;

    let complete = Packet::Complete {
        timestamp: Utc::now().timestamp(),
        code: code.to_string(),
    };
    let complete_bytes = complete.to_bytes()?;
    let encrypted = crypto::encrypt(key, &complete_bytes)?;
    client
        .upload_file(&format!("lists/{}/down_r/complete.enc", code), encrypted)
        .await?;

    {
        let mut s = status_arc.write().await;
        if let Some(ref mut st) = *s {
            st.state = "completed".into();
            st.progress = 100.0;
            st.eta_secs = 0.0;
            st.message = "Transfer complete!".into();
        }
        let _ = app.emit("send-status", s.as_ref().unwrap());
    }

    Ok(())
}

#[tauri::command]
pub async fn get_send_status(
    state: tauri::State<'_, SenderState>,
) -> Result<Option<SendStatus>, String> {
    Ok(state.status.read().await.clone())
}
