use std::collections::{BTreeMap, HashMap};
use std::fs::{self, File};
use std::io::{Seek, SeekFrom, Write};

use anyhow::{anyhow, Context};
use rand::random;

/// PCM output container.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PcmFormat {
    Wav,
    Flac,
}

/// Incremental PCM encoder used by the command-line pipeline.
///
/// WAV appends blocks directly to the file. FLAC encodes each block as a FLAC
/// frame and writes the complete stream on `finish`.
pub struct PcmStreamWriter {
    inner: PcmStreamWriterInner,
}

enum PcmStreamWriterInner {
    Wav(WavStreamWriter),
    Flac(FlacStreamWriter),
}

impl PcmStreamWriter {
    pub fn create(
        path: &str,
        sample_rate: u32,
        channels: u16,
        bits_per_sample: u16,
        format: PcmFormat,
        metadata: &HashMap<String, String>,
    ) -> Result<Self, anyhow::Error> {
        if sample_rate == 0 {
            return Err(anyhow!("sample rate must be greater than zero"));
        }
        if channels == 0 {
            return Err(anyhow!("channel count must be greater than zero"));
        }
        if !matches!(bits_per_sample, 16 | 24) {
            return Err(anyhow!(
                "unsupported bit depth {bits_per_sample}; expected 16 or 24"
            ));
        }

        Ok(Self {
            inner: match format {
                PcmFormat::Wav => PcmStreamWriterInner::Wav(WavStreamWriter::create(
                    path,
                    sample_rate,
                    channels,
                    bits_per_sample,
                    metadata,
                )?),
                PcmFormat::Flac => PcmStreamWriterInner::Flac(FlacStreamWriter::create(
                    path,
                    sample_rate,
                    channels,
                    bits_per_sample,
                    metadata,
                )?),
            },
        })
    }

    pub fn write_block(&mut self, samples: &[f32]) -> Result<(), anyhow::Error> {
        if samples.is_empty() {
            return Ok(());
        }
        match &mut self.inner {
            PcmStreamWriterInner::Wav(writer) => writer.write_block(samples),
            PcmStreamWriterInner::Flac(writer) => writer.write_block(samples),
        }
    }

    pub fn finish(self) -> Result<(), anyhow::Error> {
        match self.inner {
            PcmStreamWriterInner::Wav(writer) => writer.finish(),
            PcmStreamWriterInner::Flac(writer) => writer.finish(),
        }
    }
}

struct WavStreamWriter {
    file: File,
    channels: u16,
    bits_per_sample: u16,
    metadata_bytes: usize,
    data_size_offset: u64,
    data_bytes: u64,
}

impl WavStreamWriter {
    fn create(
        path: &str,
        sample_rate: u32,
        channels: u16,
        bits_per_sample: u16,
        metadata: &HashMap<String, String>,
    ) -> Result<Self, anyhow::Error> {
        let list = build_wav_info_chunk(metadata);
        let bytes_per_sample = u32::from(bits_per_sample) / 8;
        let block_align = (u32::from(channels) * bytes_per_sample) as u16;
        let byte_rate = sample_rate * u32::from(block_align);
        let mut file =
            File::create(path).with_context(|| format!("failed to create WAV {path}"))?;

        file.write_all(b"RIFF")?;
        file.write_all(&0u32.to_le_bytes())?;
        file.write_all(b"WAVE")?;
        file.write_all(b"fmt ")?;
        file.write_all(&16u32.to_le_bytes())?;
        file.write_all(&1u16.to_le_bytes())?;
        file.write_all(&channels.to_le_bytes())?;
        file.write_all(&sample_rate.to_le_bytes())?;
        file.write_all(&byte_rate.to_le_bytes())?;
        file.write_all(&block_align.to_le_bytes())?;
        file.write_all(&bits_per_sample.to_le_bytes())?;

        let mut metadata_bytes = 0usize;
        if !list.is_empty() {
            file.write_all(&list)?;
            metadata_bytes = list.len();
        }

        file.write_all(b"data")?;
        let data_size_offset = file.stream_position()?;
        file.write_all(&0u32.to_le_bytes())?;

        let mut writer = Self {
            file,
            channels,
            bits_per_sample,
            metadata_bytes,
            data_size_offset,
            data_bytes: 0,
        };
        writer.update_sizes()?;
        Ok(writer)
    }

    fn write_block(&mut self, samples: &[f32]) -> Result<(), anyhow::Error> {
        if samples.len() % usize::from(self.channels) != 0 {
            return Err(anyhow!(
                "sample count {} is not divisible by channel count {}",
                samples.len(),
                self.channels
            ));
        }

        let quantized = dither_and_quantize(samples, self.bits_per_sample)?;
        let bytes_per_sample = usize::from(self.bits_per_sample) / 8;
        let mut bytes = Vec::with_capacity(quantized.len() * bytes_per_sample);
        for sample in quantized {
            if self.bits_per_sample == 16 {
                bytes.extend_from_slice(&(sample as i16).to_le_bytes());
            } else {
                let le = sample.to_le_bytes();
                bytes.extend_from_slice(&le[..3]);
            }
        }

        self.file.write_all(&bytes)?;
        self.data_bytes += bytes.len() as u64;
        self.update_sizes()
    }

    fn update_sizes(&mut self) -> Result<(), anyhow::Error> {
        let data_size = u32::try_from(self.data_bytes)
            .map_err(|_| anyhow!("WAV data exceeds the 32-bit RIFF size limit"))?;
        let riff_size = 36u32
            .checked_add(self.metadata_bytes as u32)
            .and_then(|size| size.checked_add(data_size))
            .ok_or_else(|| anyhow!("WAV RIFF size overflow"))?;

        self.file.seek(SeekFrom::Start(4))?;
        self.file.write_all(&riff_size.to_le_bytes())?;
        self.file.seek(SeekFrom::Start(self.data_size_offset))?;
        self.file.write_all(&data_size.to_le_bytes())?;
        self.file.seek(SeekFrom::End(0))?;
        Ok(())
    }

    fn finish(mut self) -> Result<(), anyhow::Error> {
        self.update_sizes()?;
        self.file.flush()?;
        Ok(())
    }
}

struct FlacStreamWriter {
    path: String,
    stream: flacenc::component::Stream,
    framebuf: flacenc::source::FrameBuf,
    config: flacenc::error::Verified<flacenc::config::Encoder>,
    channels: u16,
    block_size: usize,
    frame_number: usize,
    pending: Vec<i32>,
    metadata: HashMap<String, String>,
}

impl FlacStreamWriter {
    fn create(
        path: &str,
        sample_rate: u32,
        channels: u16,
        bits_per_sample: u16,
        metadata: &HashMap<String, String>,
    ) -> Result<Self, anyhow::Error> {
        use flacenc::config;
        use flacenc::error::Verify;

        let config = config::Encoder::default()
            .into_verified()
            .map_err(|(_, err)| anyhow!("invalid FLAC encoder config: {err}"))?;
        let block_size = config.block_size;
        let stream = flacenc::component::Stream::new(
            sample_rate as usize,
            usize::from(channels),
            usize::from(bits_per_sample),
        )
        .map_err(|err| anyhow!("failed to initialize FLAC stream: {err}"))?;
        let framebuf =
            flacenc::source::FrameBuf::with_size(usize::from(channels), config.block_size)
                .map_err(|err| anyhow!("failed to initialize FLAC frame buffer: {err}"))?;

        Ok(Self {
            path: path.to_string(),
            stream,
            framebuf,
            config,
            channels,
            block_size,
            frame_number: 0,
            pending: Vec::new(),
            metadata: metadata.clone(),
        })
    }

    fn write_block(&mut self, samples: &[f32]) -> Result<(), anyhow::Error> {
        if samples.len() % usize::from(self.channels) != 0 {
            return Err(anyhow!(
                "sample count {} is not divisible by channel count {}",
                samples.len(),
                self.channels
            ));
        }

        self.pending
            .extend(dither_and_quantize(samples, self.bits_per_sample()?)?);
        let block_samples = self.block_size * usize::from(self.channels);
        while self.pending.len() >= block_samples {
            let block: Vec<i32> = self.pending.drain(..block_samples).collect();
            self.encode_frame(&block)?;
        }
        Ok(())
    }

    fn encode_frame(&mut self, interleaved: &[i32]) -> Result<(), anyhow::Error> {
        use flacenc::encode_fixed_size_frame;
        use flacenc::source::Fill;

        let frames = interleaved.len() / usize::from(self.channels);
        if frames != self.framebuf.size() {
            self.framebuf.resize(frames);
        }
        self.framebuf
            .fill_interleaved(interleaved)
            .map_err(|err| anyhow!("failed to fill FLAC frame: {err}"))?;
        let frame = encode_fixed_size_frame(
            &self.config,
            &self.framebuf,
            self.frame_number,
            self.stream.stream_info(),
        )
        .map_err(|err| anyhow!("failed to encode FLAC frame: {err:?}"))?;
        self.stream.add_frame(frame);
        self.frame_number += 1;
        Ok(())
    }

    fn finish(mut self) -> Result<(), anyhow::Error> {
        use flacenc::bitsink::ByteSink;
        use flacenc::component::{BitRepr, MetadataBlockData};

        if !self.pending.is_empty() {
            let mut block = std::mem::take(&mut self.pending);
            let frames = block.len() / usize::from(self.channels);
            let padded_frames = frames.max(16);
            block.resize(padded_frames * usize::from(self.channels), 0);
            self.encode_frame(&block)?;
        }
        if self.frame_number == 0 {
            return Err(anyhow!("FLAC output contains no audio frames"));
        }

        let vorbis = build_vorbis_comment(&self.metadata);
        let comment_block = MetadataBlockData::new_unknown(4, &vorbis)
            .map_err(|err| anyhow!("failed to build VORBIS_COMMENT block: {err}"))?;
        self.stream.add_metadata_block(comment_block);

        let mut sink = ByteSink::new();
        self.stream
            .write(&mut sink)
            .map_err(|err| anyhow!("FLAC stream serialization failed: {err}"))?;
        fs::write(&self.path, sink.as_slice())
            .with_context(|| format!("failed to write FLAC {}", self.path))
    }

    fn bits_per_sample(&self) -> Result<u16, anyhow::Error> {
        match self.stream.stream_info().bits_per_sample() {
            16 | 24 => Ok(self.stream.stream_info().bits_per_sample() as u16),
            _ => Err(anyhow!("unsupported FLAC bit depth")),
        }
    }
}

/// Encode interleaved Float32 PCM as WAV or FLAC.
///
/// Processing order per sample:
/// 1. Hard clamp to [-1.0, 1.0]
/// 2. Add TPDF dither at one least-significant-bit amplitude
/// 3. Quantize to signed 16-bit or 24-bit and round
pub fn encode_pcm(
    path: &str,
    samples: &[f32],
    sample_rate: u32,
    channels: u16,
    bits_per_sample: u16,
    format: PcmFormat,
    metadata: &HashMap<String, String>,
) -> Result<(), anyhow::Error> {
    validate_inputs(samples, sample_rate, channels, bits_per_sample)?;

    let quantized = dither_and_quantize(samples, bits_per_sample)?;
    match format {
        PcmFormat::Wav => encode_wav(
            path,
            &quantized,
            sample_rate,
            channels,
            bits_per_sample,
            metadata,
        ),
        PcmFormat::Flac => encode_flac(
            path,
            &quantized,
            sample_rate,
            channels,
            bits_per_sample,
            metadata,
        ),
    }
}

fn validate_inputs(
    samples: &[f32],
    sample_rate: u32,
    channels: u16,
    bits_per_sample: u16,
) -> Result<(), anyhow::Error> {
    if samples.is_empty() {
        return Err(anyhow!("cannot encode an empty sample buffer"));
    }
    if sample_rate == 0 {
        return Err(anyhow!("sample rate must be greater than zero"));
    }
    if channels == 0 {
        return Err(anyhow!("channel count must be greater than zero"));
    }
    if !matches!(bits_per_sample, 16 | 24) {
        return Err(anyhow!(
            "unsupported bit depth {bits_per_sample}; expected 16 or 24"
        ));
    }
    if samples.len() % usize::from(channels) != 0 {
        return Err(anyhow!(
            "sample count {} is not divisible by channel count {channels}",
            samples.len()
        ));
    }
    Ok(())
}

fn dither_and_quantize(samples: &[f32], bits_per_sample: u16) -> Result<Vec<i32>, anyhow::Error> {
    let scale = 1i32
        .checked_shl(u32::from(bits_per_sample) - 1)
        .ok_or_else(|| anyhow!("invalid bit depth for quantization"))?;
    let scale_f = scale as f32;
    let lsb_amplitude = 1.0 / scale_f;
    let min = -scale_f;
    let max = scale_f - 1.0;

    Ok(samples
        .iter()
        .map(|&sample| {
            let clamped = if sample.is_finite() {
                sample.clamp(-1.0, 1.0)
            } else {
                0.0
            };
            let dither =
                (random::<f32>() * 2.0 - 1.0 + random::<f32>() * 2.0 - 1.0) * 0.5 * lsb_amplitude;
            ((clamped + dither) * scale_f).round().clamp(min, max) as i32
        })
        .collect())
}

fn encode_wav(
    path: &str,
    quantized: &[i32],
    sample_rate: u32,
    channels: u16,
    bits_per_sample: u16,
    metadata: &HashMap<String, String>,
) -> Result<(), anyhow::Error> {
    let spec = hound::WavSpec {
        channels,
        sample_rate,
        bits_per_sample,
        sample_format: hound::SampleFormat::Int,
    };
    {
        let mut writer = hound::WavWriter::create(path, spec)
            .with_context(|| format!("failed to open {path}"))?;
        for &sample in quantized {
            if bits_per_sample == 16 {
                writer.write_sample(sample as i16)?;
            } else {
                writer.write_sample(sample)?;
            }
        }
        writer.finalize()?;
    }

    let original =
        fs::read(path).with_context(|| format!("failed to read generated WAV {path}"))?;
    let patched = insert_wav_list_chunk(&original, metadata)?;
    fs::write(path, patched).with_context(|| format!("failed to write final WAV {path}"))
}

fn insert_wav_list_chunk(
    original: &[u8],
    metadata: &HashMap<String, String>,
) -> Result<Vec<u8>, anyhow::Error> {
    if original.len() < 12 || &original[0..4] != b"RIFF" || &original[8..12] != b"WAVE" {
        return Err(anyhow!("generated WAV does not look like a RIFF/WAVE file"));
    }

    let mut position = 12usize;
    let mut data_offset = None;
    while position + 8 <= original.len() {
        let chunk_id = &original[position..position + 4];
        let chunk_size = read_u32(&original[position + 4..position + 8]) as usize;
        let body = position + 8;
        if chunk_id == b"data" {
            data_offset = Some(position);
            break;
        }
        position = body + chunk_size + (chunk_size & 1);
    }

    let data_offset = data_offset.ok_or_else(|| anyhow!("WAV data chunk not found"))?;
    let list = build_wav_info_chunk(metadata);
    if list.is_empty() {
        return Ok(original.to_vec());
    }

    let mut patched = Vec::with_capacity(original.len() + list.len());
    patched.extend_from_slice(&original[..data_offset]);
    patched.extend_from_slice(&list);
    patched.extend_from_slice(&original[data_offset..]);

    let riff_size = patched
        .len()
        .checked_sub(8)
        .ok_or_else(|| anyhow!("patched WAV is too small"))? as u32;
    patched[4..8].copy_from_slice(&riff_size.to_le_bytes());
    Ok(patched)
}

fn build_wav_info_chunk(metadata: &HashMap<String, String>) -> Vec<u8> {
    let mut tags: BTreeMap<&'static str, Vec<String>> = BTreeMap::new();
    let mut unknown = Vec::new();

    for (key, value) in metadata {
        if let Some(field) = wav_info_field(key) {
            tags.entry(field).or_default().push(value.clone());
        } else {
            unknown.push(format!("{key}={value}"));
        }
    }

    if !unknown.is_empty() {
        tags.entry("ICMT").or_default().push(unknown.join("; "));
    }
    if tags.is_empty() {
        return Vec::new();
    }

    let mut info = b"INFO".to_vec();
    for (field, values) in tags {
        append_info_field(&mut info, field, &values.join("; "));
    }

    let mut chunk = b"LIST".to_vec();
    chunk.extend_from_slice(&(info.len() as u32).to_le_bytes());
    chunk.extend_from_slice(&info);
    chunk
}

fn append_info_field(info: &mut Vec<u8>, field: &str, value: &str) {
    info.extend_from_slice(field.as_bytes());
    let bytes = value.as_bytes();
    info.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    info.extend_from_slice(bytes);
    if bytes.len() % 2 == 1 {
        info.push(0);
    }
}

fn wav_info_field(key: &str) -> Option<&'static str> {
    match key.to_ascii_lowercase().as_str() {
        "title" | "tracktitle" | "tit2" | "inam" => Some("INAM"),
        "artist" | "artistperformer" | "tpe1" | "iart" => Some("IART"),
        "album" | "talb" | "iprd" => Some("IPRD"),
        "comment" | "icmt" => Some("ICMT"),
        "genre" | "tgnr" | "ignr" => Some("IGNR"),
        "date" | "year" | "tyer" | "tdrc" | "icrd" => Some("ICRD"),
        "track" | "tracknumber" | "trck" | "trkn" | "itrk" => Some("ITRK"),
        "encoder" | "software" | "encodedby" | "tsse" | "isft" => Some("ISFT"),
        "composer" | "tcom" | "imus" => Some("IMUS"),
        _ => None,
    }
}

fn encode_flac(
    path: &str,
    quantized: &[i32],
    sample_rate: u32,
    channels: u16,
    bits_per_sample: u16,
    metadata: &HashMap<String, String>,
) -> Result<(), anyhow::Error> {
    use flacenc::bitsink::ByteSink;
    use flacenc::component::{BitRepr, MetadataBlockData};
    use flacenc::error::Verify;

    let config = flacenc::config::Encoder::default()
        .into_verified()
        .map_err(|(_, err)| anyhow!("invalid FLAC encoder config: {err}"))?;
    let source = flacenc::source::MemSource::from_samples(
        quantized,
        usize::from(channels),
        usize::from(bits_per_sample),
        sample_rate as usize,
    );
    let mut stream = flacenc::encode_with_fixed_block_size(&config, source, config.block_size)
        .map_err(|err| anyhow!("FLAC encoding failed: {err:?}"))?;

    let vorbis = build_vorbis_comment(metadata);
    let comment_block = MetadataBlockData::new_unknown(4, &vorbis)
        .map_err(|err| anyhow!("failed to build VORBIS_COMMENT block: {err}"))?;
    stream.add_metadata_block(comment_block);

    let mut sink = ByteSink::new();
    stream
        .write(&mut sink)
        .map_err(|err| anyhow!("FLAC stream serialization failed: {err}"))?;
    fs::write(path, sink.as_slice()).with_context(|| format!("failed to write FLAC {path}"))
}

fn build_vorbis_comment(metadata: &HashMap<String, String>) -> Vec<u8> {
    let vendor = "audio_engine";
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&(vendor.len() as u32).to_le_bytes());
    bytes.extend_from_slice(vendor.as_bytes());

    let mut comments = Vec::new();
    for (key, value) in metadata {
        comments.push(format!("{}={value}", key.to_ascii_uppercase()));
    }
    comments.sort();

    bytes.extend_from_slice(&(comments.len() as u32).to_le_bytes());
    for comment in comments {
        bytes.extend_from_slice(&(comment.len() as u32).to_le_bytes());
        bytes.extend_from_slice(comment.as_bytes());
    }
    bytes
}

fn read_u32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes(bytes.try_into().expect("u32 slice must be four bytes"))
}
