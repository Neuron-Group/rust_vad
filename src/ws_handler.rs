use crate::{
    convert_pcm::convert_bytes_to_f32_array,
    data_model::VoiceData,
    model_config::BaseConfig,
    model_interface::{ModelHandeler, Task},
    state::{self, ReturnStruct},
    vad_error::*,
};
use axum::{
    extract::{
        State,
        ws::{Message, Utf8Bytes, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use ndarray::Array1;
use tokio::sync::mpsc::{Receiver, Sender, channel};

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

struct Worker {
    model_handler: ModelHandeler,
    stat: state::StateMachine<f32, i16>,
    rcvr: Receiver<Array1<f32>>, // 接受输入的数组
}

impl Worker {
    fn new(
        State(cfg): State<BaseConfig<f32, i16>>,
        rcvr: Receiver<Array1<f32>>,
        sndr: Sender<state::ReturnStruct<f32>>, // 交给state让他提供返回的数据
    ) -> Result<Self> {
        Ok(Self {
            model_handler: ModelHandeler::new(&cfg)?,
            stat: state::StateMachine::new(&cfg, sndr),
            rcvr,
        })
    }

    async fn handler(&mut self) -> Result<()> {
        while let Some(value) = self.rcvr.recv().await {
            let tsk = Task::build_strict(&value)?;
            let result = self.model_handler.handle::<f32>(tsk)?;
            dbg!(Ok::<f32, ModelHandlerErr>(result));
            self.stat.process(result, value).await?;
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

    let (sndr_to_worker, rcvr_by_worker) = channel(10000);
    let (sndr_return, mut rcvr_return) = channel(10000);

    let mut wkr = Worker::new(stt, rcvr_by_worker, sndr_return)?;
    let mut buf = Buf::new(sndr_to_worker);

    tokio::spawn(async move { wkr.handler().await });

    tokio::spawn(async move {
        while let Some(Ok(msg)) = rcvr.next().await {
            // dbg!(msg.clone());
            match msg {
                Message::Text(text) => match serde_json::from_str::<VoiceData>(&text) {
                    Ok(data) => {
                        // dbg!(data.clone());
                        if let serde_json::Value::String(str) = data.voice {
                            // dbg!(str);
                        }
                    }
                    Err(_) => (),
                },
                Message::Binary(data) => {
                    // dbg!(data.clone());
                    let float_array = convert_bytes_to_f32_array(&data, 16);
                    // println!("{:?}", float_array.clone());
                    buf.push_vec(float_array).await;
                }
                _ => (),
            }
        }
    });

    // rcvr_return -> sndr
    tokio::spawn(async move {
        while let Some(ReturnStruct {
            probs: _,
            dbs: _,
            sig: sig_data,
        }) = rcvr_return.recv().await
        {
            let message = Message::Binary(sig_data);

            // dbg!(message.clone());

            if let Err(e) = sndr.send(message).await {
                tracing::info!("socket closed, task panic. {}", e);

                break;
            }
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
