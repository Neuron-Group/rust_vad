use crate::{model_config, vad_error::*};
use bytes::{Bytes, BytesMut};
use ndarray::Array1;
use num_traits::{FromPrimitive, ToPrimitive, Zero, float, int};
use tokio::sync::mpsc::Sender;

use crate::fixed_deque::FixedLengthQueue;

const MIN_CLIPS: u8 = 3;

pub enum SpeakingStates {
    Idle,
    Active,
    InActive,
}

pub struct StateMachine<
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
    stat_mchne: SpeakingStates,

    cfg: model_config::BaseConfig<Ft, It>,

    prob_buf: Vec<Ft>,
    db_buf: Vec<Ft>,

    bytes_buf: BytesMut,

    miss_count: usize,
    hit_count: usize,

    prob_window: FixedLengthQueue<Ft>,
    db_window: FixedLengthQueue<Ft>,

    pre_buf: Vec<Bytes>,

    output_channel: Sender<ReturnStruct<Ft>>,
}

pub struct ReturnStruct<Ft>
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
    pub probs: Vec<Ft>,
    pub dbs: Vec<Ft>,
    pub sig: Bytes,
}

impl<Ft> ReturnStruct<Ft>
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
    pub fn new(probs: Vec<Ft>, dbs: Vec<Ft>, sig: String) -> Self {
        Self {
            probs,
            dbs,
            sig: Bytes::from(sig),
        }
    }

    pub fn get_sig(&self) -> Result<String> {
        Ok(String::from_utf8(self.sig.to_vec())?)
    }
}

impl<Ft, It> StateMachine<Ft, It>
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
    pub fn new(cfg: &model_config::BaseConfig<Ft, It>, chnl: Sender<ReturnStruct<Ft>>) -> Self {
        Self {
            stat_mchne: SpeakingStates::Idle,

            prob_buf: Vec::new(),
            db_buf: Vec::new(),

            bytes_buf: BytesMut::new(),

            miss_count: 0,
            hit_count: 0,

            prob_window: FixedLengthQueue::new(cfg.smoothing_window),
            db_window: FixedLengthQueue::new(cfg.smoothing_window),

            pre_buf: Vec::new(),

            cfg: cfg.clone(),

            output_channel: chnl,
        }
    }

    fn calculate_db(adio_data: &Array1<Ft>) -> Ft {
        if adio_data.is_empty() {
            return Ft::neg_infinity();
        }

        let sum_squares = adio_data.iter().fold(Ft::zero(), |acc, &x| acc + x * x);
        let len = Ft::from_usize(adio_data.len()).unwrap_or_else(|| Ft::from_f64(1.0).unwrap());
        let mean_squared = sum_squares / len;

        // 计算均方根
        let rms = mean_squared.sqrt();

        // 处理静音
        if rms <= Ft::zero() {
            return Ft::neg_infinity();
        }

        // 添加小数常数避免log(0)
        let adjusted_rms = rms + Ft::from_f64(1e-7).unwrap();

        // 计算分贝值
        Ft::from_f64(20.0).unwrap() * adjusted_rms.log10()
    }

    fn get_smoothed_values(&mut self, prob: Ft, db: Ft) -> (Ft, Ft) {
        self.prob_window.push_front_(prob);
        self.db_window.push_front_(db);

        let smoothed_prob = self.prob_window.mean().unwrap_or(prob);

        let smoothed_db = self
            .db_window
            .mean()
            .unwrap_or_else(|| <Ft as num_traits::NumCast>::from(db).unwrap_or(Ft::zero()));

        // dbg!(<f32 as NumCast>::from(smoothed_db));

        (smoothed_prob, smoothed_db)
    }

    fn update(&mut self, chunk_bytes: Bytes, prob: Ft, db: Ft) {
        self.prob_buf.push(prob);
        self.db_buf.push(db);
        self.bytes_buf.extend(chunk_bytes);
    }

    fn clear(&mut self) {
        self.prob_buf.clear();
        self.db_buf.clear();
        self.bytes_buf.clear();
    }

    async fn update_on_idle(
        &mut self,
        chnk_byts: Bytes,
        smthd_prb: Ft,
        smthd_db: Ft,
    ) -> Result<()> {
        self.pre_buf.push(chnk_byts.clone());

        if smthd_prb >= self.cfg.prob_threshold && smthd_db >= self.cfg.db_threshold {
            self.hit_count += 1;
            // self.miss_count = 0;
            if self.hit_count >= self.cfg.required_hits {
                // 切换状态到 ACTIVE
                self.stat_mchne = SpeakingStates::Active;
                self.update(chnk_byts, smthd_prb, smthd_db);
                self.hit_count = 0;
                self.output_channel
                    .send(ReturnStruct::new(vec![], vec![], String::from("<|PAUSE|>")))
                    .await
                    .map_err(|e| make_parse_err_with_msg(e.to_string()))?;

                println!("pause!");
            }
        } else {
            self.hit_count = 0;
        }

        Ok(())
    }

    async fn update_on_active(
        &mut self,
        chnk_byts: Bytes,
        smthd_prb: Ft,
        smthd_db: Ft,
    ) -> Result<()> {
        self.update(chnk_byts, smthd_prb, smthd_db);

        if smthd_prb >= self.cfg.prob_threshold && smthd_db >= self.cfg.db_threshold {
            self.miss_count = 0;
        } else {
            self.miss_count += 1;

            if self.miss_count >= self.cfg.required_misses {
                self.stat_mchne = SpeakingStates::InActive;
                self.miss_count = 0;
            }
        }

        Ok(())
    }

    async fn update_on_inactive(
        &mut self,
        chnk_byts: Bytes,
        smthd_prb: Ft,
        smthd_db: Ft,
    ) -> Result<()> {
        // dbg!("connected>_<");

        self.update(chnk_byts, smthd_prb, smthd_db);

        if smthd_prb >= self.cfg.prob_threshold && smthd_db >= self.cfg.db_threshold {
            self.hit_count += 1;
            self.miss_count = 0;
            if self.hit_count >= self.cfg.required_hits {
                self.stat_mchne = SpeakingStates::Active;
                self.hit_count = 0;
            }
        } else {
            self.miss_count += 1;
            self.hit_count = 0;

            if self.miss_count >= self.cfg.required_misses {
                self.stat_mchne = SpeakingStates::Idle;
                self.miss_count = 0;
                self.output_channel
                    .send(ReturnStruct::new(
                        vec![],
                        vec![],
                        String::from("<|RESUME|>"),
                    ))
                    .await
                    .map_err(|e| make_parse_err_with_msg(e.to_string()))?;
                println!("resume!");
                if self.prob_buf.len() > MIN_CLIPS.into() {
                    let recent_chunks: Vec<&Bytes> = self
                        .pre_buf
                        .iter()
                        .rev()
                        .take(self.cfg.required_hits)
                        .rev()
                        .collect();

                    let mut out_bytes = BytesMut::new();

                    recent_chunks.into_iter().for_each(|bytes| {
                        out_bytes.extend(bytes);
                    });

                    out_bytes.extend(self.bytes_buf.clone());

                    self.output_channel
                        .send(ReturnStruct {
                            probs: self.prob_buf.clone(),
                            dbs: self.db_buf.clone(),
                            sig: out_bytes.into(),
                        })
                        .await
                        .map_err(|e| make_parse_err_with_msg(e.to_string()))?;

                    self.clear();
                }

                self.pre_buf.clear();
            }
        }

        Ok(())
    }

    pub async fn process(&mut self, prob: Ft, float_chunk_array: Array1<Ft>) -> Result<()> {
        let int_chunk_array = &float_chunk_array
            * match <Ft as num_traits::NumCast>::from(32767.0) {
                Some(value) => value,
                None => {
                    return Err(make_parse_err_with_msg(
                        "需要更大的浮点数类型! -_-".to_string(),
                    ));
                }
            };

        let f32_chunk_array: Vec<f32> = float_chunk_array
            .into_iter()
            .filter_map(|data| data.to_f32())
            .collect();

        let bytes_u8: Vec<u8> = f32_chunk_array
            .into_iter()
            .flat_map(|data| data.to_le_bytes().to_vec())
            .collect();

        let chunk_array = Bytes::from(bytes_u8);

        let db = Self::calculate_db(&int_chunk_array);

        let (smthd_prb, smthd_db) = self.get_smoothed_values(prob, db);

        match self.stat_mchne {
            SpeakingStates::Idle => {
                self.update_on_idle(chunk_array, smthd_prb, smthd_db)
                    .await?
            }
            SpeakingStates::Active => {
                self.update_on_active(chunk_array, smthd_prb, smthd_db)
                    .await?
            }
            SpeakingStates::InActive => {
                self.update_on_inactive(chunk_array, smthd_prb, smthd_db)
                    .await?
            }
        };

        Ok(())
    }
}
