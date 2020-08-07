mod errors;
mod macros;
mod repeating_source;

use clap::{App, Arg, ArgMatches};
use errors::DjError;
use lazy_static::lazy_static;
use proptest::{collection::hash_map, prelude::*};
use rand::{seq::SliceRandom, Rng};
use regex::Regex;
use rodio::{decoder::Decoder, source::Zero, Sink, Source};
use std::{
	collections::{HashMap, HashSet},
	error::Error,
	fs,
	fs::File,
	io::{BufReader, Cursor, Read},
	path::Path,
	time::Duration,
};
use zip::ZipArchive;

lazy_static! {
	static ref REGEX_IS_LOOP: Regex = Regex::new(r"loop(\d+)?$").unwrap();
	static ref REGEX_IS_DEDICATED_TRANSITION: Regex = Regex::new(r"loop(\d+)-to-(\d+)").unwrap();
}

// Do NOT use mp3.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SongSegment {
	id: String,
	format: String,
	allowed_transitions: HashSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Song {
	id: String,
	segments: HashMap<String, SongSegment>,
	has_end: bool,
	has_global_ending: bool,
	has_multiple_loops: bool,
	has_dedicated_transitions: bool,
	is_archive: bool,
}

impl Song {
	fn read_segment(
		&self, segment: &SongSegment, songs_dir: &str,
	) -> Result<Decoder<BufReader<Cursor<Vec<u8>>>>, DjError> {
		let mut data = Vec::new();
		let file_name: String;
		if self.is_archive {
			file_name = format!("{}/{}.zip", songs_dir, self.id);
			let f = File::open(&file_name).unwrap();
			let mut arch = ZipArchive::new(f).unwrap();
			arch.by_name(&format!("{}.{}", segment.id, segment.format))
				.unwrap()
				.read_to_end(&mut data)
				.unwrap();
		}
		else {
			file_name = format!("{}/{}_{}.{}", songs_dir, self.id, segment.id, segment.format);
			File::open(&file_name).unwrap().read_to_end(&mut data).unwrap();
		};
		Decoder::new(BufReader::new(Cursor::new(data))).map_err(|_| DjError::UnrecognizedSongFormat(file_name))
	}

	fn make_plan<R: Rng + ?Sized>(&self, rng: &mut R) -> Vec<SongSegment> {
		let mut plan = Vec::<SongSegment>::new();
		plan.push(self.segments["start"].clone());

		let available_ends = self.segments.clone().into_iter().map(|(_, seg)| seg.id).filter(|seg| seg.ends_with("end")).collect::<Vec<String>>();

		loop {
			assert!(plan.len() <= 100, "plan too long");

			let current_segment = plan.last().unwrap();
			// It seems pretty shit to call collect and into_iter so many times. A lot less elegant than I would have hoped.
			let mut transitions = current_segment.allowed_transitions.clone().into_iter().collect::<Vec<String>>().into_iter();
			if self.has_end && plan.len() > 7 {
				let mut filtered = transitions.clone().filter(|next| next.ends_with("end")).collect::<Vec<String>>();
				if !filtered.is_empty() {
					transitions = filtered.into_iter();
				}
				else {
					filtered = transitions.clone().filter(|next| {
						for end in available_ends.clone() {
							if end.contains(next) {
								return true;
							}
						}
						false
					}).collect::<Vec<String>>();
					if !filtered.is_empty() {
						transitions = filtered.into_iter();
					}
				}
			}
			let allowed_transitions = transitions.collect::<Vec<String>>();
			match allowed_transitions.choose(rng) {
				Some(next_segment_id) => {
					plan.push(self.segments[next_segment_id].clone());
					if self.segments[next_segment_id].is_end() {
						return plan;
					}
					else if (self.has_end && plan.len() > 7) || (!self.has_end && plan.len() > 4) {
						if self.has_dedicated_transitions && self.segments[next_segment_id].is_dedicated_transition() {
							continue;
						}
						if self.has_end && self.has_global_ending && !self.segments[next_segment_id].is_end() {
							plan.push(self.segments["end"].clone());
							return plan;
						}
						else if !self.has_end {
							return plan;
						}
					}
				}
				None => {
					return plan;
				}
			}
		}
	}
}

impl SongSegment {
	fn is_loop(&self) -> bool {
		REGEX_IS_LOOP.is_match(&self.id) && !self.is_dedicated_transition()
	}

	fn is_dedicated_transition(&self) -> bool {
		REGEX_IS_DEDICATED_TRANSITION.is_match(&self.id)
	}

	fn is_end(&self) -> bool {
		(&self.id).ends_with("end")
	}
}

#[cfg(test)]
mod test_song_segments {
	use super::*;

	#[test]
	fn test_is_loop() {
		assert!(SongSegment {
			id: "loop".to_string(),
			format: "wav".to_string(),
			allowed_transitions: set!(),
		}
		.is_loop());

		assert!(SongSegment {
			id: "loop0".to_string(),
			format: "wav".to_string(),
			allowed_transitions: set!(),
		}
		.is_loop());

		assert!(SongSegment {
			id: "loop1".to_string(),
			format: "wav".to_string(),
			allowed_transitions: set!(),
		}
		.is_loop());

		assert!(!SongSegment {
			id: "loop0-to-1".to_string(),
			format: "wav".to_string(),
			allowed_transitions: set!(),
		}
		.is_loop());

		assert!(!SongSegment {
			id: "start".to_string(),
			format: "wav".to_string(),
			allowed_transitions: set!(),
		}
		.is_loop());

		assert!(!SongSegment {
			id: "end".to_string(),
			format: "wav".to_string(),
			allowed_transitions: set!(),
		}
		.is_loop());
	}

	#[test]
	fn test_is_dedicated_transition() {
		assert!(!SongSegment {
			id: "loop".to_string(),
			format: "wav".to_string(),
			allowed_transitions: set!(),
		}
		.is_dedicated_transition());

		assert!(!SongSegment {
			id: "loop0".to_string(),
			format: "wav".to_string(),
			allowed_transitions: set!(),
		}
		.is_dedicated_transition());

		assert!(SongSegment {
			id: "loop0-to-1".to_string(),
			format: "wav".to_string(),
			allowed_transitions: set!(),
		}
		.is_dedicated_transition());
	}

	#[test]
	fn test_is_end() {
		assert!(SongSegment {
			id: "end".to_string(),
			format: "wav".to_string(),
			allowed_transitions: set!(),
		}
		.is_end());

		assert!(SongSegment {
			id: "loop0-end".to_string(),
			format: "wav".to_string(),
			allowed_transitions: set!(),
		}
		.is_end());
	}
}

pub enum FileType {
	SegmentFormat,
	SongArchiveFormat,
}

pub fn detect_file_type(file_name: &str) -> Result<FileType, DjError> {
	let extension = file_name.split('.').last().unwrap();
	match extension {
		"wav" | "ogg" | "mp3" | "flac" => Ok(FileType::SegmentFormat),
		"zip" => Ok(FileType::SongArchiveFormat),
		_ => Err(DjError::UnrecognizedSongFormat(file_name.to_string())),
	}
}

pub fn get_song_name(file_name: &str) -> Result<String, DjError> {
	let segments = file_name.split('_').collect::<Vec<_>>();
	let name = segments[..(segments.len()) - 1].join("_");
	if name == "" {
		return Err(DjError::InvalidFileName(file_name.to_string()));
	}
	Ok(name)
}

pub fn parse_segment(file_name: &str) -> Result<SongSegment, DjError> {
	let name_split = file_name.split('_');
	let mut song_segment_split = name_split.last().unwrap().split('.');
	let song_segment_id = song_segment_split
		.next()
		.ok_or_else(|| DjError::InvalidFileName(file_name.to_string()))?;
	let song_segment_format = song_segment_split
		.next()
		.ok_or_else(|| DjError::UnrecognizedSongFormat(file_name.to_string()))?;
	let segment = SongSegment {
		id: song_segment_id.to_string(),
		format: song_segment_format.to_string(),
		allowed_transitions: HashSet::new(),
	};

	Ok(segment)
}

pub fn initialize_songs<P: AsRef<Path>>(paths: &[P]) -> Result<HashMap<String, Song>, DjError> {
	let mut songs = HashMap::new();
	for path in paths {
		let path = path.as_ref();
		let file_name = match path.file_name().unwrap().to_str().ok_or(DjError::PathNotValidUnicode) {
			Ok(val) => val,
			Err(e) => {
				println!("Warning: {}. Dropping.", e);
				continue;
			}
		};
		let file_type = match detect_file_type(file_name) {
			Ok(val) => val,
			Err(e) => {
				println!("Warning: {}. Dropping.", e);
				continue;
			}
		};

		match file_type {
			FileType::SegmentFormat => {
				let song_id = get_song_name(file_name)?;
				let segment = parse_segment(file_name)?;
				let song = songs.entry(song_id.to_string()).or_insert(Song {
					id: song_id,
					segments: HashMap::new(),
					has_end: false,
					has_global_ending: false,
					has_multiple_loops: false,
					has_dedicated_transitions: false,
					is_archive: false,
				});
				if !song.has_end && segment.id.ends_with("end") {
					song.has_end = true;
					song.has_global_ending = segment.id == "end";
				}
				if segment.id != "loop" && REGEX_IS_LOOP.is_match(&segment.id) {
					song.has_multiple_loops = true;
				}
				if REGEX_IS_DEDICATED_TRANSITION.is_match(&segment.id) {
					song.has_dedicated_transitions = true;
				}
				if song.segments.contains_key(&segment.id) {
					// Having multiple files with the same ID is ambiguous.
					return Err(DjError::MultipleSegmentsWithSameId(song.id.to_string(), segment.id));
				}
				song.segments.entry(segment.id.to_string()).or_insert(segment);
			}
			FileType::SongArchiveFormat => {
				let archive = ZipArchive::new(File::open(path).unwrap()).unwrap();
				let song_id = file_name
					.split('.')
					.next()
					.ok_or(DjError::InvalidFileName(file_name.to_string()))?
					.to_string();
				println!("Encountered Archive {}.", song_id);
				let song = songs.entry(song_id.to_string()).or_insert(Song {
					id: song_id,
					segments: HashMap::new(),
					has_end: false,
					has_global_ending: false,
					has_multiple_loops: false,
					has_dedicated_transitions: false,
					is_archive: true,
				});
				for segment_path in archive.file_names() {
					let segment = parse_segment(&segment_path)?;
					if !song.has_end && segment.id.ends_with("end") {
						song.has_end = true;
						song.has_global_ending = segment.id == "end";
					}
					if segment.id != "loop" && REGEX_IS_LOOP.is_match(&segment.id) {
						song.has_multiple_loops = true;
					}
					if REGEX_IS_DEDICATED_TRANSITION.is_match(&segment.id) {
						song.has_dedicated_transitions = true;
					}
					if song.segments.contains_key(&segment.id) {
						// Having multiple files with the same ID is ambiguous.
						return Err(DjError::MultipleSegmentsWithSameId(song.id.to_string(), segment.id));
					}
					song.segments.entry(segment.id.to_string()).or_insert(segment);
				}
			}
		}
	}

	Ok(songs)
}

pub fn initialize_transitions(songs: &mut HashMap<String, Song>) {
	for song in songs.values_mut() {
		let clone_segments = &song.segments.clone();

		for song_segment in song.segments.values_mut() {
			if song_segment.is_dedicated_transition() {
				let loop_nums = REGEX_IS_DEDICATED_TRANSITION.captures(&song_segment.id).unwrap();
				let loop_to = loop_nums.get(2).unwrap();
				song_segment
					.allowed_transitions
					.insert(format!("loop{}", loop_to.as_str()));
			}
			else if song.has_multiple_loops && song_segment.is_loop() {
				if song.has_end && song.has_global_ending {
					song_segment.allowed_transitions.insert("end".to_string());
				}

				for seg in clone_segments.values() {
					if song.has_dedicated_transitions && song_segment.is_loop() && seg.is_dedicated_transition() {
						if seg.id.starts_with(&format!("{}-to", &song_segment.id)) {
							song_segment.allowed_transitions.insert(seg.id.clone());
						}
					}
					else if !song.has_global_ending && seg.id == format!("{}-end", &song_segment.id) {
						song_segment.allowed_transitions.insert(seg.id.clone());
					}
					else if !song.has_dedicated_transitions
						&& song.has_multiple_loops
						&& song_segment.is_loop()
						&& seg.is_loop()
					{
						if seg.id == song_segment.id {
							continue;
						}
						song_segment.allowed_transitions.insert(seg.id.clone());
					}
				}
			}
			else {
				match song_segment.id.as_str() {
					"start" => {
						song_segment.allowed_transitions.insert(if song.has_multiple_loops {
							"loop0".to_string()
						}
						else {
							"loop".to_string()
						});
					}
					"loop" => {
						if song.has_end && song.has_global_ending {
							song_segment.allowed_transitions.insert("end".to_string());
						}
					}
					_ => {}
				}
			}
		}
	}
}

prop_compose! {
	/// Generates a random valid song segment. May not be valid when put into an actual Song.
	fn song_segment_strategy()
		((id,segment_format) in (r"(start|end|loop(\d(-(to-\d|end)))?)",r"(wav|mp3|ogg|flac)")) -> SongSegment {
		SongSegment {
			id,
			format: segment_format,
			allowed_transitions: set!(),
		}
	}
}

prop_compose! {
	/// Generates a valid Song with kinda random dedicated transitions
	fn song_strategy(max_loop_count: u32, has_end: bool)
		(id in "[a-z0-9-]*", loop_count in 1..=max_loop_count, loop_transitions in 0..max_loop_count, has_global_ending: bool) -> Song {
		let mut segment_vec: Vec<SongSegment> = vec![];
		segment_vec.push(SongSegment {
			id: "start".to_string(),
			format:"ogg".to_string(),
			allowed_transitions: set!(),

		});

		match loop_count {
			1 => {
				segment_vec.push(SongSegment {
					id: "loop".to_string(),
					format:"ogg".to_string(),
					allowed_transitions: set!(),
				});
			},
			_ => {
				for i in 0..loop_count {
					segment_vec.push(SongSegment {
						id: format!("loop{}", i),
						format:"ogg".to_string(),
						allowed_transitions: set!(),
					});
				}
			}
		}

		if loop_transitions > 0 {
			let mut transition_count = 0;
			'outer: for from in 0..loop_count {
				for to in 0..loop_count {
					if from == to {
						continue
					}
					segment_vec.push(SongSegment {
						id: format!("loop{}-to-{}", from, to),
						format:"ogg".to_string(),
						allowed_transitions: set!(),
					});
					transition_count += 1;
					if transition_count >= loop_transitions {
						break 'outer;
					}
				}
			}
		}

		if has_end {
			segment_vec.push(SongSegment {
				id: if !has_global_ending && loop_count > 1 { format!("loop{}-end", loop_count - 1) } else { "end".to_string() },
				format:"ogg".to_string(),
				allowed_transitions: set!(),
			});
		}

		let mut segments: HashMap<String, SongSegment> = HashMap::new();
		for seg in segment_vec {
			segments.insert(seg.id.to_string(), seg.clone());
		}
		Song {
			id,
			segments,
			has_end,
			has_global_ending: has_end && (has_global_ending || loop_count == 1),
			has_multiple_loops: loop_count > 1,
			has_dedicated_transitions: loop_transitions > 0,
			is_archive: false,
		}
	}
}

prop_compose! {
	fn song_with_transitions_strategy(loop_count: usize, has_end: bool)
		(id in "[a-z0-9-_]*", transitions in hash_map(0..loop_count, 0..loop_count, 1..loop_count).prop_filter("must not transition into same loop".to_owned(), |m| {
			for (from, to) in m {
				if from == to {
					return false;
				}
			}
			true
		})) -> Song {
			let mut segment_vec: Vec<SongSegment> = vec![];
			segment_vec.push(SongSegment {
				id: "start".to_string(),
				format:"ogg".to_string(),
				allowed_transitions: set!(),
			});

			match loop_count {
				1 => {
					segment_vec.push(SongSegment {
						id: "loop".to_string(),
						format:"ogg".to_string(),
						allowed_transitions: set!(),
					});
				},
				_ => {
					for i in 0..loop_count {
						segment_vec.push(SongSegment {
							id: format!("loop{}", i),
							format:"ogg".to_string(),
							allowed_transitions: set!(),
						});
					}
				}
			}

			for (from, to) in transitions {
				if from == to {
					continue;
				}
				segment_vec.push(SongSegment {
					id: format!("loop{}-to-{}", from, to),
					format:"ogg".to_string(),
					allowed_transitions: set!(),
				});
			}

			if has_end {
				segment_vec.push(SongSegment {
					id: "end".to_string(),
					format:"ogg".to_string(),
					allowed_transitions: set!(),
				});
			}

			let mut segments: HashMap<String, SongSegment> = HashMap::new();
			for seg in segment_vec {
				segments.insert(seg.id.to_string(), seg.clone());
			}
			Song {
				id,
				segments,
				has_end,
				has_global_ending: has_end,
				has_multiple_loops: loop_count > 1,
				has_dedicated_transitions: true,
				is_archive: false,
			}
	}
}

#[cfg(test)]
mod test_song_parsing {
	use super::*;

	#[test]
	fn test_song_archive() {
		let paths = fs::read_dir("test-data/test_song_archive").expect("Unable to list files in songs-dir.");
		let path_strings = paths
			.map(|p| p.unwrap().path().display().to_string())
			.collect::<Vec<_>>();
		let songs = initialize_songs(&path_strings).unwrap();
		assert_eq!(
			songs["song_archive"],
			Song {
				id: "song_archive".to_string(),
				segments: map!(
				"start".to_string() => SongSegment {
					id: "start".to_string(),
					format:"wav".to_string(),
					allowed_transitions: HashSet::new(),

				},
				"loop".to_string() => SongSegment {
					id: "loop".to_string(),
					format:"wav".to_string(),
					allowed_transitions: HashSet::new(),

				}),
				has_end: false,
				has_global_ending: false,
				has_multiple_loops: false,
				has_dedicated_transitions: false,
				is_archive: true,
			}
		)
	}

	#[test]
	fn test_initialize_songs() {
		let paths = [
			"songs/song_1_start.ogg",
			"songs/song_1_loop.ogg",
			"songs/song_1_end.ogg",
			"songs/song_2_start.ogg",
			"songs/song_2_loop0.ogg",
			"songs/song_2_loop1.ogg",
			"songs/song_2_end.ogg",
			"songs/y3_start.ogg",
			"songs/y3_loop0.ogg",
			"songs/y3_loop0-to-1.ogg",
			"songs/y3_loop1.ogg",
			"songs/y3_end.ogg",
			"songs/song_wav_start.wav",
			"songs/song_wav_loop.wav",
			"songs/song_wav_end.wav",
		];
		let songs = initialize_songs(&paths).unwrap();
		assert_eq!(
			songs["song_1"],
			Song {
				id: "song_1".to_string(),
				segments: map!(
					"start".to_string() => SongSegment {
						id: "start".to_string(),
						format:"ogg".to_string(),
						allowed_transitions: HashSet::new(),
					},
					"loop".to_string() => SongSegment {
						id: "loop".to_string(),
						format:"ogg".to_string(),
						allowed_transitions: HashSet::new(),
					},
					"end".to_string() => SongSegment {
						id: "end".to_string(),
						format:"ogg".to_string(),
						allowed_transitions: HashSet::new(),
					}
				),
				has_end: true,
				has_global_ending: true,
				has_multiple_loops: false,
				has_dedicated_transitions: false,
				is_archive: false,
			}
		);
		assert_eq!(
			songs["song_2"],
			Song {
				id: "song_2".to_string(),
				segments: map!(
					"start".to_string() => SongSegment {
						id: "start".to_string(),
						format:"ogg".to_string(),
						allowed_transitions: HashSet::new(),
					},
					"loop0".to_string() => SongSegment {
						id: "loop0".to_string(),
						format:"ogg".to_string(),
						allowed_transitions: HashSet::new(),
					},
					"loop1".to_string() => SongSegment {
						id: "loop1".to_string(),
						format:"ogg".to_string(),
						allowed_transitions: HashSet::new(),
					},
					"end".to_string() => SongSegment {
						id: "end".to_string(),
						format:"ogg".to_string(),
						allowed_transitions: HashSet::new(),
					}
				),
				has_end: true,
				has_global_ending: true,
				has_multiple_loops: true,
				has_dedicated_transitions: false,
				is_archive: false,
			}
		);
		assert_eq!(
			songs["y3"],
			Song {
				id: "y3".to_string(),
				segments: map!(
					"start".to_string() => SongSegment {
						id: "start".to_string(),
						format:"ogg".to_string(),
						allowed_transitions: HashSet::new(),
					},
					"loop0".to_string() => SongSegment {
						id: "loop0".to_string(),
						format:"ogg".to_string(),
						allowed_transitions: HashSet::new(),
					},
					"loop0-to-1".to_string() => SongSegment {
						id: "loop0-to-1".to_string(),
						format:"ogg".to_string(),
						allowed_transitions: HashSet::new(),
					},
					"loop1".to_string() => SongSegment {
						id: "loop1".to_string(),
						format:"ogg".to_string(),
						allowed_transitions: HashSet::new(),
					},
					"end".to_string() => SongSegment {
						id: "end".to_string(),
						format:"ogg".to_string(),
						allowed_transitions: HashSet::new(),
					}
				),
				has_end: true,
				has_global_ending: true,
				has_multiple_loops: true,
				has_dedicated_transitions: true,
				is_archive: false,
			}
		);
		assert_eq!(
			songs["song_wav"],
			Song {
				id: "song_wav".to_string(),
				segments: map!(
					"start".to_string() => SongSegment {
						id: "start".to_string(),
						format:"wav".to_string(),
						allowed_transitions: HashSet::new(),
					},
					"loop".to_string() => SongSegment {
						id: "loop".to_string(),
						format:"wav".to_string(),
						allowed_transitions: HashSet::new(),
					},
					"end".to_string() => SongSegment {
						id: "end".to_string(),
						format:"wav".to_string(),
						allowed_transitions: HashSet::new(),
					}
				),
				has_end: true,
				has_global_ending: true,
				has_multiple_loops: false,
				has_dedicated_transitions: false,
				is_archive: false,
			}
		);
	}

	#[test]
	#[should_panic(
		expected = "called `Result::unwrap()` on an `Err` value: MultipleSegmentsWithSameId(\"song_format\", \"loop\")"
	)]
	fn test_detect_duplicate_segment() {
		let paths = [
			"song_format_start.wav",
			"song_format_end.wav",
			"song_format_loop.wav",
			"song_format_loop.ogg",
		];
		initialize_songs(&paths).unwrap();
	}

	#[test]
	fn test_initialize_transitions() {
		let mut songs = map! {
			"1".to_string() => Song {
				id: "1".to_string(),
				segments: map!(
					"start".to_string() => SongSegment {
						id: "start".to_string(),
						format:"ogg".to_string(),
						allowed_transitions: HashSet::new(),
					},
					"loop".to_string() => SongSegment {
						id: "loop".to_string(),
						format:"ogg".to_string(),
						allowed_transitions: HashSet::new(),
					},
					"end".to_string() => SongSegment {
						id: "end".to_string(),
						format:"ogg".to_string(),
						allowed_transitions: HashSet::new(),
					}
				),
				has_end: true,
				has_global_ending: true,
				has_multiple_loops: false,
				has_dedicated_transitions: false,
				is_archive: false,
			},
			"2".to_string() => Song {
				id: "2".to_string(),
				segments: map!(
					"start".to_string() => SongSegment {
						id: "start".to_string(),
						format:"ogg".to_string(),
						allowed_transitions: HashSet::new(),
					},
					"loop0".to_string() => SongSegment {
						id: "loop0".to_string(),
						format:"ogg".to_string(),
						allowed_transitions: HashSet::new(),
					},
					"loop1".to_string() => SongSegment {
						id: "loop1".to_string(),
						format:"ogg".to_string(),
						allowed_transitions: HashSet::new(),
					},
					"end".to_string() => SongSegment {
						id: "end".to_string(),
						format:"ogg".to_string(),
						allowed_transitions: HashSet::new(),
					}
				),
				has_end: true,
				has_global_ending: true,
				has_multiple_loops: true,
				has_dedicated_transitions: false,
				is_archive: false,
			},
			"3".to_string() => Song {
				id: "3".to_string(),
				segments: map!(
					"start".to_string() => SongSegment {
						id: "start".to_string(),
						format:"ogg".to_string(),
						allowed_transitions: HashSet::new(),
					},
					"loop0".to_string() => SongSegment {
						id: "loop0".to_string(),
						format:"ogg".to_string(),
						allowed_transitions: HashSet::new(),
					},
					"loop0-to-1".to_string() => SongSegment {
						id: "loop0-to-1".to_string(),
						format:"ogg".to_string(),
						allowed_transitions: HashSet::new(),
					},
					"loop1".to_string() => SongSegment {
						id: "loop1".to_string(),
						format:"ogg".to_string(),
						allowed_transitions: HashSet::new(),
					},
					"end".to_string() => SongSegment {
						id: "end".to_string(),
						format:"ogg".to_string(),
						allowed_transitions: HashSet::new(),
					}
				),
				has_end: true,
				has_global_ending: true,
				has_multiple_loops: true,
				has_dedicated_transitions: true,
				is_archive: false,
			}
		};

		initialize_transitions(&mut songs);

		assert_eq!(
			songs["1"].segments["start"].allowed_transitions,
			set!["loop".to_string()]
		);
		assert_eq!(songs["1"].segments["loop"].allowed_transitions, set!["end".to_string()]);
		assert_eq!(songs["1"].segments["end"].allowed_transitions, HashSet::new());

		assert_eq!(
			songs["2"].segments["start"].allowed_transitions,
			set!["loop0".to_string()]
		);
		assert_eq!(
			songs["2"].segments["loop0"].allowed_transitions,
			set!["loop1".to_string(), "end".to_string()]
		);
		assert_eq!(
			songs["2"].segments["loop1"].allowed_transitions,
			set!["loop0".to_string(), "end".to_string()]
		);
		assert_eq!(songs["2"].segments["end"].allowed_transitions, HashSet::new());

		assert_eq!(
			songs["3"].segments["start"].allowed_transitions,
			set!["loop0".to_string()]
		);
		assert_eq!(
			songs["3"].segments["loop0"].allowed_transitions,
			set!["loop0-to-1".to_string(), "end".to_string()]
		);
		assert_eq!(
			songs["3"].segments["loop0-to-1"].allowed_transitions,
			set!["loop1".to_string()]
		);
		assert_eq!(
			songs["3"].segments["loop1"].allowed_transitions,
			set!["end".to_string()]
		);
		assert_eq!(songs["3"].segments["end"].allowed_transitions, HashSet::new());
	}

	proptest! {
		#[test]
		fn prop_multiloop_song_should_not_contain_references_to_loop(song_id in "[a-z0-9]+", loop_count in 2..10) {
			let mut paths: Vec<String> = vec![format!("songs/{}_start.ogg", song_id)];
			for i in 0..loop_count {
				paths.push(format!("songs/{}_loop{}.ogg", song_id, i))
			}

			let mut songs: HashMap<String, Song> = initialize_songs(&paths).unwrap();
			initialize_transitions(&mut songs);
			for song in songs.values() {
				for segment in song.segments.values() {
					prop_assert_ne!(&segment.id, "loop");
					for transition in &segment.allowed_transitions {
						prop_assert_ne!(transition, &"loop".to_string());
					}
				}
			}
		}

		#[test]
		fn prop_should_generate_transitions(song in song_strategy(12, true)) {
			let song_id = song.id.to_string();
			let mut songs: HashMap<String, Song> = map!(song_id.clone() => song);
			initialize_transitions(&mut songs);
			prop_assert!(!songs[&song_id].segments["start"].allowed_transitions.is_empty());
			if songs[&song_id].has_multiple_loops {
				prop_assert!(!songs[&song_id].segments["loop0"].allowed_transitions.is_empty());
			}
			else {
				prop_assert!(!songs[&song_id].segments["loop"].allowed_transitions.is_empty());
			}
			for segment in songs[&song_id].segments.values() {
				for transition in &segment.allowed_transitions {
					if transition.to_string().ends_with("end") {
						prop_assert!(songs[&song_id].segments[transition].allowed_transitions.is_empty());
					}
				}
			}
		}

		#[test]
		fn prop_should_not_allow_transitions_to_start_segment(song in song_strategy(12, false)) {
			let song_id = song.id.to_string();
			let mut songs: HashMap<String, Song> = map!(song_id => song);
			initialize_transitions(&mut songs);
			for song in songs.values() {
				for segment in song.segments.values() {
					for transition in &segment.allowed_transitions {
						prop_assert_ne!(transition, &"start".to_string());
					}
				}
			}
		}
	}
}

#[cfg(test)]
mod test_song_planning {
	use super::*;

	// #[test]
	// / Yeah this test is extremely brittle. This just makes sure we test this specific scenario at least once.
	// / Property tests will guarentee that something will fail if it should.
	// fn test_song_must_end_with_loop_specific_end() {
	// 	let mut rng = rand::thread_rng();
	// 	let mut songs = map! {
	// 		"1".to_string() => Song {
	// 			id: "1".to_string(),
	// 			segments: map!(
	// 				"start".to_string() => SongSegment {
	// 					id: "start".to_string(),
	// 					format:"ogg".to_string(),
	// 					allowed_transitions: HashSet::new(),
	// 				},
	// 				"loop0".to_string() => SongSegment {
	// 					id: "loop0".to_string(),
	// 					format:"ogg".to_string(),
	// 					allowed_transitions: HashSet::new(),
	// 				},
	// 				"loop1".to_string() => SongSegment {
	// 					id: "loop1".to_string(),
	// 					format:"ogg".to_string(),
	// 					allowed_transitions: HashSet::new(),
	// 				},
	// 				"loop2".to_string() => SongSegment {
	// 					id: "loop2".to_string(),
	// 					format:"ogg".to_string(),
	// 					allowed_transitions: HashSet::new(),
	// 				},
	// 				"loop3".to_string() => SongSegment {
	// 					id: "loop3".to_string(),
	// 					format:"ogg".to_string(),
	// 					allowed_transitions: HashSet::new(),
	// 				},
	// 				"loop4".to_string() => SongSegment {
	// 					id: "loop4".to_string(),
	// 					format:"ogg".to_string(),
	// 					allowed_transitions: HashSet::new(),
	// 				},
	// 				"loop5".to_string() => SongSegment {
	// 					id: "loop5".to_string(),
	// 					format:"ogg".to_string(),
	// 					allowed_transitions: HashSet::new(),
	// 				},
	// 				"loop6".to_string() => SongSegment {
	// 					id: "loop6".to_string(),
	// 					format:"ogg".to_string(),
	// 					allowed_transitions: HashSet::new(),
	// 				},
	// 				"loop7".to_string() => SongSegment {
	// 					id: "loop7".to_string(),
	// 					format:"ogg".to_string(),
	// 					allowed_transitions: HashSet::new(),
	// 				},
	// 				"loop1-end".to_string() => SongSegment {
	// 					id: "loop1-end".to_string(),
	// 					format:"ogg".to_string(),
	// 					allowed_transitions: HashSet::new(),
	// 				}
	// 			),
	// 			has_end: true,
	// 			has_global_ending: false,
	// 			has_multiple_loops: true,
	// 			has_dedicated_transitions: false
	// 		},
	// 	};
	// 	initialize_transitions(&mut songs);
	// 	let plan = songs["1"].make_plan(&mut rng);
	// 	assert_eq!(&plan.last().unwrap().id, &"loop1-end".to_string())
	// }

	proptest! {
		#[test]
		fn prop_plan_should_end_with_end(song in song_strategy(12, true)) {
			let mut rng = rand::thread_rng();
			let song_id = song.id.to_string();
			let mut songs: HashMap<String, Song> = map!(song_id.clone() => song);
			initialize_transitions(&mut songs);
			let plan = songs[&song_id].make_plan(&mut rng);
			prop_assert!(&plan.last().unwrap().id.ends_with("end"));
		}

		#[test]
		fn prop_plan_should_not_end_with_end(song in song_strategy(12, false)) {
			let mut rng = rand::thread_rng();
			let song_id = song.id.to_string();
			let mut songs: HashMap<String, Song> = map!(song_id.clone() => song);
			initialize_transitions(&mut songs);
			let plan = songs[&song_id].make_plan(&mut rng);
			prop_assert_ne!(&plan.last().unwrap().id, &"end".to_string())
		}

		#[test]
		fn prop_plan_should_always_start_with_start(song in song_strategy(12, true)) {
			let mut rng = rand::thread_rng();
			let song_id = song.id.to_string();
			let mut songs: HashMap<String, Song> = map!(song_id.clone() => song);
			initialize_transitions(&mut songs);
			let plan = songs[&song_id].make_plan(&mut rng);
			prop_assert_eq!(&plan.first().unwrap().id, &"start".to_string())
		}

		#[test]
		fn prop_plan_should_always_be_at_least_3(song in song_strategy(12, true)) {
			let mut rng = rand::thread_rng();
			let song_id = song.id.to_string();
			let mut songs: HashMap<String, Song> = map!(song_id.clone() => song);
			initialize_transitions(&mut songs);
			let plan = songs[&song_id].make_plan(&mut rng);
			prop_assert!(plan.len() >= 3)
		}

		#[test]
		fn prop_plan_should_always_be_at_least_2(song in song_strategy(12, false)) {
			let mut rng = rand::thread_rng();
			let song_id = song.id.to_string();
			let mut songs: HashMap<String, Song> = map!(song_id.clone() => song);
			initialize_transitions(&mut songs);
			let plan = songs[&song_id].make_plan(&mut rng);
			prop_assert!(plan.len() >= 2);
		}

		#[test]
		fn prop_plan_should_never_end_with_transition(song in song_with_transitions_strategy(12, false)) {
			let mut rng = rand::thread_rng();
			let song_id = song.id.to_string();
			let mut songs: HashMap<String, Song> = map!(song_id.clone() => song);
			initialize_transitions(&mut songs);
			let plan = songs[&song_id].make_plan(&mut rng);
			prop_assert!(!plan.last().unwrap().is_dedicated_transition())
		}
	}
}

fn main() {
	let args = App::new("stream-autodj")
		.version("0.1.0")
		.author("Carson McManus <@dyc3>")
		.about("Plays music loops for random durations in random order.")
		.arg(Arg::with_name("songs-dir")
			.short("s")
			.long("songs-dir")
			.value_name("SONGS_DIR")
			.help("Sets a custom config file")
			.default_value("./songs")
			.takes_value(true))
		.arg(Arg::with_name("OVERRIDE")
			.help("Overrides song selection with this song.")
			.required(false)
			.index(1))
		.arg(Arg::with_name("max-repeats")
			.long("max-repeats")
			.default_value("13")
			.takes_value(true)
			.help("Sets the max number of loop repeats"))
		.arg(Arg::with_name("debug-wait-each-segment")
			.long("debug-wait-each-segment")
			.help("Force the program to wait for the sink to empty after each source is added to the sink, and print the name of the segments as they get queued up. Will cause small pauses between song segments as a result."))
		.get_matches();

	if let Err(e) = run(args) {
		eprintln!("Error: {}.", e);
		std::process::exit(1);
	}
}

fn run(args: ArgMatches) -> Result<(), Box<dyn Error>> {
	let songs_dir = args.value_of("songs-dir").unwrap();
	let paths = fs::read_dir(songs_dir)?;
	let path_strings = paths
		.map(|p| p.unwrap().path().display().to_string())
		.collect::<Vec<_>>();

	let mut songs = initialize_songs(&path_strings)?;
	initialize_transitions(&mut songs);
	println!("Found {} songs.", songs.len());

	let mut rng = rand::thread_rng();
	let device = rodio::default_output_device().ok_or(DjError::NoOutputDeviceAvailable)?;
	let sink = Sink::new(&device);

	let max_repeats: u32 = args
		.value_of("max-repeats")
		.unwrap()
		.parse()
		.map_err(|_| DjError::MaxRepeatsInvalidValue)?;

	loop {
		let current_song_id = args
			.value_of("OVERRIDE")
			.unwrap_or_else(|| songs.keys().collect::<Vec<_>>().choose(&mut rng).unwrap());
		println!("Now playing: {}.", current_song_id);
		let current_song = &songs[current_song_id];

		let plan = current_song.make_plan(&mut rng);
		println!(
			"Plan: {:?}.",
			plan.clone().iter().map(|x| x.id.clone()).collect::<Vec<_>>()
		);

		for segment in &plan {
			let source = current_song.read_segment(segment, songs_dir)?;
			if args.is_present("debug-wait-each-segment") {
				println!("Playing segment: {}.", segment.id);
			}
			if segment.is_loop() && !segment.is_dedicated_transition() {
				let repeat_counts: u32 = rng.gen_range(5, max_repeats);
				println!("Repeating {} {} times.", segment.id, repeat_counts);
				sink.append(repeating_source::repeat_with_count(source, repeat_counts));
			}
			else {
				sink.append(source);
			}
			if args.is_present("debug-wait-each-segment") {
				sink.sleep_until_end();
			}
		}
		if !current_song.has_end {
			let segment = plan.last().unwrap();
			let source_end = current_song.read_segment(segment, songs_dir)?;
			let empty_source: Zero<f32> = Zero::new(source_end.channels(), source_end.sample_rate());
			sink.append(source_end.take_crossfade_with(empty_source, Duration::from_secs(8)));
		}

		sink.sleep_until_end();
	}
}
