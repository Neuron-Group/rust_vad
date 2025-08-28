use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VoiceData {
    // 根据你的 JSON 结构定义字段
    // 示例中 voice 是空对象，id 是字符串
    pub voice: serde_json::Value,
    pub id: String,
}
