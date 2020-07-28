use core::fmt::Formatter;
use core::fmt::Display;
use std::fs::File;
use std::io::BufReader;
use std::collections::HashSet;
use std::fs;
use rodio::Sink;
use rand::Rng;

// Do NOT use mp3.

// Song:
// - id: string
// - has_end: bool
// - multi_loop_count: bool
// - valid_transitions: dict[loop num] = array of loop nums

struct Song {
	start_file: String,
	loop_file: String,
	end_file: String,
}

impl Display for Song {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.start_file)
	}
}

fn main() {
	let paths = fs::read_dir("./songs").unwrap();
	let mut unique_songs = HashSet::new();
	for path in paths.map(|p| p.unwrap().path().display().to_string()) {
		let file_name = path.split('/').rev().nth(0).unwrap().clone().to_string();
		let song_num = file_name.split('_').nth(1).unwrap();
		unique_songs.insert(song_num.to_owned());
	}
	let mut sorted_song_nums = unique_songs.into_iter().collect::<Vec<_>>();
	sorted_song_nums.sort();

	let mut songs = vec![];

	for song in sorted_song_nums {
		songs.push(Song {
			start_file: format!("song_{}_start.ogg", song),
			loop_file: format!("song_{}_loop.ogg", song),
			end_file: format!("song_{}_end.ogg", song),
		});
	}

	let mut rng = rand::thread_rng();

	let device = rodio::default_output_device().unwrap();
	let sink = Sink::new(&device);

	loop {
		let song_num = rng.gen_range(0, songs.len());
		let repeat_count = rng.gen_range(3, 15);
		let current_song = &songs[song_num];
		println!("playing: song {}, repeated {} times", song_num, repeat_count);

		let file_start = File::open(format!("songs/{}", current_song.start_file.as_str())).unwrap();
		let file_end = File::open(format!("songs/{}", current_song.end_file.as_str())).unwrap();
		let source_start = rodio::Decoder::new(BufReader::new(file_start)).unwrap();
		let source_end = rodio::Decoder::new(BufReader::new(file_end)).unwrap();

		sink.append(source_start);
		for _ in 0..repeat_count {
			let file_loop = File::open(format!("songs/{}", current_song.loop_file.as_str())).unwrap();
			let source_loop = rodio::Decoder::new(BufReader::new(file_loop)).unwrap();
			sink.append(source_loop);
		}
		sink.append(source_end);
		sink.sleep_until_end();
	}
}
