use std::{
	fmt,
	error::Error,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppError {
	NoOutputDeviceAvailable,
	MaxRepeatsInvalidValue,
	UnrecognizedSongFormat(String),
	PathNotValidUnicode,
	InvalidFileName(String),
	MultipleSegmentsWithSameId(String, String),
}

impl fmt::Display for AppError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			AppError::NoOutputDeviceAvailable => write!(f, "no output device is available"),
			AppError::MaxRepeatsInvalidValue => write!(f, "invalid value for max-repeats"),
			AppError::UnrecognizedSongFormat(file_name) => write!(f, "'{}' - unrecognized song format. Only wav, flac, ogg, mp3 and zip are supported", file_name),
			AppError::PathNotValidUnicode => write!(f, "one or several files have non Unicode characters in their path or filename"),
			AppError::InvalidFileName(file_name) => write!(f, "'{}' - invalid file name. Files should be named in this format: song_1_start", file_name),
			AppError::MultipleSegmentsWithSameId(song_id, segment_id) => write!(f, "found multiple segments with same ID: Song: {} Segment: {}", song_id, segment_id),
		}
	}
}

impl Error for AppError {}
