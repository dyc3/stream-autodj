# stream-autodj

![Rust](https://github.com/dyc3/stream-autodj/workflows/Rust/badge.svg)

This is the program I use to play background music on my streams. To reduce repetition, it plays songs in a random order, for random durations using pre-made song segments. Smooth transitions between songs and different loop segments are guarenteed.

https://twitch.tv/rollthedyc3

# Installation

## Required Packages

### Ubuntu

```
sudo apt-get install libasound2-dev
```

## Running

```
cargo run --release
```

# Usage

For each song you want to add, place audio files (the song segments) in the `songs` folder in the following format:
Allowed formats include:
`wav`, `ogg`, `mp3`, and `flac`
`mp3` files technically work, but you will get weird pauses when transitioning between different segments.

```
song_SONGNAME_start.ogg
song_SONGNAME_loop.ogg
```

All songs require a `start` segment, and at least 1 `loop` segment. If the song only has one loop, the loop segment must be called `loop`. Segments must have a matching `SONGNAME` in order to be associated with each other.

You can add multiple loops that will be switched between at random intervals:

```
song_SONGNAME_loop0.ogg
song_SONGNAME_loop1.ogg
song_SONGNAME_loop2.ogg
...
```

You can add dedicated transitions between loops like this:

```
song_SONGNAME_loop0-to-1.ogg
song_SONGNAME_loop1-to-0.ogg
song_SONGNAME_loop2-to-0.ogg
...
```
Using dedicated loops at all requires the program to plan the song's playback using **only** dedicated transitions.
This means if a loop segment does not have any dedicated transitions that lead to that segment, it will be unreachable and not be played.

You can add a dedicated end to the song as well:
```
song_SONGNAME_end.ogg
```
If no dedicated end segment is supplied, the loop will fade out before switching to the next song.

# Contributing

Contributions are welcome! Simply fork the repo, make your changes, and make a pull request.

## Style

Style is enforced by `rustfmt`. To auto format your code to comply, you must use the nightly version of rustfmt ([See instructions here](https://github.com/rust-lang/rustfmt#on-the-nightly-toolchain)).

```
cargo +nightly fmt
```

# License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

# Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
