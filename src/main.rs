pub mod config;
pub mod convert_pcm;
pub mod data_model;
pub mod fixed_deque;
pub mod model_config;
pub mod model_handler;
pub mod model_interface;
pub mod state;
pub mod vad_error;
pub mod ws_handler;

use axum::{Router, routing::get};

use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let cfgs = config::Config::from_config_file().unwrap();
    let state_cfg = model_config::BaseConfig::<f32, i16>::new(cfgs);
    let app = Router::new()
        .route("/ws/audio", get(ws_handler::websocket_upgrade))
        .with_state(state_cfg);

    let listener = TcpListener::bind("0.0.0.0:8765").await.unwrap();

    axum::serve::serve(listener, app).await.unwrap();
}
