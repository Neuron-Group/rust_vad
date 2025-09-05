use crate::{
    data_model::*,
    handlers::{asr, vad},
    model_config::BaseConfig,
    // play_audio::*,
    state,
    vad_error::*,
};
use axum::{
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use tokio::sync::mpsc::{Receiver, Sender, channel};
use tracing::*;

use reqwest;

const INPUT_LEN: usize = 512;

struct Buf {
    buf: Option<[f32; INPUT_LEN]>,
    ptr: usize,
    act: Sender<VoiceData>,
}

impl Buf {
    pub fn new(sndr: Sender<VoiceData>) -> Self {
        Self {
            buf: None,
            ptr: 0,
            act: sndr,
        }
    }

    pub async fn push_vec(&mut self, mut data: VoiceData) -> Result<()> {
        if data.audio.as_mut().is_none() {
            return Ok(());
        }
        if data.audio.as_mut().unwrap().is_empty() {
            return Ok(());
        }

        // 初始化缓冲区
        if self.buf.is_none() {
            self.buf = Some([0.0; INPUT_LEN]);
            self.ptr = 0;
        }

        let mut audio_data = data.audio.take().unwrap();

        while !audio_data.is_empty() {
            let avlb = INPUT_LEN - self.ptr;
            let cpy_l = audio_data.len().min(avlb);

            {
                let buf = match self.buf.as_mut() {
                    Some(v) => v,
                    None => return Err(make_parse_err()),
                };
                buf[self.ptr..self.ptr + cpy_l].copy_from_slice(&audio_data[..cpy_l]);
            }

            self.ptr += cpy_l;

            // 更新剩余数据
            audio_data.drain(..cpy_l);

            if self.ptr == INPUT_LEN && !audio_data.is_empty() {
                let buf_data = self.buf.take().unwrap();

                let mut data_cpy = data.clone();
                data_cpy.audio = Some(Vec::from(buf_data));
                self.act.send(data_cpy).await?;

                self.buf = Some([0.0; INPUT_LEN]);
                self.ptr = 0;
            }
        }

        if self.ptr == INPUT_LEN {
            let buf_data = self.buf.take().unwrap();
            data.audio = Some(Vec::from(buf_data));
            self.act.send(data).await?;

            self.buf = Some([0.0; INPUT_LEN]);
            self.ptr = 0;
        }

        Ok(())
    }
}

struct VadWorker {
    vad_model_handler: vad::ModelHandler,
    stat: state::StateMachine<f32, i16>,
    rcvr: Receiver<VoiceData>, // 接受输入的数组
}

impl VadWorker {
    fn new(
        State(cfg): State<BaseConfig<f32, i16>>,
        rcvr: Receiver<VoiceData>,
        sndr: Sender<SlicedVoiceData>, // 交给state让他提供返回的数据
    ) -> Result<Self> {
        Ok(Self {
            vad_model_handler: vad::ModelHandler::new(&cfg)?,
            stat: state::StateMachine::new(&cfg, sndr),
            rcvr,
        })
    }

    async fn handler(&mut self) -> Result<()> {
        while let Some(value) = self.rcvr.recv().await {
            let tsk = vad::Task::build_strict(&value)?;
            let result = self.vad_model_handler.handle::<f32>(tsk)?;
            self.stat.process(result, value).await?;
        }
        Ok(())
    }
}

struct AsrWorker<'a> {
    asr_model_handler: asr::ModelHandler<'a>,
    rcvr: Receiver<SlicedVoiceData>,
    sndr: Sender<TextData>,
}

impl<'a> AsrWorker<'a> {
    fn new(
        State(cfg): State<BaseConfig<f32, i16>>,
        rcvr: Receiver<SlicedVoiceData>,
        sndr: Sender<TextData>,
    ) -> Result<Self> {
        Ok(Self {
            asr_model_handler: asr::ModelHandler::new(&cfg)?,
            rcvr,
            sndr,
        })
    }

    async fn handler(&mut self) -> Result<()> {
        while let Some(mut value) = self.rcvr.recv().await {
            let audio_data = value.audio.take();
            if let Some(s) = audio_data {
                let tsk = asr::Task::build(&s);
                let result_text = self.asr_model_handler.handle(tsk)?;
                let mut result = value.init_to_text_data();
                result.text = result_text;
                self.sndr.send(result).await.unwrap();
            }
        }
        Ok(())
    }
}

pub async fn audio_websocket_handler(
    socket: WebSocket,
    stt: State<BaseConfig<f32, i16>>,
) -> Result<()> {
    tracing::info!("Connect constructed >_<");
    dbg!("Connected!");

    let client = reqwest::Client::new();
    let url = stt.output_socket.clone();

    let (mut sndr, mut rcvr) = socket.split();

    let (sndr_input, rcvr_by_vad_worker) = channel(10000);
    let (sndr_by_vad_worker, rcvr_by_asr_worker) = channel(10000);
    let (sndr_by_asr_worker, mut rcvr_return) = channel(10000);

    let mut vad_worker = VadWorker::new(stt.clone(), rcvr_by_vad_worker, sndr_by_vad_worker)?;
    let mut asr_worker = AsrWorker::new(stt, rcvr_by_asr_worker, sndr_by_asr_worker)?;
    let mut buf = Buf::new(sndr_input);

    tokio::spawn(async move { vad_worker.handler().await });
    tokio::spawn(async move { asr_worker.handler().await });

    tokio::spawn(async move {
        while let Some(Ok(msg)) = rcvr.next().await {
            match msg {
                Message::Text(text) => {
                    // dbg!(&text);
                    if let Ok(data) = serde_json::from_str::<NetworkData>(&text) {
                        // println!("!");
                        let voice_data: VoiceData = match data.try_into() {
                            Ok(d) => d,
                            Err(e) => {
                                dbg!("{:?}", e);
                                continue;
                            }
                        };

                        if buf
                            .push_vec(voice_data)
                            .await
                            .map_err(|e| {
                                warn!("{:?}", e);
                                e
                            })
                            .is_err()
                        {
                            continue;
                        };
                    }
                }
                /*
                                Message::Binary(data) => {
                                    // dbg!(data.clone());
                                    let float_array = convert_bytes_to_f32_array(&data, 16);
                                    // println!("{:?}", float_array.clone());

                                    if buf
                                        .push_vec(float_array)
                                        .await
                                        .map_err(|e| {
                                            warn!("{:?}", e);
                                            e
                                        })
                                        .is_err()
                                    {
                                        continue;
                                    };
                                }
                */
                _ => (),
            }
        }
    });

    // rcvr_return -> sndr
    tokio::spawn(async move {
        while let Some(out_form) = rcvr_return.recv().await {
            // println!("{:?}", out_form.clone());
            let resp = client
                .post("http://".to_string() + &url.to_string())
                .form(&out_form)
                .send()
                .await;
            match resp {
                Ok(value) => println!("{:?}", value),
                Err(e) => println!("{:?}", e),
            }

            match sndr.send("success!".to_string().into()).await {
                Ok(_) => continue,
                Err(_) => continue,
            };
        }
    });

    Ok(())
}

pub async fn websocket_upgrade(
    ws: WebSocketUpgrade,
    stt: State<BaseConfig<f32, i16>>,
) -> impl IntoResponse {
    ws.on_upgrade(async move |socket| audio_websocket_handler(socket, stt).await.unwrap())
}
