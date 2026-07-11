/// Encode mono f32 PCM samples as a 16-bit WAV byte stream.
pub fn samples_to_wav(samples: &[f32], sample_rate: u32) -> Vec<u8> {
    let mut pcm = Vec::with_capacity(samples.len() * 2);
    for &sample in samples {
        let clamped = sample.clamp(-1.0, 1.0);
        let value = (clamped * i16::MAX as f32) as i16;
        pcm.extend_from_slice(&value.to_le_bytes());
    }

    let byte_rate = sample_rate * 2;
    let block_align = 2u16;
    let bits_per_sample = 16u16;
    let data_size = pcm.len() as u32;
    let riff_size = 36 + data_size;

    let mut wav = Vec::with_capacity(44 + pcm.len());
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&riff_size.to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    wav.extend_from_slice(&block_align.to_le_bytes());
    wav.extend_from_slice(&bits_per_sample.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_size.to_le_bytes());
    wav.extend_from_slice(&pcm);
    wav
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_config::SAMPLE_RATE;

    #[test]
    fn wav_has_header() {
        let wav = samples_to_wav(&[0.0, 0.5, -0.5], SAMPLE_RATE);
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
    }
}
