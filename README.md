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
SONGNAME_start.ogg
SONGNAME_loop.ogg
```

All songs require a `start` segment, and at least 1 `loop` segment. If the song only has one loop, the loop segment must be called `loop`. Segments must have a matching `SONGNAME` in order to be associated with each other.

You can add multiple loops that will be switched between at random intervals:

```
SONGNAME_loop0.ogg
SONGNAME_loop1.ogg
SONGNAME_loop2.ogg
...
```

You can add dedicated transitions between loops like this:

```
SONGNAME_loop0-to-1.ogg
SONGNAME_loop1-to-0.ogg
SONGNAME_loop2-to-0.ogg
...
```
Using dedicated loops at all requires the program to plan the song's playback using **only** dedicated transitions.
This means if a loop segment does not have any dedicated transitions that lead to that segment, it will be unreachable and not be played.

You can add a dedicated end to the song as well:
```
song_SONGNAME_end.ogg
```
Or you can add a loop specific ending:
```
SONGNAME_loop0-end.ogg
SONGNAME_loop5-end.ogg
```
If no dedicated end segment is supplied, the loop will fade out before switching to the next song.

The same format can also be used with `.zip` files, where the zip file contains the song name:

```
SONGNAME.zip
```

and the files contained in the `.zip` contain the segment id and correct file type:

```
$ unzip -l SONGNAME.zip
Archive:  SONGNAME.zip
  Length      Date    Time    Name
---------  ---------- -----   ----
    39514  2020-08-04 20:12   end.wav
    42078  2020-08-04 20:11   loop.wav
    44306  2020-08-04 20:12   start.wav
---------                     -------
   125898                     3 files
```

This allows for the easy packaging of songs, that way they can easily be renamed or moved as one unit, instead of as chunks.

# Contributing

Contributions are welcome! Simply fork the repo, make your changes, and make a pull request.

## Style

Style is enforced by `rustfmt`. To auto format your code to comply, you must use the nightly version of rustfmt ([See instructions here](https://github.com/rust-lang/rustfmt#on-the-nightly-toolchain)).

```
cargo +nightly fmt
```
