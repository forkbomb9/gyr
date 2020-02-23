# FLauncher

[![License](https://img.shields.io/crates/l/flauncher?style=flat-square)](https://gitlab.com/forkbomb9/flauncher/-/blob/master/LICENSE)
[![Latest version](https://img.shields.io/crates/v/flauncher?style=flat-square)](https://crates.io/crates/flauncher)
[![Build status](https://img.shields.io/gitlab/pipeline/forkbomb9/flauncher?style=flat-square)]()
[![standard-readme compliant](https://img.shields.io/badge/readme%20style-standard-brightgreen.svg?style=flat-square)](https://github.com/RichardLitt/standard-readme)

Fantastic Launcher, a _fast_ TUI launcher for *BSD and Linux

## Table of Contents

- [Install](#install)
- [Usage](#usage)
- [Contributing](#contributing)
- [License](#license)

## Install

Once I setup the CI, there'll be binaries for FreeBSD and Linux.
But for now, build from source:

* Install [Rust](https://www.rust-lang.org/learn/get-started)
* Build:
    ```sh
    $ git clone https://gitlab.com/forkbomb9/flauncher.git && cd flauncher
    $ cargo build --release
    ```

* Copy `target/release/flauncher` to somewhere in your `$PATH`

## Usage

Run `flauncher` from a terminal. Scroll through the app list, find some app typing chars, run selected pressing ENTER. Pretty straightforward.
Oh, yes: go to the bottom with the left arrow, top with right. Cancel pressing Esc.

It's useful for tiling WMs, e.g. [Sway](https://swaywm.org/) or [i3](https://i3wm.org/)

You can configure some things with cli flags, check `flauncher --help`

## Contributing

PRs are not accepted because for now this is my personal project, and I don't want to share it with anyone
Maybe in the future :grinnig_face:
But you can open bug reports / feature requests!

## License

[BSD 2-Clause](./LICENSE) (c) 2020 Namkhai B.
