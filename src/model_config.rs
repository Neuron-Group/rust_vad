use crate::config::Config;
use crate::type_trait::*;
use std::net::SocketAddr;

/// 基础模型配置

#[derive(Debug, Clone)]
pub struct BaseConfig<Ft: FloatTrait + From<It>, It: IntTrait> {
    pub orig_sr: It,
    pub target_sr: It,
    pub prob_threshold: Ft,
    pub db_threshold: Ft,

    /// start_delay
    pub required_hits: usize,

    /// end_delay
    pub required_misses: usize,

    pub pre_sample_cnt: usize,

    /// 平滑窗口
    pub smoothing_window: usize,

    pub model_path: Box<std::path::Path>,

    pub asr_model_path: Box<std::path::Path>,
    // 网络接口
    pub input_socket: SocketAddr,
    pub output_socket: SocketAddr,
}

impl<Ft: FloatTrait + From<It>, It: IntTrait> BaseConfig<Ft, It> {
    pub fn new(cfg: Config) -> Self {
        Self {
            orig_sr: It::from(16000).unwrap(),
            target_sr: It::from(16000).unwrap(),
            prob_threshold: <Ft as num_traits::NumCast>::from(0.6).unwrap(),
            db_threshold: <Ft as num_traits::NumCast>::from(30.0).unwrap(),
            required_hits: 3,
            required_misses: 20,
            pre_sample_cnt: 15,
            smoothing_window: 2,
            model_path: cfg.model_path.into_path_buf().into_boxed_path(),
            asr_model_path: cfg.asr_model_path.into_path_buf().into_boxed_path(),

            input_socket: "0.0.0.0:8765".parse().unwrap(),
            output_socket: "0.0.0.0:8766".parse().unwrap(),
        }
    }
}
