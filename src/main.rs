mod macros;

use std::fs::File;
use std::io::BufReader;
use std::collections::{HashSet, HashMap};
use std::fs;
use std::path::Path;
use rodio::Sink;
use rodio::decoder::Decoder;
use rodio::{source::Zero, Source};
use rand::Rng;
use rand::seq::SliceRandom;
use std::time::Duration;
use regex::Regex;
use lazy_static::lazy_static;
use proptest::prelude::*;

lazy_static! {
	static ref REGEX_IS_LOOP: Regex = Regex::new(r"loop(\d+)?").unwrap();
	static ref REGEX_IS_DEDICATED_TRANSITION: Regex = Regex::new(r"loop(\d+)-to-(\d+)").unwrap();
}

// Do NOT use mp3.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SongSegment {
	id: String,
	allowed_transitions: HashSet<String>
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Song {
	id: String,
	segments: HashMap<String, SongSegment>,
	has_end: bool,
	has_multiple_loops: bool,
	has_dedicated_transitions: bool,
}

impl Song {
	fn read_segment(&self, segment: &str) -> Decoder<BufReader<File>> {
		let file = File::open(format!("songs/song_{}_{}.ogg", self.id, segment)).unwrap();
		Decoder::new(BufReader::new(file)).unwrap()
	}

	fn make_plan(&self, rng: &mut rand::rngs::ThreadRng) -> Vec<SongSegment> {
		let mut plan = Vec::<SongSegment>::new();
		plan.push(self.segments["start"].clone());

		loop {
			if plan.len() > 100 {
				panic!("plan too long");
			}

			let current_segment = plan.last().unwrap();
			let allowed_transitions = current_segment.allowed_transitions.clone().into_iter().collect::<Vec<_>>();
			match allowed_transitions.choose(rng) {
				Some(next_segment_id) => {
					plan.push(self.segments[next_segment_id].clone());
					if self.has_end && next_segment_id.ends_with("end") {
						return plan;
					}
					else if plan.len() > 7 {
						if self.has_end && !next_segment_id.ends_with("end") {
							plan.push(self.segments["end"].clone());
						}
						return plan;
					}
				},
				None => {
					return plan;
				}
			}
		}
	}
}

pub fn initialize_songs<P: AsRef<Path>>(paths: &[P]) -> HashMap<String, Song> {
	let mut songs = HashMap::new();
	for path in paths {
		let path = path.as_ref();
		let file_name = path.file_name().unwrap().to_str().unwrap().to_string();
		let name_split = file_name.split('_').collect::<Vec<_>>();
		let song_id = name_split[1].to_string();
		let song_segment_id = name_split[2].to_string().split('.').collect::<Vec<_>>()[0].to_string();
		let song = songs.entry(song_id.clone()).or_insert(Song {
			id: song_id,
			segments: HashMap::<String, SongSegment>::new(),
			has_end: false,
			has_multiple_loops: false,
			has_dedicated_transitions: false
		});
		if !song.has_end && song_segment_id == "end"{
			song.has_end = true;
		}
		if !song.has_multiple_loops && song_segment_id != "loop" && REGEX_IS_LOOP.is_match(&song_segment_id) {
			song.has_multiple_loops = true;
		}
		if !song.has_dedicated_transitions && REGEX_IS_DEDICATED_TRANSITION.is_match(&song_segment_id) {
			song.has_dedicated_transitions = true;
		}
		song.segments.entry(song_segment_id.clone()).or_insert(SongSegment {
			id: song_segment_id,
			allowed_transitions: HashSet::<String>::new(),
		});
	}

	songs
}

pub fn initialize_transitions(songs: &mut HashMap<String, Song>) {
	for song in songs.values_mut() {
		let clone_segments = &song.segments.clone();

		for song_segment in song.segments.values_mut() {
			if REGEX_IS_DEDICATED_TRANSITION.is_match(&song_segment.id) {
				let loop_nums = REGEX_IS_DEDICATED_TRANSITION.captures(&song_segment.id).unwrap();
				let loop_to = loop_nums.get(2).unwrap();
				song_segment.allowed_transitions.insert(format!("loop{}", loop_to.as_str()));

				// can't do this because double borrow
				// let from_seg = song.segments.get_mut(&format!("loop{}", loop_from.as_str())).unwrap();
				// from_seg.allowed_transitions.insert(song_segment.id);
			}
			else if song.has_multiple_loops && REGEX_IS_LOOP.is_match(&song_segment.id) {
				if song.has_end {
					song_segment.allowed_transitions.insert("end".to_string());
				}

				for seg in clone_segments.values() {
					if song.has_dedicated_transitions && REGEX_IS_DEDICATED_TRANSITION.is_match(&seg.id) {
						if seg.id.starts_with(&format!("{}-to", &song_segment.id)) {
							song_segment.allowed_transitions.insert(seg.id.clone());
						}
					}
					else if !song.has_dedicated_transitions && song.has_multiple_loops && REGEX_IS_LOOP.is_match(&seg.id) {
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
						song_segment.allowed_transitions.insert(
							if song.has_multiple_loops {
								"loop0".to_string()
							}
							else {
								"loop".to_string()
							}
						);
					},
					"loop" => {
						if song.has_end {
							song_segment.allowed_transitions.insert("end".to_string());
						}
					},
					_ => {}
				}
			}
		}
	}
}

prop_compose! {
	/// Generates a random valid song segment. May not be valid when put into an actual Song.
	fn song_segment_strategy()
		(id in r"(start|end|loop(\d(-to-\d))?)") -> SongSegment {
		SongSegment {
			id,
			allowed_transitions: set!()
		}
	}
}

prop_compose! {
	/// Generates a valid Song without dedicated transitions
	fn song_strategy(max_loop_count: u32, has_end: bool)
		(id in "[a-z0-9-]*", loop_count in 1..=max_loop_count) -> Song {
		let mut segment_vec: Vec<SongSegment> = vec![];
		segment_vec.push(SongSegment {
			id: "start".to_string(),
			allowed_transitions: set!()
		});

		match loop_count {
			1 => {
				segment_vec.push(SongSegment {
					id: "loop".to_string(),
					allowed_transitions: set!()
				});
			},
			_ => {
				for i in 0..loop_count {
					segment_vec.push(SongSegment {
						id: format!("loop{}", i),
						allowed_transitions: set!()
					});
				}
			}
		}

		if has_end {
			segment_vec.push(SongSegment {
				id: "end".to_string(),
				allowed_transitions: set!()
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
			has_multiple_loops: loop_count > 1,
			has_dedicated_transitions: false,
		}
	}
}

#[cfg(test)]
mod test_song_parsing {
	use super::*;

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
			"songs/song_3_start.ogg",
			"songs/song_3_loop0.ogg",
			"songs/song_3_loop0-to-1.ogg",
			"songs/song_3_loop1.ogg",
			"songs/song_3_end.ogg"
		];
		let songs = initialize_songs(&paths);
		assert_eq!(songs["1"], Song {
			id: "1".to_string(),
			segments: map!(
				"start".to_string() => SongSegment {
					id: "start".to_string(),
					allowed_transitions: HashSet::new(),
				},
				"loop".to_string() => SongSegment {
					id: "loop".to_string(),
					allowed_transitions: HashSet::new(),
				},
				"end".to_string() => SongSegment {
					id: "end".to_string(),
					allowed_transitions: HashSet::new(),
				}
			),
			has_end: true,
			has_multiple_loops: false,
			has_dedicated_transitions: false
		});
		assert_eq!(songs["2"], Song {
			id: "2".to_string(),
			segments: map!(
				"start".to_string() => SongSegment {
					id: "start".to_string(),
					allowed_transitions: HashSet::new(),
				},
				"loop0".to_string() => SongSegment {
					id: "loop0".to_string(),
					allowed_transitions: HashSet::new(),
				},
				"loop1".to_string() => SongSegment {
					id: "loop1".to_string(),
					allowed_transitions: HashSet::new(),
				},
				"end".to_string() => SongSegment {
					id: "end".to_string(),
					allowed_transitions: HashSet::new(),
				}
			),
			has_end: true,
			has_multiple_loops: true,
			has_dedicated_transitions: false
		});
		assert_eq!(songs["3"], Song {
			id: "3".to_string(),
			segments: map!(
				"start".to_string() => SongSegment {
					id: "start".to_string(),
					allowed_transitions: HashSet::new(),
				},
				"loop0".to_string() => SongSegment {
					id: "loop0".to_string(),
					allowed_transitions: HashSet::new(),
				},
				"loop0-to-1".to_string() => SongSegment {
					id: "loop0-to-1".to_string(),
					allowed_transitions: HashSet::new(),
				},
				"loop1".to_string() => SongSegment {
					id: "loop1".to_string(),
					allowed_transitions: HashSet::new(),
				},
				"end".to_string() => SongSegment {
					id: "end".to_string(),
					allowed_transitions: HashSet::new(),
				}
			),
			has_end: true,
			has_multiple_loops: true,
			has_dedicated_transitions: true
		});
	}

	#[test]
	fn test_initialize_transitions() {
		let mut songs = map! {
			"1".to_string() => Song {
				id: "1".to_string(),
				segments: map!(
					"start".to_string() => SongSegment {
						id: "start".to_string(),
						allowed_transitions: HashSet::new(),
					},
					"loop".to_string() => SongSegment {
						id: "loop".to_string(),
						allowed_transitions: HashSet::new(),
					},
					"end".to_string() => SongSegment {
						id: "end".to_string(),
						allowed_transitions: HashSet::new(),
					}
				),
				has_end: true,
				has_multiple_loops: false,
				has_dedicated_transitions: false
			},
			"2".to_string() => Song {
				id: "2".to_string(),
				segments: map!(
					"start".to_string() => SongSegment {
						id: "start".to_string(),
						allowed_transitions: HashSet::new(),
					},
					"loop0".to_string() => SongSegment {
						id: "loop0".to_string(),
						allowed_transitions: HashSet::new(),
					},
					"loop1".to_string() => SongSegment {
						id: "loop1".to_string(),
						allowed_transitions: HashSet::new(),
					},
					"end".to_string() => SongSegment {
						id: "end".to_string(),
						allowed_transitions: HashSet::new(),
					}
				),
				has_end: true,
				has_multiple_loops: true,
				has_dedicated_transitions: false
			},
			"3".to_string() => Song {
				id: "3".to_string(),
				segments: map!(
					"start".to_string() => SongSegment {
						id: "start".to_string(),
						allowed_transitions: HashSet::new(),
					},
					"loop0".to_string() => SongSegment {
						id: "loop0".to_string(),
						allowed_transitions: HashSet::new(),
					},
					"loop0-to-1".to_string() => SongSegment {
						id: "loop0-to-1".to_string(),
						allowed_transitions: HashSet::new(),
					},
					"loop1".to_string() => SongSegment {
						id: "loop1".to_string(),
						allowed_transitions: HashSet::new(),
					},
					"end".to_string() => SongSegment {
						id: "end".to_string(),
						allowed_transitions: HashSet::new(),
					}
				),
				has_end: true,
				has_multiple_loops: true,
				has_dedicated_transitions: true
			}
		};

		initialize_transitions(&mut songs);

		assert_eq!(songs["1"].segments["start"].allowed_transitions, set!["loop".to_string()]);
		assert_eq!(songs["1"].segments["loop"].allowed_transitions, set!["end".to_string()]);
		assert_eq!(songs["1"].segments["end"].allowed_transitions, HashSet::new());

		assert_eq!(songs["2"].segments["start"].allowed_transitions, set!["loop0".to_string()]);
		assert_eq!(songs["2"].segments["loop0"].allowed_transitions, set!["loop1".to_string(), "end".to_string()]);
		assert_eq!(songs["2"].segments["loop1"].allowed_transitions, set!["loop0".to_string(), "end".to_string()]);
		assert_eq!(songs["2"].segments["end"].allowed_transitions, HashSet::new());

		assert_eq!(songs["3"].segments["start"].allowed_transitions, set!["loop0".to_string()]);
		assert_eq!(songs["3"].segments["loop0"].allowed_transitions, set!["loop0-to-1".to_string(), "end".to_string()]);
		assert_eq!(songs["3"].segments["loop0-to-1"].allowed_transitions, set!["loop1".to_string()]);
		assert_eq!(songs["3"].segments["loop1"].allowed_transitions, set!["end".to_string()]);
		assert_eq!(songs["3"].segments["end"].allowed_transitions, HashSet::new());
	}

	proptest! {
		#[test]
		fn prop_multiloop_song_should_not_contain_references_to_loop(song_id in "[a-z0-9]*", loop_count in 2..10) {
			let mut paths: Vec<String> = vec![format!("songs/song_{}_start.ogg", song_id)];
			for i in 0..loop_count {
				paths.push(format!("songs/song_{}_loop{}.ogg", song_id, i))
			}

			let songs: HashMap<String, Song> = initialize_songs(&paths);
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
			prop_assert!(songs[&song_id].segments["end"].allowed_transitions.is_empty());
		}
	}
}

#[cfg(test)]
mod test_song_planning {
	use super::*;

	proptest! {
		#[test]
		fn prop_plan_should_end_with_end(song in song_strategy(12, true)) {
			let mut rng = rand::thread_rng();
			let song_id = song.id.to_string();
			let mut songs: HashMap<String, Song> = map!(song_id.clone() => song);
			initialize_transitions(&mut songs);
			let plan = songs[&song_id].make_plan(&mut rng);
			prop_assert_eq!(&plan.last().unwrap().id, &"end".to_string())
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
	}
}

fn main() {
	// let args: Vec<String> = env::args().collect();

	let paths = fs::read_dir("./songs").unwrap();
	let path_strings = paths.map(|p| p.unwrap().path().display().to_string()).collect::<Vec<_>>();

	let mut songs = initialize_songs(&path_strings);
	initialize_transitions(&mut songs);
	// println!("{:#?}", songs);

	let mut rng = rand::thread_rng();
	let device = rodio::default_output_device().unwrap();
	let sink = Sink::new(&device);

	loop {
		let current_song_id = *songs.keys().collect::<Vec<_>>().choose(&mut rng).unwrap();
		let current_song = &songs[current_song_id];
		println!("Now playing: {}", current_song_id);

		let plan = current_song.make_plan(&mut rng);
		// println!("{:#?}", plan);

		for segment in &plan {
			let source = current_song.read_segment(&segment.id).buffered();
			if REGEX_IS_LOOP.is_match(&segment.id) && !REGEX_IS_DEDICATED_TRANSITION.is_match(&segment.id) {
				let repeat_counts = rng.gen_range(3, 12);
				for _ in 0..repeat_counts {
					sink.append(source.clone());
				}
			}
			else {
				sink.append(source);
			}
		}
		if !current_song.has_end {
			let id = &plan.last().unwrap().id;
			let source_end = current_song.read_segment(&id);
			let empty_source: Zero<f32> = Zero::new(source_end.channels(), source_end.sample_rate());
			sink.append(source_end.take_crossfade_with(empty_source, Duration::from_secs(8)));
		}

		sink.sleep_until_end();
	}
}
