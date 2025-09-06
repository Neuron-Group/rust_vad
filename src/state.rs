use crate::{
    data_model::*, fixed_deque::FixedLengthQueue, model_config, type_trait::*, vad_error::*,
};

use ndarray::Array1;

use tokio::sync::mpsc::Sender;

const MIN_CLIPS: u8 = 3;

pub enum SpeakingStates {
    Idle,
    Active,
    InActive,
}

pub struct StateMachine<Ft: FloatTrait + From<It>, It: IntTrait> {
    stat_mchne: SpeakingStates,

    cfg: model_config::BaseConfig<Ft, It>,

    prob_buf: Vec<Ft>,
    db_buf: Vec<Ft>,

    vec_buf: Option<Vec<Ft>>,
    time_buf: Option<Vec<String>>,

    miss_count: usize,
    hit_count: usize,

    prob_window: FixedLengthQueue<Ft>,
    db_window: FixedLengthQueue<Ft>,

    pre_buf_vec: Option<FixedLengthQueue<Vec<Ft>>>,
    pre_time_buf: Option<FixedLengthQueue<String>>,

    output_data: Option<SlicedVoiceData<Ft>>,
    output_channel: Sender<SlicedVoiceData<Ft>>,

    state_sender: Sender<VadReturnState>,
}

impl<Ft: FloatTrait + From<It>, It: IntTrait> StateMachine<Ft, It> {
    pub fn new(
        cfg: &model_config::BaseConfig<Ft, It>,
        chnl: Sender<SlicedVoiceData<Ft>>,
        vad_state_detect_chnl: Sender<VadReturnState>,
    ) -> Self {
        Self {
            stat_mchne: SpeakingStates::Idle,

            prob_buf: Vec::new(),
            db_buf: Vec::new(),

            vec_buf: Some(Vec::new()),
            time_buf: Some(Vec::new()),

            miss_count: 0,
            hit_count: 0,

            prob_window: FixedLengthQueue::new(cfg.smoothing_window),
            db_window: FixedLengthQueue::new(cfg.smoothing_window),

            pre_buf_vec: Some(FixedLengthQueue::new(
                cfg.required_hits + cfg.pre_sample_cnt,
            )),
            pre_time_buf: Some(FixedLengthQueue::new(
                cfg.required_hits + cfg.pre_sample_cnt,
            )),

            cfg: cfg.clone(),

            output_data: None,
            output_channel: chnl,

            state_sender: vad_state_detect_chnl,
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

        let smoothed_prb = self.prob_window.mean().unwrap_or(prob);

        let smoothed_db = self
            .db_window
            .mean()
            .unwrap_or_else(|| <Ft as num_traits::NumCast>::from(db).unwrap_or(Ft::zero()));

        // dbg!(<f32 as NumCast>::from(smoothed_db));

        (smoothed_prb, smoothed_db)
    }

    fn update(&mut self, chunk_vec: Vec<Ft>, prob: Ft, db: Ft, time_stamp: String) {
        self.prob_buf.push(prob);
        self.db_buf.push(db);
        self.vec_buf = Some(
            self.vec_buf
                .take()
                .unwrap()
                .into_iter()
                .chain(chunk_vec)
                .collect(),
        );
        self.time_buf.as_mut().unwrap().push(time_stamp);
    }

    fn clear(&mut self) {
        self.prob_buf.clear();
        self.db_buf.clear();
        self.vec_buf = Some(Vec::new());
        self.time_buf = Some(Vec::new());
    }

    async fn update_on_idle(
        &mut self,
        chnk_vec: Vec<Ft>,
        smthd_prb: Ft,
        smthd_db: Ft,
        time_stamp: String,
    ) -> Result<()> {
        if let Some(ls) = self.pre_buf_vec.take() {
            self.pre_buf_vec = Some(ls.push_front(chnk_vec));
        }

        if let Some(ls) = self.pre_time_buf.take() {
            self.pre_time_buf = Some(ls.push_front(time_stamp));
        }

        if smthd_prb >= self.cfg.prob_threshold && smthd_db >= self.cfg.db_threshold {
            self.hit_count += 1;
            // self.miss_count = 0;
            if self.hit_count >= self.cfg.required_hits {
                // 切换状态到 ACTIVE
                self.stat_mchne = SpeakingStates::Active;
                // self.update(chnk_byts, chnk_vec, smthd_prb, smthd_db);
                self.hit_count = 0;
                /*
                                self.output_channel
                                    .send(ReturnStruct::new(vec![], vec![], String::from("<|PAUSE|>")))
                                    .await
                                    .map_err(|e| make_parse_err_with_msg(e.to_string()))?;
                */

                self.state_sender
                    .send(VadReturnState::Pause)
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
        chnk_vec: Vec<Ft>,
        smthd_prb: Ft,
        smthd_db: Ft,
        time_stamp: String,
    ) -> Result<()> {
        self.update(chnk_vec, smthd_prb, smthd_db, time_stamp);

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
        chnk_vec: Vec<Ft>,
        smthd_prb: Ft,
        smthd_db: Ft,
        time_stamp: String,
    ) -> Result<()> {
        self.update(chnk_vec, smthd_prb, smthd_db, time_stamp);

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

                println!("resume!");
                if self.prob_buf.len() > MIN_CLIPS.into() && !self.prob_buf.is_empty() {
                    let out_vec: Vec<Ft> = self
                        .pre_buf_vec
                        .take()
                        .unwrap()
                        .queue
                        .into_iter()
                        .rev()
                        .flatten()
                        .collect();

                    self.pre_buf_vec = Some(FixedLengthQueue::new(
                        self.cfg.required_hits + self.cfg.pre_sample_cnt,
                    ));

                    let out_vec = out_vec
                        .into_iter()
                        .chain(self.vec_buf.take().unwrap())
                        .collect::<Vec<Ft>>();

                    self.vec_buf = Some(Vec::new());

                    // let len_pre_time_buf = self.pre_time_buf.as_ref().unwrap().len();
                    let start_time = self.pre_time_buf.take().unwrap().queue.pop_back().unwrap();
                    let end_time = self.time_buf.take().unwrap().pop().unwrap();

                    self.pre_time_buf = Some(FixedLengthQueue::new(
                        self.cfg.required_hits + self.cfg.pre_sample_cnt,
                    ));
                    self.time_buf = Some(Vec::new());

                    /*
                    let mut data: Vec<f32> = Vec::new();
                    data = out_vec
                        .clone()
                        .into_iter()
                        .map(|v| v.to_f32().unwrap())
                        .collect();
                    play_audio(&data, 16000);
                    */

                    if let Some(mut return_data) = self.output_data.take() {
                        return_data.audio = Some(out_vec);
                        return_data.start_time = start_time;
                        return_data.end_time = end_time;
                        self.output_channel
                            .send(return_data)
                            .await
                            .map_err(|e| make_parse_err_with_msg(e.to_string()))?;

                        self.state_sender
                            .send(VadReturnState::Resume)
                            .await
                            .map_err(|e| make_parse_err_with_msg(e.to_string()))?;

                        self.clear();
                    }
                }

                self.pre_time_buf = Some(FixedLengthQueue::new(
                    self.cfg.required_hits + self.cfg.pre_sample_cnt,
                ));
            }
        }

        Ok(())
    }

    pub async fn process(&mut self, prob: Ft, mut input_data: VoiceData<Ft>) -> Result<()> {
        self.output_data = Some(input_data.init_to_sliced_voice_data_with_ref());

        if input_data.audio.is_none() {
            return Ok(());
        };

        let arried_audio = Array1::from_iter(input_data.audio.as_ref().unwrap().iter().cloned());

        let int_chunk_array = &arried_audio
            * match <Ft as num_traits::NumCast>::from(32767.0) {
                Some(value) => value,
                None => {
                    return Err(make_parse_err_with_msg(
                        "需要更大的浮点数类型! -_-".to_string(),
                    ));
                }
            };

        let ft_chunk_array: Vec<Ft> = input_data.audio.take().unwrap();

        /*
                let ft_chunk_array: Vec<Ft> = f32_chunk_array
                    .into_iter()
                    .map(|v| <Ft as NumCast>::from(v).unwrap_or(Ft::zero()))
                    .collect();
        */

        let db = Self::calculate_db(&int_chunk_array);

        // let (_, smthd_db) = self.get_smoothed_values(prob, db);
        // let smthd_prb = self.bio_filter.update(prob);

        let (smthd_prb, smthd_db) = self.get_smoothed_values(prob, db);

        // dbg!(Result::<f32>::Ok(smthd_prb.to_f32().unwrap()));

        let time_stamp = input_data.time_stamp;

        self.output_data.as_mut().unwrap().img = input_data.img;

        match self.stat_mchne {
            SpeakingStates::Idle => {
                self.update_on_idle(ft_chunk_array, smthd_prb, smthd_db, time_stamp)
                    .await?
            }
            SpeakingStates::Active => {
                self.update_on_active(ft_chunk_array, smthd_prb, smthd_db, time_stamp)
                    .await?
            }
            SpeakingStates::InActive => {
                self.update_on_inactive(ft_chunk_array, smthd_prb, smthd_db, time_stamp)
                    .await?
            }
        };

        Ok(())
    }
}
