[![Build Status](https://travis-ci.org/dpc/rhex.svg?branch=master)](https://travis-ci.org/dpc/rhex)

# rhex

## Introduction

Simple roguelike prototype written in [Rust][rust-home].

You can try it out by pointing your **ssh client** to: rhex [at] rhex.dpc.pw (password is obvious). Note: **Make sure your terminal supports 256 colors and exports `TERM=xterm-256color`!**

It's intendent to exercise my [Rust][rust-home] knowledge and let me play with
certain mechanisms that I'd like to see in roguelike game:

* hexagonal map
* tactical positioning (strafing, face-direction)

Previous iteration of this idea was/is: [Rustyhex][rustyhex] . This two project might merge into whole at some point.

This project is based on more general and useful: [hex2d-rs - Hexagonal grid map utillity library][hex2d-rs].

[rust-home]: http://rust-lang.org
[rustyhex]: //github.com/dpc/rustyhex
[hex2d-rs]: //github.com/dpc/hex2d-rs

## Overview

![RustyHex screenshot][ss]

[ss]: http://i.imgur.com/LI0FOPF.png

[Watch rhex demo video][screencast]

[screencast]: https://asciinema.org/a/16712

## Building

	git clone https://github.com/dpc/rhex.git
	cd rhex
	cargo build


## Status and goals

ATM. the game is just a working prototype. There's not much of gameplay yet.

Some core features that it already implements: simple AI, LoS, lighting, random map generation, areas, autoexplore command.

Core features that I'd like to add soon:

* sound (actions make noise that propagates)
* combat
* working stats
* stairs
* items/inventory

[Report problems and ideas][issues]

[issues]: https://github.com/dpc/rhex/issues

# How to play

## Basics

* Use `hjkl` or arrow keys to move.
* Press `o` to autoexplore
* Hold `Shift` to strafe (with Left/Right move)
* Press `.` to wait.


