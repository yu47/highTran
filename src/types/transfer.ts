export interface SendStatus {
  state: "waiting" | "transferring" | "completed" | "error";
  code: string;
  progress: number;
  message: string;
  filename: string;
  file_size: number;
  speed_bps: number;
  eta_secs: number;
}

export interface ReceiveStatus {
  state: "connecting" | "downloading" | "completed" | "error";
  code: string;
  progress: number;
  message: string;
  filename: string;
  file_size: number;
  speed_bps: number;
  eta_secs: number;
}

export interface SpeedTestResult {
  filename: string;
  file_size: number;
  upload_secs: number;
  download_secs: number;
  upload_bps: number;
  download_bps: number;
}

export interface LarkConfig {
  appId: string;
  appSecret: string;
}

export type LarkCredential = { type: "app"; appId: string; appSecret: string };

export type TabType = "send" | "receive" | "speed";
