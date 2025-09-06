use crate::{model_config, type_trait::*, vad_error::*};

pub mod model_handler;
use model_handler::*;

// const INPUT_LEN: usize = 1600000;
pub struct Task(Vec<f32>);

impl Task {
    pub fn build<Ft: FloatTrait>(data: &[Ft]) -> Self {
        // if data.len() > INPUT_LEN {
        //     println!("length may not long enough");
        // }
        // let mut result = vec![];
        // let len = data.len().min(INPUT_LEN);
        // for (i, val) in data.iter().rev().take(len).rev().enumerate() {
        //     result[i] = val.to_f32().unwrap_or(0.0);
        // }

        let result = data.iter().map(|val| val.to_f32().unwrap_or(0.0)).collect();
        Self(result)
    }
}

pub struct ModelHandler<'a> {
    pub model: WhisperASR<'a>,
    sr: u32,
}

impl<'a> ModelHandler<'a> {
    pub fn new<Ft: FloatTrait + From<It>, It: IntTrait>(
        cfg: &model_config::BaseConfig<Ft, It>,
    ) -> Result<Self> {
        Ok(Self {
            sr: match cfg.orig_sr.to_u32() {
                Some(v) => v,
                None => return Err(make_type_convert_err()),
            },
            model: WhisperASR::new(&cfg.asr_model_path)?,
        })
    }

    pub fn handle(&mut self, tsk: Task) -> Result<String> {
        self.model.process_chunk_with_vec(tsk.0, self.sr)
    }
}
