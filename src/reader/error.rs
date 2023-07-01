use std::{error::Error, fmt};

use symphonia::core::{audio::Layout, errors::Error as SymphoniaError};

#[derive(Debug)]
pub enum SampleLoadError {
    // Unhandled IO error
    IoError(std::io::Error),
    // Unhandled symphonia error
    SymphoniaError(SymphoniaError),
    NoSupportedAudioTracks,
    UnsupportedCodec,
    UnsupportedChannelLayout(Layout),
    MissingRequiredMetadata(&'static str),
    UnexpectedState(&'static str),
    ResetRequired,
}

impl fmt::Display for SampleLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SampleLoadError::IoError(e) => write!(f, "{}", e),
            SampleLoadError::SymphoniaError(e) => write!(f, "{}", e),
            SampleLoadError::NoSupportedAudioTracks => write!(f, "No supported audio tracks found"),
            SampleLoadError::UnsupportedCodec => write!(f, "Unsupported codec"),
            SampleLoadError::UnsupportedChannelLayout(layout) => {
                write!(f, "Unsupported channel layout {}", layout.into_channels())
            }
            SampleLoadError::MissingRequiredMetadata(msg) => {
                write!(f, "Missing required {} metadata", msg)
            }
            SampleLoadError::UnexpectedState(msg) => write!(f, "Unexpected read state: {}", msg),
            SampleLoadError::ResetRequired => write!(f, "{}", SymphoniaError::ResetRequired),
        }
    }
}

impl Error for SampleLoadError {}

/// Errors used mostly for inner logic and reasoning
#[derive(Debug)]
pub enum SampleDecodeError {
    LoadError(SampleLoadError),
    SkippablePacket,
    EndReached,
    ResetRequired,
}

impl fmt::Display for SampleDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SampleDecodeError::LoadError(e) => write!(f, "{}", e),
            SampleDecodeError::SkippablePacket => {
                write!(f, "{}", SymphoniaError::DecodeError("skippable packet"))
            }
            SampleDecodeError::EndReached => write!(f, "end of stream reached"),
            SampleDecodeError::ResetRequired => write!(f, "{}", SampleLoadError::ResetRequired),
        }
    }
}

impl Error for SampleDecodeError {}
