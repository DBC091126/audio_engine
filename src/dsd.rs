use std::collections::HashMap;
use std::fs;

use anyhow::{anyhow, Context};

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
    let dsf_data = interleave_to_dsf_blocks(&stream.data, channels)?;
    let data_chunk_size = DSF_DATA_HEADER_SIZE + dsf_data.len() as u64;
    let audio_end = 28 + DSF_FMT_CHUNK_SIZE + data_chunk_size;

    let id3 = build_id3_tag(metadata)?;
    let metadata_ptr = if id3.is_empty() { 0 } else { audio_end };
    let total_size = audio_end + id3.len() as u64;

    let mut file = Vec::with_capacity(total_size as usize);
    file.extend_from_slice(b"DSD ");
    file.extend_from_slice(&28u64.to_le_bytes());
    file.extend_from_slice(&total_size.to_le_bytes());
    file.extend_from_slice(&metadata_ptr.to_le_bytes());

    file.extend_from_slice(b"fmt ");
    file.extend_from_slice(&DSF_FMT_CHUNK_SIZE.to_le_bytes());
    file.extend_from_slice(&1u32.to_le_bytes());
    file.extend_from_slice(&0u32.to_le_bytes());
    file.extend_from_slice(&dsf_channel_type(stream.channels).to_le_bytes());
    file.extend_from_slice(&u32::from(stream.channels).to_le_bytes());
    file.extend_from_slice(&stream.sample_rate.to_le_bytes());
    file.extend_from_slice(&1u32.to_le_bytes());
    file.extend_from_slice(&sample_count.to_le_bytes());
    file.extend_from_slice(&(DSF_BLOCK_SIZE as u32).to_le_bytes());
    file.extend_from_slice(&0u32.to_le_bytes());

    file.extend_from_slice(b"data");
    file.extend_from_slice(&data_chunk_size.to_le_bytes());
    file.extend_from_slice(&dsf_data);
    file.extend_from_slice(&id3);

    fs::write(path, file).with_context(|| format!("failed to write DSF {path}"))
}

/// Write a DSDIFF/DFF container around a packed DSD stream.
pub fn encode_dff(
    path: &str,
    stream: &DsdStream,
    metadata: &HashMap<String, String>,
) -> Result<(), anyhow::Error> {
    validate_stream(stream)?;

    let mut root = b"DSD ".to_vec();

    append_dff_chunk(&mut root, b"FVER", &0x0105_0000u32.to_be_bytes());

    let mut prop = b"SND ".to_vec();
    append_dff_chunk(&mut prop, b"FS  ", &stream.sample_rate.to_be_bytes());
    append_dff_chunk(&mut prop, b"CHNL", &build_channel_chunk(stream.channels));

    let mut cmpr = Vec::with_capacity(18);
    cmpr.extend_from_slice(b"DSD ");
    cmpr.extend_from_slice(&14u32.to_be_bytes());
    cmpr.extend_from_slice(b"not compressed");
    append_dff_chunk(&mut prop, b"CMPR", &cmpr);
    append_dff_chunk(&mut root, b"PROP", &prop);

    append_dff_chunk(&mut root, b"DSD ", &stream.data);

    let diin = build_diin(metadata);
    if !diin.is_empty() {
        append_dff_chunk(&mut root, b"DIIN", &diin);
    }

    let mut file = b"FRM8".to_vec();
    file.extend_from_slice(&(root.len() as u64).to_be_bytes());
    file.extend_from_slice(&root);

    fs::write(path, file).with_context(|| format!("failed to write DFF {path}"))
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

fn interleave_to_dsf_blocks(data: &[u8], channels: usize) -> Result<Vec<u8>, anyhow::Error> {
    let bytes_per_channel = data.len() / channels;
    let blocks = bytes_per_channel.div_ceil(DSF_BLOCK_SIZE);
    let mut out = Vec::with_capacity(blocks * channels * DSF_BLOCK_SIZE);

    for block in 0..blocks {
        let start = block * DSF_BLOCK_SIZE;
        let end = (start + DSF_BLOCK_SIZE).min(bytes_per_channel);
        for channel in 0..channels {
            let channel_offset = channel * bytes_per_channel;
            for byte in &data[channel_offset + start..channel_offset + end] {
                out.push(reverse_bits(*byte));
            }
            out.resize(out.len() + DSF_BLOCK_SIZE - (end - start), 0);
        }
    }
    Ok(out)
}

fn reverse_bits(byte: u8) -> u8 {
    byte.reverse_bits()
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
