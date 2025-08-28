use crate::config::Config;

use num_traits::{FromPrimitive, ToPrimitive, Zero, float, int};

/// 基础模型配置

#[derive(Debug, Clone)]
pub struct BaseConfig<
    Ft: float::Float
        + float::FloatConst
        + FromPrimitive
        + Zero
        + std::iter::Sum
        + From<It>
        + std::ops::Mul<Output = Ft>
        + ndarray::ScalarOperand
        + ToPrimitive,
    It: int::PrimInt + FromPrimitive + Zero + std::iter::Sum,
> {
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

impl<Ft, It> BaseConfig<Ft, It>
where
    It: int::PrimInt + FromPrimitive + Zero + std::iter::Sum,
    Ft: float::Float
        + float::FloatConst
        + FromPrimitive
        + Zero
        + std::iter::Sum
        + From<It>
        + std::ops::Mul<Output = Ft>
        + ndarray::ScalarOperand
        + ToPrimitive,
{
    pub fn new(cfg: Config) -> Self {
        Self {
            orig_sr: It::from(16000).unwrap(),
            target_sr: It::from(16000).unwrap(),
            prob_threshold: <Ft as num_traits::NumCast>::from(0.4).unwrap(),
            db_threshold: <Ft as num_traits::NumCast>::from(20.0).unwrap(),
            required_hits: 4,
            required_misses: 30,
            smoothing_window: 2,
            model_path: cfg.model_path.into_path_buf().into_boxed_path(),
        }
    }
}
