# Gyr

[![License](https://img.shields.io/crates/l/gyr?style=flat-square)](https://gitlab.com/forkbomb9/gyr/-/blob/master/LICENSE)
[![Latest version](https://img.shields.io/crates/v/gyr?style=flat-square)](https://crates.io/crates/gyr)
[![Build status](https://img.shields.io/gitlab/pipeline/forkbomb9/gyr?style=flat-square)]()
[![standard-readme compliant](https://img.shields.io/badge/readme%20style-standard-brightgreen.svg?style=flat-square)](https://github.com/RichardLitt/standard-readme)

Gyr launcher, a _blazing fast_ TUI launcher for *BSD and Linux

## Table of Contents

- [Install](#install)
- [Usage](#usage)
- [TODO](#todos)
- [Contributing](#contributing)
- [License](#license)

## Install

Once I setup the CI, there'll be binaries for FreeBSD and Linux.
But for now, build from source:

* Install [Rust](https://www.rust-lang.org/learn/get-started)
* Build:
    ```sh
    $ git clone https://gitlab.com/forkbomb9/gyr.git && cd gyr
    $ cargo build --release
    ```

* Copy `target/release/gyr` to somewhere in your `$PATH`

## Usage

Run `gyr` from a terminal. Scroll through the app list, find some app typing chars, run selected pressing ENTER. Pretty straightforward.
Oh, yes: go to the bottom with the left arrow, top with right. Cancel pressing Esc.

It's useful for tiling WMs, e.g. [Sway](https://swaywm.org/) or [i3](https://i3wm.org/)

> Note for Sway: When `$SWAYSOCK` is set, `swaymsg exec` is used to run the program.
> This allows Sway to spawn the program in the workspace Gyr was run in
> (i3 has `libstartup-notification`, but Alacritty doesn't implement it AND I haven't found a way to do so.

You can configure some things with cli flags, check `gyr --help`
There's also a config file which can be placed in `$HOME/.config/gyr/config.toml`/`$XDG_DATA_HOME/gyr/config.toml` ([sample](./config.toml))

## TODO

* ~~Most used entries first~~ Done! :tada:
* Cached entries

## Contributing

PRs are not accepted because for now this is my personal project, and I don't want to share it with anyone.
Maybe in the future :grinning:

But you can open bug reports / feature requests!

## License

[BSD 2-Clause](./LICENSE) (c) 2020 Namkhai B.
