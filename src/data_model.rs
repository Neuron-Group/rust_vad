use crate::{base64_2_vecu8::*, convert_pcm::*, vad_error::*};
use ndarray::Array1;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NetworkData {
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VoiceData {
    pub status: String,
    pub device_id: String,
    pub user_id: String,
    pub id: String,
    pub img: String,
    pub audio: Option<Vec<f32>>,
    pub time_stamp: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SlicedVoiceData {
    pub status: String,
    pub device_id: String,
    pub user_id: String,
    pub id: String,
    pub img: String,
    pub audio: Option<Vec<f32>>,
    pub start_time: String,
    pub end_time: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TextData {
    pub status: String,
    pub device_id: String,
    pub user_id: String,
    pub id: String,
    pub img: String,
    pub text: String,
    pub start_time: String,
    pub end_time: String,
}

impl NetworkData {
    pub fn init_to_voice_data(&self) -> Result<VoiceData> {
        Ok(VoiceData {
            status: match self.status.clone() {
                serde_json::Value::String(s) => s,
                _ => return Err(make_parse_err_with_msg("cannot parse status TAT".into())),
            },

            device_id: match self.device_id.clone() {
                serde_json::Value::String(s) => s,
                _ => return Err(make_parse_err_with_msg("cannot parse device_id TAT".into())),
            },

            user_id: match self.user_id.clone() {
                serde_json::Value::String(s) => s,
                _ => return Err(make_parse_err_with_msg("cannot parse user_id TAT".into())),
            },

            id: match self.id.clone() {
                serde_json::Value::String(s) => s,
                _ => return Err(make_parse_err_with_msg("cannot parse id TAT".into())),
            },

            img: match self.image.clone() {
                serde_json::Value::String(s) => s,
                _ => return Err(make_parse_err_with_msg("cannot parse image TAT".into())),
            },

            audio: None,

            time_stamp: match self.end_time.clone() {
                serde_json::Value::String(s) => s,
                _ => {
                    return Err(make_parse_err_with_msg(
                        "cannot parse time stamp TAT".into(),
                    ));
                }
            },
        })
    }
}

impl VoiceData {
    pub fn init_to_sliced_voice_data_with_ref(&self) -> SlicedVoiceData {
        SlicedVoiceData {
            status: self.status.clone(),
            device_id: self.device_id.clone(),
            user_id: self.user_id.clone(),
            id: self.id.clone(),
            img: String::new(),
            audio: None,
            start_time: String::new(),
            end_time: String::new(),
        }
    }
}

impl SlicedVoiceData {
    pub fn init_to_text_data(self) -> TextData {
        TextData {
            status: self.status,
            device_id: self.device_id,
            user_id: self.user_id,
            id: self.id,
            img: self.img,
            text: String::new(),
            start_time: self.start_time,
            end_time: self.end_time,
        }
    }
}

impl TryFrom<NetworkData> for VoiceData {
    type Error = Box<dyn std::error::Error + Send + Sync>;
    fn try_from(value: NetworkData) -> std::result::Result<Self, Self::Error> {
        Ok(VoiceData {
            status: match value.status {
                serde_json::Value::String(s) => s,
                _ => return Err(make_parse_err_with_msg("cannot parse status TAT".into())),
            },

            device_id: match value.device_id {
                serde_json::Value::String(s) => s,
                _ => return Err(make_parse_err_with_msg("cannot parse device_id TAT".into())),
            },

            user_id: match value.user_id {
                serde_json::Value::String(s) => s,
                _ => return Err(make_parse_err_with_msg("cannot parse user_id TAT".into())),
            },

            id: match value.id {
                serde_json::Value::String(s) => s,
                _ => return Err(make_parse_err_with_msg("cannot parse id TAT".into())),
            },

            img: match value.image {
                serde_json::Value::String(s) => s,
                _ => return Err(make_parse_err_with_msg("cannot parse image TAT".into())),
            },

            audio: match value.audio {
                serde_json::Value::String(s) => {
                    Some(convert_bytes_to_f32_array(&base64_2_vecu8(s)?, 16))
                }
                _ => return Err(make_parse_err_with_msg("parse audio failed TAT".into())),
            },

            time_stamp: match value.end_time {
                serde_json::Value::String(s) => s,
                _ => {
                    return Err(make_parse_err_with_msg(
                        "cannot parse time stamp TAT".into(),
                    ));
                }
            },
        })
    }
}
