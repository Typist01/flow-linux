use crate::SttError;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Read, Write};
use std::path::Path;

const HF_BASE: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main";

/// SHA256 of ggml-base.en.bin on Hugging Face (ggerganov/whisper.cpp).
const BASE_EN_SHA256: &str = "a03779c86df3323075f5e796cb2ce5029f00ec8869eee3fdfb897afe36c6d002";

pub fn model_download_url(model: &str) -> String {
    format!("{HF_BASE}/ggml-{model}.bin")
}

pub fn download_local_model(
    model: &str,
    dest: &Path,
    mut progress: impl FnMut(u64, Option<u64>),
) -> Result<(), SttError> {
    if dest.exists() {
        if model == "base.en" {
            verify_sha256(dest, BASE_EN_SHA256)?;
        }
        return Ok(());
    }

    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| SttError::Cloud(e.to_string()))?;
    }

    let url = model_download_url(model);
    let tmp = dest.with_extension("bin.partial");
    let _ = fs::remove_file(&tmp);

    let mut response = reqwest::blocking::Client::builder()
        .use_rustls_tls()
        .build()
        .map_err(|e| SttError::Cloud(e.to_string()))?
        .get(&url)
        .send()
        .map_err(|e| SttError::Cloud(e.to_string()))?
        .error_for_status()
        .map_err(|e| SttError::Cloud(e.to_string()))?;

    let total = response.content_length();
    let mut file = fs::File::create(&tmp).map_err(|e| SttError::Cloud(e.to_string()))?;
    let mut downloaded = 0u64;
    let mut buf = [0u8; 64 * 1024];

    loop {
        let n = response
            .read(&mut buf)
            .map_err(|e| SttError::Cloud(e.to_string()))?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])
            .map_err(|e| SttError::Cloud(e.to_string()))?;
        downloaded += n as u64;
        progress(downloaded, total);
    }
    file.flush().map_err(|e| SttError::Cloud(e.to_string()))?;
    drop(file);

    if downloaded < 1_000_000 {
        let _ = fs::remove_file(&tmp);
        return Err(SttError::Cloud(format!(
            "downloaded model is too small ({downloaded} bytes)"
        )));
    }

    if model == "base.en" {
        verify_sha256(&tmp, BASE_EN_SHA256)?;
    }

    fs::rename(&tmp, dest).map_err(|e| SttError::Cloud(e.to_string()))?;
    Ok(())
}

fn verify_sha256(path: &Path, expected: &str) -> Result<(), SttError> {
    let mut file = fs::File::open(path).map_err(|e| SttError::Cloud(e.to_string()))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file
            .read(&mut buf)
            .map_err(|e| SttError::Cloud(e.to_string()))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let actual = format!("{:x}", hasher.finalize());
    if actual != expected {
        return Err(SttError::Cloud(format!(
            "SHA256 mismatch for {}: expected {expected}, got {actual}",
            path.display()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::model_download_url;

    #[test]
    fn download_url_uses_whisper_cpp_hf() {
        assert_eq!(
            model_download_url("base.en"),
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin"
        );
    }
}
