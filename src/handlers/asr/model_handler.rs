use crate::vad_error::*;
use ndarray::ArrayView1;
use std::path::Path;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

const INPUT_LEN: usize = 1600000;
pub struct WhisperASR<'a> {
    ctx: WhisperContext,
    params: FullParams<'a, 'a>,
}

impl<'a> WhisperASR<'a> {
    pub fn new(model_path: &Path) -> Result<Self> {
        let mut params = FullParams::new(SamplingStrategy::BeamSearch {
            beam_size: 5,
            patience: -1.0,
        });

        params.set_initial_prompt("你好！");
        params.set_translate(false);
        params.set_language(Option::Some("zh"));
        // params.set_length_penalty(-1.0);
        // params.set_temperature(1.0);
        // params.set_no_context(true);

        Ok(Self {
            ctx: WhisperContext::new_with_params(
                match model_path.to_str() {
                    Some(s) => s,
                    None => {
                        return Err(make_parse_err_with_msg(
                            "mo model path found >_<".to_string(),
                        ));
                    }
                },
                WhisperContextParameters::default(),
            )
            .map_err(|_| {
                make_model_handler_err_with_msg("cannot init asr model TAT!".to_string())
            })?,
            params,
        })
    }

    fn validate_input(&self, x: &ArrayView1<f32>, sr: u32) -> Result<()> {
        if sr != 16000 {
            return Err(make_parse_err_with_msg(
                "input length must fit to 16kHz QwQ".to_string(),
            ));
        }

        if x.len() != INPUT_LEN {
            return Err(make_parse_err_with_msg(
                "input length must fit to 32000 >_<".to_string(),
            ));
        }

        Ok(())
    }

    fn validate_input_on_vec(&self, x: &[f32], sr: u32) -> Result<()> {
        if sr != 16000 {
            return Err(make_parse_err_with_msg(
                "input length must fit to 16kHz QwQ".to_string(),
            ));
        }

        if x.len() != INPUT_LEN {
            return Err(make_parse_err_with_msg(
                "input length must fit to 32000 >_<".to_string(),
            ));
        }

        Ok(())
    }

    pub fn process_chunk(&mut self, x: &ArrayView1<f32>, sr: u32) -> Result<String> {
        // self.validate_input(x, sr)?;

        let x = x.to_vec();
        self.process_chunk_with_vec(x, sr)
    }

    pub fn process_chunk_with_vec(&mut self, x: Vec<f32>, sr: u32) -> Result<String> {
        // self.validate_input_on_vec(&x, sr)?;

        let mut state = self.ctx.create_state().map_err(|_| {
            make_model_handler_err_with_msg("create asr state failed TAT".to_string())
        })?;

        state
            .full(self.params.clone(), &x[..])
            .map_err(|_| "asr process failed TAT".to_string())?;

        let mut output = String::new();

        state.as_iter().for_each(|seg| {
            output += seg.to_str().unwrap_or("");
            output += " "
        });

        Ok(output)
    }
}
