use crate::{
    base64_2_vecu8::*,
    convert_pcm::convert_bytes_to_f32_array,
    data_model::VoiceData,
    handlers::{asr, vad},
    model_config::BaseConfig,
    // play_audio::*,
    state::{self, ReturnStruct},
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
use ndarray::Array1;
use tokio::sync::mpsc::{Receiver, Sender, channel};
use tracing::*;

const INPUT_LEN: usize = 512;

struct Buf {
    buf: Option<[f32; INPUT_LEN]>,
    ptr: usize,
    act: Sender<Array1<f32>>,
}

impl Buf {
    pub fn new(sndr: Sender<Array1<f32>>) -> Self {
        Self {
            buf: None,
            ptr: 0,
            act: sndr,
        }
    }

    pub async fn push_vec(&mut self, mut data: Vec<f32>) -> Result<()> {
        if data.is_empty() {
            return Ok(());
        }

        // 初始化缓冲区
        if self.buf.is_none() {
            self.buf = Some([0.0; INPUT_LEN]);
            self.ptr = 0;
        }

        while !data.is_empty() {
            let avlb = INPUT_LEN - self.ptr;
            let cpy_l = data.len().min(avlb);

            {
                let buf = match self.buf.as_mut() {
                    Some(v) => v,
                    None => return Err(make_parse_err()),
                };
                buf[self.ptr..self.ptr + cpy_l].copy_from_slice(&data[..cpy_l]);
            }

            self.ptr += cpy_l;

            // 更新剩余数据
            data.drain(..cpy_l);

            if self.ptr == INPUT_LEN {
                let buf_data = self.buf.take().unwrap();
                // println!("{:?}", buf_data.clone());
                let input_arr = Array1::from_iter(buf_data.into_iter());
                // println!("{}", input_arr.clone());

                self.act.send(input_arr).await?;

                self.buf = Some([0.0; INPUT_LEN]);
                self.ptr = 0;
            }
        }

        Ok(())
    }
}

struct VadWorker {
    vad_model_handler: vad::ModelHandler,
    // asr_model_handler: asr::ModelHandler<'a>,
    stat: state::StateMachine<f32, i16>,
    rcvr: Receiver<Array1<f32>>, // 接受输入的数组
}

impl VadWorker {
    fn new(
        State(cfg): State<BaseConfig<f32, i16>>,
        rcvr: Receiver<Array1<f32>>,
        sndr: Sender<state::ReturnStruct<f32>>, // 交给state让他提供返回的数据
    ) -> Result<Self> {
        Ok(Self {
            vad_model_handler: vad::ModelHandler::new(&cfg)?,
            // asr_model_handler: asr::ModelHandler::new(&cfg)?,
            stat: state::StateMachine::new(&cfg, sndr),
            rcvr,
        })
    }

    async fn handler(&mut self) -> Result<()> {
        while let Some(value) = self.rcvr.recv().await {
            let tsk = vad::Task::build_strict(&value)?;
            let result = self.vad_model_handler.handle::<f32>(tsk)?;
            // dbg!(Ok::<f32, ModelHandlerErr>(result));
            self.stat.process(result, value).await?;
        }
        Ok(())
    }
}

struct AsrWorker<'a> {
    asr_model_handler: asr::ModelHandler<'a>,
    rcvr: Receiver<state::ReturnStruct<f32>>,
    sndr: Sender<String>,
}

impl<'a> AsrWorker<'a> {
    fn new(
        State(cfg): State<BaseConfig<f32, i16>>,
        rcvr: Receiver<state::ReturnStruct<f32>>,
        sndr: Sender<String>,
    ) -> Result<Self> {
        Ok(Self {
            asr_model_handler: asr::ModelHandler::new(&cfg)?,
            rcvr,
            sndr,
        })
    }

    async fn handler(&mut self) -> Result<()> {
        while let Some(value) = self.rcvr.recv().await {
            if let Some(s) = value.audio_vec {
                let tsk = asr::Task::build(&s);
                let result = self.asr_model_handler.handle(tsk)?;
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
        // let mut history = vec![];
        // let mut cnt = 0;

        while let Some(Ok(msg)) = rcvr.next().await {
            // dbg!(msg.clone());
            match msg {
                Message::Text(text) => {
                    // dbg!(&text);
                    if let Ok(data) = serde_json::from_str::<VoiceData>(&text) {
                        // dbg!(data.clone());
                        if let serde_json::Value::String(s) = data.audio {
                            let pcm_data = base64_2_vecu8(s).unwrap();
                            let float_array = convert_bytes_to_f32_array(&pcm_data, 16);
                            // history = history.into_iter().chain(float_array.clone()).collect();
                            // cnt += 1;

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
                    }
                }
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
                _ => (),
            }
            // if cnt == 100 {
            //     crate::play_audio::play_audio(&history, 16000);
            // }
        }
    });

    // rcvr_return -> sndr
    tokio::spawn(async move {
        while let Some(str) = rcvr_return.recv().await {
            println!("{str}");
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
