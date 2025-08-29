use crate::{
    model_config,
    model_handler::SileroVAD,
    vad_error::{ConfigErr, make_parse_err_with_msg, make_type_convert_err},
};
use ndarray::Array1;
use num_traits::{FromPrimitive, ToPrimitive, Zero, float, int};
// use silero_vad_rs::SileroVAD;

pub struct Task(Array1<f32>);

impl Task {
    pub fn build_strict<Ft>(
        data: &Array1<Ft>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>>
    where
        Ft: float::Float
            + float::FloatConst
            + FromPrimitive
            + Zero
            + std::iter::Sum
            + ndarray::ScalarOperand
            + ToPrimitive,
    {
        if 512 != data.len() {
            return Err(make_parse_err_with_msg(
                "data not match to 512 samples >_<".to_string(),
            ));
        }

        Ok(Self(
            data.clone().mapv(|value| value.to_f32().unwrap_or(0.0)),
        ))
    }

    pub fn build<Ft>(data: &Array1<Ft>) -> Self
    where
        Ft: float::Float
            + float::FloatConst
            + FromPrimitive
            + Zero
            + std::iter::Sum
            + ndarray::ScalarOperand
            + ToPrimitive,
    {
        let target_len = 512;

        if data.len() >= target_len {
            // 截取最后512个元素
            let slice = data.slice(ndarray::s![data.len() - target_len..]);
            Self(slice.mapv(|value| value.to_f32().unwrap_or(0.0)))
        } else {
            // 创建新数组并填充
            let mut result = Array1::zeros(target_len);
            let data_f32 = data.mapv(|value| value.to_f32().unwrap_or(0.0));
            result
                .slice_mut(ndarray::s![..data.len()])
                .assign(&data_f32);
            Self(result)
        }
    }
}

pub struct ModelHandeler {
    pub model: SileroVAD,
    sr: u32,
}

impl ModelHandeler {
    pub fn new<Ft, It>(
        cfg: &model_config::BaseConfig<Ft, It>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>>
    where
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
    {
        Ok(Self {
            sr: match cfg.orig_sr.clone().to_u32() {
                Some(v) => v,
                None => return Err(make_type_convert_err()),
            },
            model: SileroVAD::new(&cfg.model_path).map_err(|_| {
                Box::new(ConfigErr::new()) as Box<dyn std::error::Error + Send + Sync>
            })?,
        })
    }

    pub fn handle<Ft>(&mut self, tsk: Task) -> Result<Ft, Box<dyn std::error::Error + Send + Sync>>
    where
        Ft: float::Float
            + float::FloatConst
            + FromPrimitive
            + Zero
            + std::iter::Sum
            + std::ops::Mul<Output = Ft>
            + ndarray::ScalarOperand
            + ToPrimitive,
    {
        let result = self.model.process_chunk(&tsk.0.view(), self.sr)?;

        // let n = tsk.0.len();
        // let m = result.len();
        // let _k = std::cmp::min(n, m);
        let last_k_ele = result; //.slice(ndarray::s![m - k..]);
        // let _mean_value = last_k_ele.mean().unwrap_or(0.0);
        // let first_value = last_k_ele.get(0).unwrap();

        let max_value = last_k_ele.fold(0.0, |acc, &x| if x > acc { x } else { acc });

        // dbg!(1. - max_value);

        Ft::from_f32(1. - max_value).ok_or_else(make_type_convert_err)
    }
}
