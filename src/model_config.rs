use crate::config::Config;
use crate::type_trait::*;

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

    /// 平滑窗口
    pub smoothing_window: usize,

    pub model_path: Box<std::path::Path>,
}

impl<Ft: FloatTrait + From<It>, It: IntTrait> BaseConfig<Ft, It> {
    pub fn new(cfg: Config) -> Self {
        Self {
            orig_sr: It::from(16000).unwrap(),
            target_sr: It::from(16000).unwrap(),
            prob_threshold: <Ft as num_traits::NumCast>::from(0.2).unwrap(),
            db_threshold: <Ft as num_traits::NumCast>::from(30.0).unwrap(),
            required_hits: 5,
            required_misses: 30,
            smoothing_window: 3,
            model_path: cfg.model_path.into_path_buf().into_boxed_path(),
        }
    }
}
