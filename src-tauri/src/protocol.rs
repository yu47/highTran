use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Packet {
    FileInfo {
        filename: String,
        size: u64,
        md5: String,
        timestamp: i64,
        code: String,
        #[serde(default)]
        chunk_size: u64,
    },
    StartSignal {
        filename: String,
        size: u64,
        md5: String,
        timestamp: i64,
        code: String,
    },
    DataChunk {
        timestamp: i64,
        code: String,
        chunk_no: u32,
        total_chunks: u32,
        #[serde(with = "base64_serde")]
        data: Vec<u8>,
    },
    Ack {
        timestamp: i64,
        code: String,
        received_up_to: u32,
    },
    Complete {
        timestamp: i64,
        code: String,
    },
}

mod base64_serde {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(data: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&STANDARD.encode(data))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        STANDARD.decode(&s).map_err(serde::de::Error::custom)
    }
}

impl Packet {
    pub fn to_bytes(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec(self).map_err(|e| format!("Serialize failed: {}", e))
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, String> {
        serde_json::from_slice(data).map_err(|e| format!("Deserialize failed: {}", e))
    }
}
