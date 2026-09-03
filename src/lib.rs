pub mod encoder;
pub mod ffi;
#[cfg(feature = "ffmpeg")]
mod ffmpeg_decoder;
mod symphonia_decoder;

pub mod ate;
pub mod decoder;
pub mod dsd;
pub mod dsd_modulator;
pub mod pipeline;
pub mod resampler;

pub use decoder::{decode_file, AudioData};
pub use dsd::{encode_dff, encode_dsf, DsdStream};
pub use dsd_modulator::{pcm_to_dsd, pcm_to_dsd_with_family, DsdMode};
pub use encoder::{encode_pcm, PcmFormat, PcmStreamWriter};
pub use resampler::{get_recommended_rates, validate_rate_family, Resampler};
