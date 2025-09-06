use rodio::buffer::SamplesBuffer;
use rodio::{OutputStream, Sink};

pub fn play_audio(data: &[f32], sample_rate: u32) {
    // 1. 获取默认输出流（新版 API）
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();

    // 2. 创建 Sink 使用新的 API
    let sink = Sink::try_new(&stream_handle).unwrap();

    // 3. 创建音频源
    let source = SamplesBuffer::new(1, sample_rate, data.to_vec());

    // 4. 播放音频
    sink.append(source);
    sink.sleep_until_end();
}
