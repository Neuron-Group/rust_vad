use crate::vad_error::*;
use base64::{Engine as _, engine::general_purpose};

pub fn base64_2_vecu8(input: String) -> Result<Vec<u8>> {
    let decoded_data = general_purpose::STANDARD.decode(input).map_err(|e| {
        dbg!(&e);
        make_parse_err_with_msg(format!("parse base64 error because {e} TAT").to_string())
    })?;

    if !decoded_data.len().is_multiple_of(2) {
        dbg!("cannot convert");
        return Err(make_parse_err_with_msg(
            "decoded data can't mod by 2 TAT".to_string(),
        ));
    }

    Ok(decoded_data)
}
