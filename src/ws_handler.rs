use crate::{
    convert_pcm::convert_bytes_to_f32_array,
    data_model::{self, Task},
    model_config::BaseConfig,
    state::{self, ReturnStruct},
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

    pub async fn push(&mut self, data: f32) {
        if self.ptr + 1 >= INPUT_LEN {
            let input_data = self.buf.take().unwrap();
            let input_arr: Array1<f32> = Array1::from_iter(input_data.into_iter());
            self.act.send(input_arr).await.unwrap();
        }

        if self.buf.is_none() {
            self.buf = Some([0.0; INPUT_LEN]);
            self.ptr = 0;
        }

        self.buf.as_mut().unwrap()[self.ptr] = data;
        self.ptr += 1;
    }

    pub async fn push_vec(&mut self, data_vec: Vec<f32>) {
        for data in data_vec {
            self.push(data).await;
        }
    }
}

struct Worker {
    model_handler: data_model::ModelHandeler,
    stat: state::StateMachine<f32, i16>,
    rcvr: Receiver<Array1<f32>>, // 接受输入的数组
}

impl Worker {
    fn new(
        State(cfg): State<BaseConfig<f32, i16>>,
        rcvr: Receiver<Array1<f32>>,
        sndr: Sender<state::ReturnStruct<f32>>, // 交给state让他提供返回的数据
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Self {
            model_handler: data_model::ModelHandeler::new(&cfg)?,
            stat: state::StateMachine::new(&cfg, sndr),
            rcvr,
        })
    }

    async fn handler(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        while let Some(value) = self.rcvr.recv().await {
            let tsk = Task::build_strict(&value)?;
            let result = self.model_handler.handle::<f32>(tsk)?;
            self.stat.process(result, value).await;
        }
        Ok(())
    }
}

pub async fn audio_websocket_handler(
    socket: WebSocket,
    stt: State<BaseConfig<f32, i16>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (mut sndr, mut rcvr) = socket.split();

    let (sndr_to_worker, rcvr_by_worker) = channel(10000);
    let (sndr_return, mut rcvr_return) = channel(10000);

    let mut wkr = Worker::new(stt, rcvr_by_worker, sndr_return)?;
    let mut buf = Buf::new(sndr_to_worker);

    tokio::spawn(async move { wkr.handler().await });

    tokio::spawn(async move {
        while let Some(Ok(msg)) = rcvr.next().await {
            if let Message::Binary(data) = msg {
                let float_array = convert_bytes_to_f32_array(&data, 16);
                buf.push_vec(float_array).await;
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
