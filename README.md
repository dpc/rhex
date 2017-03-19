# rhex

<p align="center">
  <a href="https://travis-ci.org/dpc/rhex">
      <img src="https://img.shields.io/travis/dpc/rhex/master.svg?style=flat-square" alt="Build Status">
  </a>
  <a href="https://gitter.im/dpc/rhex">
      <img src="https://img.shields.io/badge/GITTER-join%20chat-green.svg?style=flat-square" alt="Gitter Chat">
  </a>
</p>

## Contributors welcome!

Rhex is looking for contributors. See
[Contributing page](https://github.com/dpc/rhex/wiki/Contributing)
for details.

## Introduction

Simple ASCII terminal hexagonal map  roguelike written in [Rust][rust-home].

You can try the game without installing anything. Check
[rhex demo server](https://github.com/dpc/rhex/wiki/Demo-server) instructions.

The core goal of the project:

* ASCI/Unicode pure terminal UI first
* hexagonal map with tactical positioning

It's also intendent to exercise and practice my [Rust][rust-home] knowledge.

Previous iteration of this idea was/is: [Rustyhex][rustyhex] . This two project
might merge into whole at some point.

Rhex is using [hex2d-rs - Hexagonal grid map utillity library][hex2d-rs].

[rust-home]: http://rust-lang.org
[rustyhex]: //github.com/dpc/rustyhex
[hex2d-rs]: //github.com/dpc/hex2d-rs

## Overview

![rhex v0.0.3 screenshot](http://i.imgur.com/M9fi1ri.png)

Watch *rhex* gameplay video:

[![asciicast](https://asciinema.org/a/34224.png)](https://asciinema.org/a/34224)

## Running

Game requires terminal with 256 colors support, and Unicode font.

	git clone https://github.com/dpc/rhex.git
	cd rhex
	cargo run --release

## Status

The game is playable but not feature and gameplay wise complete.

*rhex* is actively seeking collaborators. If you'd like to practice your Rust
or/and find roguelikes interesting ping `@dpc` on [rhex gitter channel][rhex-gitter] and we
can get your started.

[Report problems and ideas][issues]

[issues]: https://github.com/dpc/rhex/issues
[rhex-gitter]: https://gitter.im/dpc/rhex
