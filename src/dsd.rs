use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};

use anyhow::{anyhow, Context};

use crate::decoder::AudioData;
use crate::encoder::{PcmFormat, PcmStreamWriter};

const DSF_BLOCK_SIZE: usize = 4096;
const DSF_FMT_CHUNK_SIZE: u64 = 52;
const DSF_DATA_HEADER_SIZE: u64 = 12;

/// Packed 1-bit DSD audio.
///
/// `data` stores channel-byte-interleaved DSD samples:
///
/// - 8 samples occupy one byte
/// - bit 7 is the oldest sample and bit 0 is the newest sample
/// - channel 0 byte, channel 1 byte, channel 0 byte, channel 1 byte, ...
///
/// `sample_rate` is the DSD bit rate, e.g. 2,822,400 Hz for DSD64.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DsdStream {
    pub data: Vec<u8>,
    pub sample_rate: u32,
    pub channels: u16,
}

/// Write a DSF container around a packed DSD stream.
///
/// DSF stores channel data in 4096-byte channel blocks and uses LSB-first
/// sample order. The writer converts the MSB-first `DsdStream` layout into the
/// DSF layout internally, so the same stream can be written as DSF or DFF.
pub fn encode_dsf(
    path: &str,
    stream: &DsdStream,
    metadata: &HashMap<String, String>,
) -> Result<(), anyhow::Error> {
    validate_stream(stream)?;

    let channels = usize::from(stream.channels);
    let bytes_per_channel = stream.data.len() / channels;
    let sample_count = bytes_per_channel as u64 * 8;
    let blocks = bytes_per_channel.div_ceil(DSF_BLOCK_SIZE);
    let dsf_data_len = (blocks * channels * DSF_BLOCK_SIZE) as u64;
    let data_chunk_size = DSF_DATA_HEADER_SIZE + dsf_data_len;
    let audio_end = 28 + DSF_FMT_CHUNK_SIZE + data_chunk_size;

    let id3 = build_id3_tag(metadata)?;
    let metadata_ptr = if id3.is_empty() { 0 } else { audio_end };
    let total_size = audio_end + id3.len() as u64;

    let mut writer = BufWriter::new(
        File::create(path).with_context(|| format!("failed to create DSF {path}"))?,
    );
    writer.write_all(b"DSD ")?;
    writer.write_all(&28u64.to_le_bytes())?;
    writer.write_all(&total_size.to_le_bytes())?;
    writer.write_all(&metadata_ptr.to_le_bytes())?;

    writer.write_all(b"fmt ")?;
    writer.write_all(&DSF_FMT_CHUNK_SIZE.to_le_bytes())?;
    writer.write_all(&1u32.to_le_bytes())?;
    writer.write_all(&0u32.to_le_bytes())?;
    writer.write_all(&dsf_channel_type(stream.channels).to_le_bytes())?;
    writer.write_all(&u32::from(stream.channels).to_le_bytes())?;
    writer.write_all(&stream.sample_rate.to_le_bytes())?;
    writer.write_all(&1u32.to_le_bytes())?;
    writer.write_all(&sample_count.to_le_bytes())?;
    writer.write_all(&(DSF_BLOCK_SIZE as u32).to_le_bytes())?;
    writer.write_all(&0u32.to_le_bytes())?;

    writer.write_all(b"data")?;
    writer.write_all(&data_chunk_size.to_le_bytes())?;

    let mut block = Vec::with_capacity(DSF_BLOCK_SIZE);
    for block_index in 0..blocks {
        let start = block_index * DSF_BLOCK_SIZE;
        let end = (start + DSF_BLOCK_SIZE).min(bytes_per_channel);
        for channel in 0..channels {
            let channel_offset = channel * bytes_per_channel;
            block.clear();
            for &byte in &stream.data[channel_offset + start..channel_offset + end] {
                block.push(reverse_bits(byte));
            }
            block.resize(DSF_BLOCK_SIZE, 0);
            writer.write_all(&block)?;
        }
    }
    writer.write_all(&id3)?;
    writer.flush()?;
    Ok(())
}

/// Read a DSF container and reconstruct the packed MSB-first interleaved DSD bytes.
pub fn decode_dsf(path: &str) -> Result<DsdStream, anyhow::Error> {
    let bytes = std::fs::read(path).with_context(|| format!("failed to read DSF {path}"))?;
    if bytes.len() < 92 || &bytes[0..4] != b"DSD " {
        return Err(anyhow!("invalid DSF header: {path}"));
    }

    let sample_rate = read_le_u32(&bytes, 56)?;
    let channels = read_le_u32(&bytes, 52)?;
    if sample_rate == 0 || channels == 0 {
        return Err(anyhow!("DSF stream parameters are invalid in {path}"));
    }
    if bytes.len() < 92 || &bytes[80..84] != b"data" {
        return Err(anyhow!("missing DSF data chunk in {path}"));
    }
    let data_start = 92;
    let raw = &bytes[data_start..];
    let channels_usize = channels as usize;
    let bytes_per_channel = raw.len() / channels_usize;
    let block_size = 4096usize;
    let blocks = bytes_per_channel.div_ceil(block_size);
    let mut dsd_data = Vec::with_capacity(raw.len());
    for block in 0..blocks {
        let start = block * block_size;
        let end = (start + block_size).min(bytes_per_channel);
        let len = end - start;
        for channel in 0..channels_usize {
            let offset = channel * bytes_per_channel + start;
            dsd_data.extend(
                raw[offset..offset + len]
                    .iter()
                    .map(|byte| byte.reverse_bits()),
            );
        }
    }
    Ok(DsdStream {
        data: dsd_data,
        sample_rate,
        channels: channels as u16,
    })
}

/// Read a DSDIFF/DFF container and reconstruct packed MSB-first interleaved DSD bytes.
pub fn decode_dff(path: &str) -> Result<DsdStream, anyhow::Error> {
    let bytes = std::fs::read(path).with_context(|| format!("failed to read DFF {path}"))?;
    if bytes.len() < 16 || &bytes[0..4] != b"FRM8" || &bytes[12..16] != b"DSD " {
        return Err(anyhow!("invalid DFF header: {path}"));
    }
    let root_start = 12;
    let root_end = bytes.len();
    let mut pos = root_start + 4;

    let mut sample_rate = 0u32;
    let mut channels = 0u16;
    let mut data: Option<Vec<u8>> = None;

    while pos + 12 <= root_end {
        let id = &bytes[pos..pos + 4];
        let size = read_be_u64(&bytes, pos + 4)? as usize;
        let data_start = pos + 12;
        let data_end = data_start.saturating_add(size).min(root_end);
        if id == b"PROP" {
            let mut sub = data_start + 4;
            while sub + 12 <= data_end {
                let sub_id = &bytes[sub..sub + 4];
                let sub_size = read_be_u64(&bytes, sub + 4)? as usize;
                let sub_data = sub + 12;
                if sub_id == b"FS  " && sample_rate == 0 && sub_data + 4 <= data_end {
                    sample_rate = read_be_u32(&bytes, sub_data)?;
                }
                if sub_id == b"CHNL" && channels == 0 && sub_data + 2 <= data_end {
                    channels = read_be_u16(&bytes, sub_data)?;
                }
                sub += 12 + sub_size + (sub_size % 2);
            }
        } else if id == b"DSD " && data.is_none() {
            data = Some(bytes[data_start..data_end].to_vec());
        }
        pos += 12 + size + (size % 2);
    }

    if sample_rate == 0 || channels == 0 {
        return Err(anyhow!("DFF could not determine sample rate or channels in {path}"));
    }
    let data = data.ok_or_else(|| anyhow!("missing DFF DSD data chunk in {path}"))?;
    Ok(DsdStream {
        data,
        sample_rate,
        channels,
    })
}

/// Convert packed 1-bit DSD into interleaved Float32 PCM using 64x boxcar decimation.
pub fn dsd_to_pcm(stream: &DsdStream, output_rate: u32) -> Result<AudioData, anyhow::Error> {
    if stream.sample_rate == 0 || output_rate == 0 {
        return Err(anyhow!("DSD and PCM sample rates must be greater than zero"));
    }
    let ratio = (stream.sample_rate / output_rate) as usize;
    if ratio == 0 || ratio > 256 {
        return Err(anyhow!(
            "unsupported DSD-to-PCM ratio {} for {} Hz output",
            ratio,
            output_rate
        ));
    }

    let channels = usize::from(stream.channels);
    let bytes_per_channel = stream.data.len() / channels;
    let bits_per_channel = bytes_per_channel * 8;
    let frames = bits_per_channel / ratio;
    let mut samples = Vec::with_capacity(frames * channels);

    for frame in 0..frames {
        let start = frame * ratio;
        for channel in 0..channels {
            let mut sum = 0.0f32;
            for offset in 0..ratio {
                let bit_index = start + offset;
                let byte = stream.data[(bit_index / 8) * channels + channel];
                let mask = 1u8 << (7 - (bit_index % 8));
                sum += if byte & mask != 0 { 1.0 } else { -1.0 };
            }
            samples.push(sum / ratio as f32);
        }
    }

    Ok(AudioData {
        samples,
        sample_rate: output_rate,
        channels: stream.channels,
        bits_per_sample: 1,
        total_frames: frames as u64,
        metadata: HashMap::new(),
    })
}

/// Decode a DSF/DFF file directly to a PCM file in fixed-size blocks.
///
/// This keeps memory bounded by the block size instead of materializing the
/// complete Float32 PCM stream.
pub fn decode_dsd_to_pcm_file(
    path: &str,
    output_path: &str,
    output_rate: u32,
    bit_depth: u16,
    format: PcmFormat,
) -> Result<(), anyhow::Error> {
    let stream = if extension(path).as_deref() == Some("dsf") {
        decode_dsf(path)?
    } else {
        decode_dff(path)?
    };
    let channels = usize::from(stream.channels);
    if stream.sample_rate == 0 || output_rate == 0 {
        return Err(anyhow!("DSD and PCM sample rates must be greater than zero"));
    }
    let ratio = (stream.sample_rate / output_rate) as usize;
    if ratio == 0 || ratio > 256 {
        return Err(anyhow!("unsupported DSD-to-PCM ratio {ratio}"));
    }

    let bytes_per_channel = stream.data.len() / channels;
    let bits_per_channel = bytes_per_channel * 8;
    let total_frames = bits_per_channel / ratio;
    let mut writer = PcmStreamWriter::create(
        output_path,
        output_rate,
        stream.channels,
        bit_depth,
        format,
        &HashMap::new(),
    )?;

    const BLOCK_FRAMES: usize = 4096;
    let mut frame = 0usize;
    while frame < total_frames {
        let take = BLOCK_FRAMES.min(total_frames - frame);
        let mut block = Vec::with_capacity(take * channels);
        for offset in 0..take {
            let start = (frame + offset) * ratio;
            for channel in 0..channels {
                let mut sum = 0.0f32;
                for bit in 0..ratio {
                    let bit_index = start + bit;
                    let byte = stream.data[(bit_index / 8) * channels + channel];
                    let mask = 1u8 << (7 - (bit_index % 8));
                    sum += if byte & mask != 0 { 1.0 } else { -1.0 };
                }
                block.push(sum / ratio as f32);
            }
        }
        writer.write_block(&block)?;
        frame += take;
    }
    writer.finish()
}

fn extension(path: &str) -> Option<String> {
    std::path::Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
}

/// Write a DSDIFF/DFF container around a packed DSD stream.
pub fn encode_dff(
    path: &str,
    stream: &DsdStream,
    metadata: &HashMap<String, String>,
) -> Result<(), anyhow::Error> {
    validate_stream(stream)?;

    let mut prop = b"SND ".to_vec();
    append_dff_chunk(&mut prop, b"FS  ", &stream.sample_rate.to_be_bytes());
    append_dff_chunk(&mut prop, b"CHNL", &build_channel_chunk(stream.channels));

    let mut cmpr = Vec::with_capacity(18);
    cmpr.extend_from_slice(b"DSD ");
    cmpr.extend_from_slice(&14u32.to_be_bytes());
    cmpr.extend_from_slice(b"not compressed");
    append_dff_chunk(&mut prop, b"CMPR", &cmpr);

    let diin = build_diin(metadata);

    let fver_chunk = dff_chunk_padded_len(4);
    let prop_chunk = dff_chunk_padded_len(prop.len());
    let dsd_chunk = dff_chunk_padded_len(stream.data.len());
    let diin_chunk = if diin.is_empty() {
        0
    } else {
        dff_chunk_padded_len(diin.len())
    };
    let root_len = 4 + fver_chunk + prop_chunk + dsd_chunk + diin_chunk;

    let mut writer = BufWriter::new(
        File::create(path).with_context(|| format!("failed to create DFF {path}"))?,
    );
    writer.write_all(b"FRM8")?;
    writer.write_all(&root_len.to_be_bytes())?;
    writer.write_all(b"DSD ")?;
    write_dff_chunk(&mut writer, b"FVER", &0x0105_0000u32.to_be_bytes())?;
    write_dff_chunk(&mut writer, b"PROP", &prop)?;
    write_dff_chunk(&mut writer, b"DSD ", &stream.data)?;
    if !diin.is_empty() {
        write_dff_chunk(&mut writer, b"DIIN", &diin)?;
    }
    writer.flush()?;
    Ok(())
}

fn validate_stream(stream: &DsdStream) -> Result<(), anyhow::Error> {
    if stream.data.is_empty() {
        return Err(anyhow!("cannot encode an empty DSD stream"));
    }
    if stream.sample_rate == 0 {
        return Err(anyhow!("DSD sample rate must be greater than zero"));
    }
    if stream.channels == 0 {
        return Err(anyhow!("DSD channel count must be greater than zero"));
    }
    if stream.data.len() % usize::from(stream.channels) != 0 {
        return Err(anyhow!(
            "DSD data length {} is not divisible by channel count {}",
            stream.data.len(),
            stream.channels
        ));
    }
    Ok(())
}

fn dsf_channel_type(channels: u16) -> u32 {
    match channels {
        1 => 1,
        2 => 2,
        3 => 3,
        4 => 4,
        5 => 5,
        6 => 7,
        _ => 0,
    }
}

fn reverse_bits(byte: u8) -> u8 {
    byte.reverse_bits()
}

fn dff_chunk_padded_len(data_len: usize) -> u64 {
    12 + data_len as u64 + (data_len % 2) as u64
}

fn write_dff_chunk<W: Write>(
    writer: &mut W,
    id: &[u8; 4],
    data: &[u8],
) -> Result<(), anyhow::Error> {
    writer.write_all(id)?;
    writer.write_all(&(data.len() as u64).to_be_bytes())?;
    writer.write_all(data)?;
    if data.len() % 2 == 1 {
        writer.write_all(&[0])?;
    }
    Ok(())
}

fn append_dff_chunk(out: &mut Vec<u8>, id: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(id);
    out.extend_from_slice(&(data.len() as u64).to_be_bytes());
    out.extend_from_slice(data);
    if data.len() % 2 == 1 {
        out.push(0);
    }
}

fn build_channel_chunk(channels: u16) -> Vec<u8> {
    let mut data = Vec::with_capacity(2 + usize::from(channels) * 4);
    data.extend_from_slice(&channels.to_be_bytes());
    for channel in 0..usize::from(channels) {
        data.extend_from_slice(&dff_channel_id(channel, usize::from(channels)));
    }
    data
}

fn dff_channel_id(channel: usize, channels: usize) -> [u8; 4] {
    match channels {
        2 => match channel {
            0 => *b"SLFT",
            _ => *b"SRGT",
        },
        5 => match channel {
            0 => *b"MLFT",
            1 => *b"MRGT",
            2 => *b"C   ",
            3 => *b"LS  ",
            _ => *b"RS  ",
        },
        6 => match channel {
            0 => *b"MLFT",
            1 => *b"MRGT",
            2 => *b"C   ",
            3 => *b"LFE ",
            4 => *b"LS  ",
            _ => *b"RS  ",
        },
        _ => {
            let id = format!("C{:03}", channel % 1000);
            let bytes = id.as_bytes();
            [bytes[0], bytes[1], bytes[2], bytes[3]]
        }
    }
}

fn build_diin(metadata: &HashMap<String, String>) -> Vec<u8> {
    let mut diin = Vec::new();

    if let Some(artist) = lookup_value(metadata, &["artist", "artistperformer"]) {
        if !artist.is_empty() {
            let mut chunk = Vec::with_capacity(4 + artist.len());
            chunk.extend_from_slice(&(artist.len() as u32).to_be_bytes());
            chunk.extend_from_slice(artist.as_bytes());
            append_dff_chunk(&mut diin, b"DIAR", &chunk);
        }
    }

    if let Some(title) = lookup_value(metadata, &["title", "tracktitle"]) {
        if !title.is_empty() {
            let mut chunk = Vec::with_capacity(4 + title.len());
            chunk.extend_from_slice(&(title.len() as u32).to_be_bytes());
            chunk.extend_from_slice(title.as_bytes());
            append_dff_chunk(&mut diin, b"DITI", &chunk);
        }
    }

    diin
}

fn lookup_value<'a>(metadata: &'a HashMap<String, String>, keys: &[&str]) -> Option<&'a str> {
    metadata
        .iter()
        .find(|(key, _)| {
            keys.iter()
                .any(|candidate| key.eq_ignore_ascii_case(candidate))
        })
        .map(|(_, value)| value.as_str())
}

fn build_id3_tag(metadata: &HashMap<String, String>) -> Result<Vec<u8>, anyhow::Error> {
    let mut frames = Vec::new();
    let mut unknown: Vec<String> = Vec::new();

    for (key, value) in metadata {
        if value.is_empty() {
            continue;
        }
        if key.eq_ignore_ascii_case("comment") {
            push_id3_comment_frame(&mut frames, value)?;
        } else if let Some(frame_id) = id3_text_frame(key) {
            push_id3_text_frame(&mut frames, frame_id, value)?;
        } else {
            unknown.push(format!("{key}={value}"));
        }
    }
    if !unknown.is_empty() {
        push_id3_comment_frame(&mut frames, &unknown.join("; "))?;
    }
    if frames.is_empty() {
        return Ok(Vec::new());
    }

    let tag_size = synchsafe_u32(frames.len() as u32)?;
    let mut tag = b"ID3".to_vec();
    tag.extend_from_slice(&[4, 0, 0]);
    tag.extend_from_slice(&tag_size);
    tag.extend_from_slice(&frames);
    Ok(tag)
}

fn id3_text_frame(key: &str) -> Option<&'static [u8; 4]> {
    match key.to_ascii_lowercase().as_str() {
        "title" | "tracktitle" => Some(b"TIT2"),
        "artist" | "artistperformer" => Some(b"TPE1"),
        "album" => Some(b"TALB"),
        "albumartist" | "album_artist" => Some(b"TPE2"),
        "composer" => Some(b"TCOM"),
        "genre" => Some(b"TCON"),
        "date" | "year" => Some(b"TDRC"),
        "track" | "tracknumber" => Some(b"TRCK"),
        "encoder" | "software" => Some(b"TSSE"),
        _ => None,
    }
}

fn push_id3_text_frame(
    frames: &mut Vec<u8>,
    frame_id: &[u8; 4],
    value: &str,
) -> Result<(), anyhow::Error> {
    let mut body = Vec::with_capacity(1 + value.len());
    body.push(3);
    body.extend_from_slice(value.as_bytes());
    push_id3_frame(frames, frame_id, &body)
}

fn push_id3_comment_frame(frames: &mut Vec<u8>, value: &str) -> Result<(), anyhow::Error> {
    let mut body = Vec::with_capacity(5 + value.len());
    body.push(3);
    body.extend_from_slice(b"eng");
    body.push(0);
    body.extend_from_slice(value.as_bytes());
    push_id3_frame(frames, b"COMM", &body)
}

fn push_id3_frame(
    frames: &mut Vec<u8>,
    frame_id: &[u8; 4],
    body: &[u8],
) -> Result<(), anyhow::Error> {
    frames.extend_from_slice(frame_id);
    frames.extend_from_slice(&synchsafe_u32(body.len() as u32)?);
    frames.extend_from_slice(&[0, 0]);
    frames.extend_from_slice(body);
    Ok(())
}

fn synchsafe_u32(value: u32) -> Result<[u8; 4], anyhow::Error> {
    if value > 0x0fff_ffff {
        return Err(anyhow!("ID3v2 frame or tag is too large"));
    }
    let mut out = [0u8; 4];
    let mut remaining = value;
    for byte in out.iter_mut().rev() {
        *byte = (remaining & 0x7f) as u8;
        remaining >>= 7;
    }
    Ok(out)
}

fn read_le_u32(bytes: &[u8], offset: usize) -> Result<u32, anyhow::Error> {
    let slice = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| anyhow!("truncated little-endian u32"))?;
    Ok(u32::from_le_bytes(slice.try_into().unwrap()))
}

fn read_be_u32(bytes: &[u8], offset: usize) -> Result<u32, anyhow::Error> {
    let slice = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| anyhow!("truncated big-endian u32"))?;
    Ok(u32::from_be_bytes(slice.try_into().unwrap()))
}

fn read_be_u16(bytes: &[u8], offset: usize) -> Result<u16, anyhow::Error> {
    let slice = bytes
        .get(offset..offset + 2)
        .ok_or_else(|| anyhow!("truncated big-endian u16"))?;
    Ok(u16::from_be_bytes(slice.try_into().unwrap()))
}

fn read_be_u64(bytes: &[u8], offset: usize) -> Result<u64, anyhow::Error> {
    let slice = bytes
        .get(offset..offset + 8)
        .ok_or_else(|| anyhow!("truncated big-endian u64"))?;
    Ok(u64::from_be_bytes(slice.try_into().unwrap()))
}
