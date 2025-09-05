use hound::{SampleFormat, WavReader};
use ndarray::Array1;

use std::fs::File;
use std::io::Write;

use rust_vad::{
    config,
    data_model::{ModelHandeler, Task},
    model_config,
};

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 1. 读取WAV文件
    let mut reader = WavReader::open("test_16k.wav")?;
    let spec = reader.spec();

    // 确认音频格式符合预期
    assert_eq!(spec.sample_rate, 16000, "Sample rate should be 16kHz");
    assert_eq!(spec.channels, 1, "Audio should be mono");

    // 2. 读取音频数据并转换为f32
    let samples: Vec<f32> = match spec.sample_format {
        SampleFormat::Int => {
            let max_value = 2i32.pow((spec.bits_per_sample - 1) as u32) as f32;
            reader
                .samples::<i32>()
                .map(|s| s.unwrap() as f32 / max_value)
                .collect()
        }
        SampleFormat::Float => reader.samples::<f32>().map(|s| s.unwrap()).collect(),
    };

    let config =
        model_config::BaseConfig::<f32, i16>::new(config::Config::from_config_file().unwrap());

    // 4. 初始化模型处理器
    let mut handler = ModelHandeler::new(&config)?;

    // 5. 处理音频并收集结果
    let chunk_size = 512; // VAD模型要求必须是512个样本
    let mut results = Vec::new();

    // 确保每个块都是512个样本，不足的部分用零填充
    for chunk in samples.chunks(chunk_size) {
        let mut padded_chunk = chunk.to_vec();

        // 如果最后一个块不足512个样本，用零填充
        if padded_chunk.len() < chunk_size {
            padded_chunk.resize(chunk_size, 0.0);
        }

        let array = Array1::from_vec(padded_chunk);
        let task = Task::build_strict(&array)?;
        let result: f32 = handler.handle(task)?;
        results.push(result.to_string());
    }

    // 6. 将结果写入文件
    let mut file = File::create("err.log")?;
    writeln!(file, "{}", results.join(","))?;

    println!("Processing completed. Results saved to err.log");
    Ok(())
}
