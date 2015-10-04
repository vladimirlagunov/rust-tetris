# rust-tetris
[![Build Status](https://travis-ci.org/werehuman/rust-tetris.svg?branch=master)](https://travis-ci.org/werehuman/rust-tetris)

Simple Tetris implementation written in Rust. Has no scores, has no animation, just a game.

![Screenshot](http://i.imgur.com/0fXT0eX.png)

# Building
## Mac OS X with homebrew

You need to install Rust 1.2 or higher and SDL2.
```
$ brew install sdl2
```

Specify path to SDL library when building and running:
```
$ export LIBRARY_PATH=path/to/homebrew/Cellar/SDL2/<version>/lib:${LIBRARY_PATH}
$ cargo run
```
