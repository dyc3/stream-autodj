use std::fs::File;
use std::io::BufReader;
use std::collections::{HashSet, HashMap};
use std::fs;
use std::env;
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
		let file = File::open(format!("songs/song_{}_{}.ogg", self.id.as_str(), segment)).unwrap();
		Decoder::new(BufReader::new(file)).unwrap()
	}

	// fn read_loop(&self, loop_num: Option<Box<i32>>) -> Decoder<BufReader<File>> {
	// 	match loop_num {
	// 		Some(n) => self.read_segment(format!("loop{}", n).as_str()),
	// 		None => self.read_segment("loop")
	// 	}
	// }

	// fn read_loop_transition(&self, from:i32, to:i32) -> Decoder<BufReader<File>> {
	// 	self.read_segment(format!("loop{}-to-{}", from, to).as_str())
	// }

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
					if self.has_end && next_segment_id.ends_with("end") || plan.len() > 5 {
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

pub fn initialize_songs(paths: &[String]) -> HashMap<String, Song> {
	let mut songs = HashMap::new();
	for path in paths {
		let file_name = path.split('/').rev().next().unwrap().to_string();
		let name_split = file_name.split('_').collect::<Vec<&str>>();
		let song_id = name_split[1].to_string();
		let song_segment_id = name_split[2].to_string().split('.').collect::<Vec<&str>>()[0].to_string();
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
		if !song.has_multiple_loops && song_segment_id != "loop" && REGEX_IS_LOOP.is_match(song_segment_id.as_str()) {
			song.has_multiple_loops = true;
		}
		if !song.has_dedicated_transitions && REGEX_IS_DEDICATED_TRANSITION.is_match(song_segment_id.as_str()) {
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
				let (_, loop_to) = (loop_nums.get(1).unwrap(), loop_nums.get(2).unwrap());
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
					}
					"loop" => {
						if song.has_end {
							song_segment.allowed_transitions.insert("end".to_string());
						}
					}
					_ => {}
				}
			}
		}
	}
}

#[cfg(test)]
mod test_song_parsing {
	use super::*;

	macro_rules! map(
		{ $($key:expr => $value:expr),+ } => {
			{
				let mut m = ::std::collections::HashMap::new();
				$(
					m.insert($key, $value);
				)+
				m
			}
		 };
	);

	macro_rules! set {
		( $( $x:expr ),* ) => {  // Match zero or more comma delimited items
			{
				let mut temp_set = HashSet::new();  // Create a mutable HashSet
				$(
					temp_set.insert($x); // Insert each item matched into the HashSet
				)*
				temp_set // Return the populated HashSet
			}
		};
	}

	#[test]
	fn test_initialize_songs() {
		let paths = [
			"songs/song_1_start.ogg".to_string(),
			"songs/song_1_loop.ogg".to_string(),
			"songs/song_1_end.ogg".to_string(),
			"songs/song_2_start.ogg".to_string(),
			"songs/song_2_loop0.ogg".to_string(),
			"songs/song_2_loop1.ogg".to_string(),
			"songs/song_2_end.ogg".to_string(),
			"songs/song_3_start.ogg".to_string(),
			"songs/song_3_loop0.ogg".to_string(),
			"songs/song_3_loop0-to-1.ogg".to_string(),
			"songs/song_3_loop1.ogg".to_string(),
			"songs/song_3_end.ogg".to_string()
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
		let mut songs = map!{
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

	proptest!{
		#[test]
		fn prop_multiloop_song_should_not_contain_references_to_loop(song_id in "[a-z0-9]*", loop_count in 2..10) {
			let mut paths: Vec<String> = vec![format!("songs/song_{}_start.ogg", song_id)];
			for i in 0..loop_count {
				paths.push(format!("songs/song_{}_loop{}.ogg", song_id, i))
			}

			let songs: HashMap<String, Song> = initialize_songs(&paths);
			for song in songs.values() {
				println!("{:#?}", song);
				for segment in song.segments.values() {
					prop_assert_ne!(&segment.id, "loop");
					for transition in &segment.allowed_transitions {
						prop_assert_ne!(transition, &"loop".to_string());
					}
				}
			}
		}
	}
}

fn main() {
	let args: Vec<String> = env::args().collect();

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
		println!("DEBUG: song id {}", current_song_id);

		let plan = current_song.make_plan(&mut rng);
		// println!("{:#?}", plan);

		for segment in plan.clone() {
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

		// let source_start = current_song.read_segment("start");
		// sink.append(source_start);

		// if current_song.multi_loop_count == 1 {
		// 	let repeat_count = rng.gen_range(3, 15);
		// 	let source_loop = read_song_loop(current_song, Option::None).buffered();
		// 	for _ in 0..repeat_count {
		// 		sink.append(source_loop.clone());
		// 	}
		// 	println!("playing: song {}, repeated {} times", current_song, repeat_count);
		// }
		// else if current_song.valid_transitions.is_empty() {
		// 	let loop_transitions = rng.gen_range(1, 4);
		// 	let loop_plays = (0..loop_transitions).map(|_| {
		// 		(rng.gen_range(0, current_song.multi_loop_count), rng.gen_range(3, 15))
		// 	}).collect::<Vec<_>>();
		// 	println!("playing: song {}, {} loop transitions, repeated {:?} times", current_song, loop_transitions, &loop_plays);
		// 	for (loop_num, repeats) in loop_plays {
		// 		let source_loop = read_song_loop(current_song, Some(Box::new(loop_num))).buffered();
		// 		for _ in 0..repeats {
		// 			sink.append(source_loop.clone());
		// 		}
		// 	}
		// }
		// else {
		// 	let loop_transitions = rng.gen_range(1, 4);
		// 	println!("playing: song {}, {} loop transitions with special loop transitions", current_song, loop_transitions);
		// 	let mut current_loop_num = 0;
		// 	let mut flow = vec![];
		// 	for _ in 0..loop_transitions {
		// 		match &current_song.valid_transitions.get(&current_loop_num) {
		// 			Some(possible_next_loops) => {
		// 				current_loop_num = *possible_next_loops.choose(&mut rng).unwrap();
		// 				flow.push(current_loop_num);
		// 			}
		// 			None => {
		// 				flow.push(0);
		// 			}
		// 		}
		// 	}

		// 	let repeats = rng.gen_range(3, 7);
		// 	current_loop_num = 0;
		// 	let source_loop = read_song_loop(current_song, Some(Box::new(current_loop_num))).buffered();
		// 	for _ in 0..repeats {
		// 		sink.append(source_loop.clone());
		// 	}

		// 	for loop_num in flow {
		// 		if fs::metadata(format!("songs/song_{}_loop{}-to-{}.ogg", current_song.id, current_loop_num, loop_num)).is_ok() {
		// 			let source_transition = read_song_loop_transition(current_song, current_loop_num, loop_num);
		// 			sink.append(source_transition);
		// 		}
		// 		else {
		// 			println!("CROSSFADING");
		// 			let source_from = read_song_loop(current_song, Some(Box::new(current_loop_num)));
		// 			let source_to = read_song_loop(current_song, Some(Box::new(loop_num)));
		// 			match source_from.total_duration() {
		// 				Some(duration) => {
		// 					sink.append(source_from.take_crossfade_with(source_to, duration));
		// 				}
		// 				None => {
		// 					sink.append(source_from.take_crossfade_with(source_to, Duration::from_secs(8)));
		// 				}
		// 			}
		// 		}
		// 		current_loop_num = loop_num;

		// 		let repeats = rng.gen_range(3, 7);
		// 		let source_loop = read_song_loop(current_song, Some(Box::new(current_loop_num))).buffered();
		// 		for _ in 0..repeats {
		// 			sink.append(source_loop.clone());
		// 		}
		// 	}
		// }
		// if current_song.has_end {
		// 	let source_end = read_song_segment(current_song, "end");
		// 	sink.append(source_end);
		// }
		// else {
		// 	let source_end;
		// 	let loop_num = match current_song.multi_loop_count {
		// 		1 => None,
		// 		_ => Some(Box::new(0))
		// 	};
		// 	source_end = read_song_loop(current_song, loop_num).buffered();
		// 	let empty_source: Zero<f32> = Zero::new(source_end.channels(), source_end.sample_rate());
		// 	match source_end.total_duration() {
		// 		Some(duration) => {
		// 			sink.append(source_end.take_crossfade_with(empty_source, duration));
		// 		}
		// 		None => {
		// 			println!("failed to grab end duration");
		// 			sink.append(source_end.take_crossfade_with(empty_source, Duration::from_secs(8)));
		// 		}
		// 	}
		// }
		// sink.sleep_until_end();
	}
}
