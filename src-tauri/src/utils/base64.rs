use base64::Engine;

pub fn buf_to_base64(buf: &[u8]) -> String {
    let encoded = base64::engine::general_purpose::STANDARD.encode(buf);
    format!("data:image/png;base64,{}", encoded)
}

#[cfg(test)]
mod tests;
