use crate::crypto;
use crate::lark_drive::LarkCredential;
use crate::lark_drive::LarkDriveClient;
use crate::protocol::Packet;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;
use tokio::sync::{mpsc, RwLock};

const POLL_INTERVAL_MS: u64 = 500;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiveStatus {
    pub state: String,
    pub code: String,
    pub progress: f64,
    pub message: String,
    pub filename: String,
    pub file_size: u64,
    pub speed_bps: f64,
    pub eta_secs: f64,
}

pub struct ReceiverState {
    pub status: Arc<RwLock<Option<ReceiveStatus>>>,
}

impl ReceiverState {
    pub fn new() -> Self {
        Self {
            status: Arc::new(RwLock::new(None)),
        }
    }
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

enum DiscoveryMsg {
    ChunkAvailable(u32),
    CompleteFound,
}

#[tauri::command]
pub async fn start_receive(
    app: AppHandle,
    state: tauri::State<'_, ReceiverState>,
    credential: LarkCredential,
    code: String,
    save_dir: String,
) -> Result<String, String> {
    let key = crypto::derive_key(&code);
    let client = LarkDriveClient::new(credential);

    let status = ReceiveStatus {
        state: "connecting".into(),
        code: code.clone(),
        progress: 0.0,
        message: "Connecting...".into(),
        filename: String::new(),
        file_size: 0,
        speed_bps: 0.0,
        eta_secs: -1.0,
    };
    *state.status.write().await = Some(status.clone());
    let _ = app.emit("receive-status", &status);

    let meta_data = match client
        .download_file(&format!("lists/{}/down_r/meta.enc", code))
        .await
    {
        Ok(data) => data,
        Err(e) if e == "NOT_FOUND" => {
            return Err("INVALID_CODE".into());
        }
        Err(e) => return Err(e),
    };
    let decrypted = crypto::decrypt(&key, &meta_data)?;
    let packet = Packet::from_bytes(&decrypted)?;

    let (filename, file_size, md5, chunk_size_val, total_chunks) = match &packet {
        Packet::FileInfo {
            filename,
            size,
            md5,
            chunk_size,
            ..
        } => {
            let cs = if *chunk_size > 0 {
                *chunk_size as usize
            } else {
                8 * 1024 * 1024
            };
            let tc = if *size == 0 {
                1
            } else {
                ((*size as f64) / (cs as f64)).ceil() as u32
            };
            (filename.clone(), *size, md5.clone(), cs, tc)
        }
        _ => return Err("Invalid meta packet".into()),
    };

    let _ = chunk_size_val;

    let _ = client
        .delete_item(&format!("lists/{}/down_r/meta.enc", code))
        .await;

    let save_path = PathBuf::from(&save_dir).join(&filename);

    {
        let mut s = state.status.write().await;
        if let Some(ref mut st) = *s {
            st.filename = filename.clone();
            st.file_size = file_size;
            st.message = format!("{} ({:.1}MB)", filename, file_size as f64 / 1048576.0);
        }
        let _ = app.emit("receive-status", s.as_ref().unwrap());
    }

    let start_signal = Packet::StartSignal {
        filename: filename.clone(),
        size: file_size,
        md5: md5.clone(),
        timestamp: Utc::now().timestamp(),
        code: code.clone(),
    };
    let signal_bytes = start_signal.to_bytes()?;
    let encrypted = crypto::encrypt(&key, &signal_bytes)?;
    client
        .upload_file(&format!("lists/{}/send_r/start.enc", code), encrypted)
        .await?;

    {
        let mut s = state.status.write().await;
        if let Some(ref mut st) = *s {
            st.state = "downloading".into();
            st.message = "Waiting for data...".into();
        }
        let _ = app.emit("receive-status", s.as_ref().unwrap());
    }

    let status_arc = state.status.clone();
    let app_clone = app.clone();

    tokio::spawn(async move {
        if let Err(e) = receiver_pipeline(
            app_clone.clone(),
            status_arc.clone(),
            client,
            code.clone(),
            key,
            save_path,
            total_chunks,
            file_size,
        )
        .await
        {
            eprintln!("Receiver error: {}", e);
            let mut s = status_arc.write().await;
            if let Some(ref mut st) = *s {
                st.state = "error".into();
                st.message = format!("Error: {}", e);
            }
            let _ = app_clone.emit("receive-status", s.as_ref().unwrap());
        }
    });

    Ok(filename)
}

async fn receiver_pipeline(
    app: AppHandle,
    status_arc: Arc<RwLock<Option<ReceiveStatus>>>,
    client: LarkDriveClient,
    code: String,
    key: [u8; 32],
    save_path: PathBuf,
    total_chunks: u32,
    file_size: u64,
) -> Result<(), String> {
    let (tx, mut rx) = mpsc::channel::<DiscoveryMsg>(32);

    let disc_client = client.clone();
    let disc_code = code.clone();
    let discovery_task = tokio::spawn(async move {
        let mut next_to_discover: u32 = 0;
        let mut complete_sent = false;

        loop {
            let items = match disc_client
                .list_children(&format!("lists/{}/down_r", disc_code))
                .await
            {
                Ok(items) => items,
                Err(_) => {
                    tokio::time::sleep(tokio::time::Duration::from_millis(POLL_INTERVAL_MS)).await;
                    continue;
                }
            };

            let mut found_new = true;
            while found_new {
                found_new = false;
                let name = format!("chunk_{}.enc", next_to_discover);
                if items.iter().any(|i| i.name == name) {
                    if tx
                        .send(DiscoveryMsg::ChunkAvailable(next_to_discover))
                        .await
                        .is_err()
                    {
                        return;
                    }
                    next_to_discover += 1;
                    found_new = true;
                }
            }

            if !complete_sent && items.iter().any(|i| i.name == "complete.enc") {
                let _ = tx.send(DiscoveryMsg::CompleteFound).await;
                complete_sent = true;
            }

            if next_to_discover >= total_chunks && complete_sent {
                return;
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(POLL_INTERVAL_MS)).await;
        }
    });

    let file = tokio::fs::File::create(&save_path)
        .await
        .map_err(|e| format!("Create file failed: {}", e))?;
    let mut writer = tokio::io::BufWriter::new(file);

    let mut next_chunk: u32 = 0;
    let mut got_complete = false;
    let transfer_start = Instant::now();
    let mut bytes_received: u64 = 0;

    while let Some(msg) = rx.recv().await {
        match msg {
            DiscoveryMsg::ChunkAvailable(chunk_no) => {
                if chunk_no != next_chunk {
                    continue;
                }

                let chunk_name = format!("chunk_{}.enc", chunk_no);
                let data = client
                    .download_file(&format!("lists/{}/down_r/{}", code, chunk_name))
                    .await
                    .map_err(|e| format!("Download chunk {} failed: {}", chunk_no, e))?;
                let decrypted = crypto::decrypt(&key, &data)?;
                let packet = Packet::from_bytes(&decrypted)?;

                if let Packet::DataChunk { data, .. } = packet {
                    writer
                        .write_all(&data)
                        .await
                        .map_err(|e| format!("Write chunk failed: {}", e))?;

                    bytes_received += data.len() as u64;

                    let del_client = client.clone();
                    let del_path = format!("lists/{}/down_r/{}", code, chunk_name);
                    tokio::spawn(async move {
                        let _ = del_client.delete_item(&del_path).await;
                    });

                    next_chunk += 1;

                    let elapsed = transfer_start.elapsed().as_secs_f64();
                    let speed = if elapsed > 0.0 {
                        bytes_received as f64 / elapsed
                    } else {
                        0.0
                    };
                    let remaining = file_size.saturating_sub(bytes_received) as f64;
                    let eta = if speed > 0.0 { remaining / speed } else { -1.0 };
                    let progress = (next_chunk as f64 / total_chunks as f64) * 100.0;

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
                        let _ = app.emit("receive-status", s.as_ref().unwrap());
                    }
                }
            }
            DiscoveryMsg::CompleteFound => {
                got_complete = true;
                let del_client = client.clone();
                let del_code = code.clone();
                tokio::spawn(async move {
                    let _ = del_client
                        .delete_item(&format!("lists/{}/down_r/complete.enc", del_code))
                        .await;
                });
            }
        }

        if next_chunk >= total_chunks && got_complete {
            break;
        }
    }

    let _ = discovery_task.await;

    writer
        .flush()
        .await
        .map_err(|e| format!("Flush file failed: {}", e))?;
    drop(writer);

    {
        let mut s = status_arc.write().await;
        if let Some(ref mut st) = *s {
            st.state = "completed".into();
            st.progress = 100.0;
            st.eta_secs = 0.0;
            st.message = format!("Done! Saved to: {}", save_path.display());
        }
        let _ = app.emit("receive-status", s.as_ref().unwrap());
    }

    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    let _ = client.delete_item(&format!("lists/{}", code)).await;
    Ok(())
}

#[tauri::command]
pub async fn get_receive_status(
    state: tauri::State<'_, ReceiverState>,
) -> Result<Option<ReceiveStatus>, String> {
    Ok(state.status.read().await.clone())
}
