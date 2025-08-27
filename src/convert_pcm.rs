pub fn convert_bytes_to_f32_array_(bytes: &[u8]) -> Vec<f32> {
    if !bytes.len().is_multiple_of(4) {
        return Vec::new();
    }

    let mut float_vec = Vec::with_capacity(bytes.len() / 4);

    bytes.chunks_exact(4).for_each(|chunk| {
        let bytes_array: [u8; 4] = chunk.try_into().unwrap();
        let value = f32::from_le_bytes(bytes_array);
        float_vec.push(value);
    });

    float_vec
}

pub fn convert_bytes_to_f32_array(bytes: &[u8], bit_depth: usize) -> Vec<f32> {
    match bit_depth {
        16 => {
            // 将字节转换为i16，然后归一化为f32
            let mut float_vec = Vec::with_capacity(bytes.len() / 2);

            for chunk in bytes.chunks_exact(2) {
                let bytes_array: [u8; 2] = chunk.try_into().unwrap();
                let int_value = i16::from_le_bytes(bytes_array);
                let float_value = int_value as f32 / 32768.0; // 归一化到[-1.0, 1.0]
                float_vec.push(float_value);
            }

            float_vec
        }
        32 => {
            // 直接转换为f32
            convert_bytes_to_f32_array_(bytes)
        }
        _ => {
            // 处理其他位深度或不支持的格式
            Vec::new()
        }
    }
}
