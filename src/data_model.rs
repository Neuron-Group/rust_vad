use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VoiceData {
    // 根据你的 JSON 结构定义字段
    // 示例中 voice 是空对象，id 是字符串
    pub status: serde_json::Value,
    pub device_id: serde_json::Value,
    pub user_id: serde_json::Value,
    pub id: serde_json::Value,
    pub image: serde_json::Value,
    pub audio: serde_json::Value,
    pub start_time: serde_json::Value,
    pub end_time: serde_json::Value,
    pub sample_rate: serde_json::Value,
    pub bit_depth: serde_json::Value,
    pub format: serde_json::Value,
}
