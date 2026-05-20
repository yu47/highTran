mod crypto;
mod lark_drive;
mod protocol;
mod receiver;
mod sender;
mod speed_test;

use receiver::ReceiverState;
use sender::SenderState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(SenderState::new())
        .manage(ReceiverState::new())
        .invoke_handler(tauri::generate_handler![
            sender::start_send,
            sender::get_send_status,
            receiver::start_receive,
            receiver::get_receive_status,
            speed_test::run_speed_test,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
