<div align="center">

  ![Logo](./assets/gyr.png)

  [![License](https://img.shields.io/crates/l/gyr?style=flat-square)](https://git.sr.ht/~f9/gyr/blob/main/LICENSE)
  [![Latest version](https://img.shields.io/crates/v/gyr?style=flat-square)](https://crates.io/crates/gyr)
  [![standard-readme compliant](https://img.shields.io/badge/readme%20style-standard-brightgreen.svg?style=flat-square)](https://github.com/RichardLitt/standard-readme)
  ![written in Rust](https://img.shields.io/badge/language-rust-red.svg?style=flat-square)

  _Blazing fast_ TUI launcher for GNU/Linux and \*BSD

  [![asciicast](https://asciinema.org/a/n34HCGxXINEoryRkuM8XOIVbJ.svg)](https://asciinema.org/a/n34HCGxXINEoryRkuM8XOIVbJ)

</div>

## Table of Contents

- [Install](#install)
- [Usage](#usage)
- [TODO](#todo)
- [Contributing](#contributing)
- [Changelog](#changelog)
- [License](#license)

## Install

#### Option 1: Build from source

* Install [Rust](https://www.rust-lang.org/learn/get-started)
* Build:
    ```sh
    $ cargo install gyr
    ```
* Add `$HOME/.cargo/bin` to your `$PATH`

Or build from Git:

* Build:
    ```sh
    $ git clone https://git.sr.ht/~f9/gyr && cd gyr
    $ cargo build --release
    ```
* Copy `target/release/gyr` to somewhere in your `$PATH`

#### Option 2: Distro packages

Gyr is in the Arch Linux AUR (`gyr`, `gyr-git` and `gyr-bin`).

Other distros may come soon-ish

Alternatively, pre-built binaries for Linux (x86_64 and aarch64) and FreeBSD 13.1 (x86_64) are available in the [releases](https://git.sr.ht/~f9/gyr/refs/).

## Usage

Run `gyr` from a terminal. Scroll through the app list, find some app typing chars, run selected pressing ENTER. Pretty straightforward.
Oh, yes: go to the bottom with the left arrow, top with right. Cancel pressing Esc.

Alternative bindings are Ctrl-Q to cancel, Ctrl-Y to run the app, Ctrl-N scroll down and Ctrl-P to scroll up (VIM bindings).

I designed it for tiling WMs like [Sway](https://swaywm.org/) or [i3](https://i3wm.org/).

> Note for Sway: When `$SWAYSOCK` is set, `swaymsg exec` is used to run the program.
> This allows Sway to spawn the program in the workspace Gyr was run in.

You can configure some stuff with cli flags, see `gyr --help`

Gyr also has a history feature, so most used entries will be sorted first. This can be reset with `gyr --clear_history`

There's also a config file which can be placed in `$HOME/.config/gyr/config.toml` or `$XDG_DATA_HOME/gyr/config.toml` ([sample](./config.toml))

Verbosity levels (`-v`, `-vv`, `-vvv`, each level adds logs to the previous one):

* `-v`: will make the launched binary inherit Gyr's `stdio`. (which means you'll see the logs)
* `-vv`: will show the path of each app in the info
* `-vvv`: adds some debug information (number of times the apps were run, etc.)

### Sway-specific usage

This is what I have on my config file:

```shell
$ cat ~/.config/sway/config
...
set $menu alacritty --title launcher -e gyr
bindsym $mod+d exec $menu
for_window [title="^launcher$"] floating enable, resize set width 500 height 430, border none
...
```

## TODO

* [X] Most used entries first
* [ ] Cached entries

## Contributing

Feature requests and bug reports are most welcome.

I'll accept patchsets fixing bugs or adding requested features.

NOTE: The preferred way to contribute is via SourceHut, tickets can be opened at https://todo.sr.ht/~f9/gyr

The GitLab releases & issues are kept for convenience, but merge requests are closed.

## Changelog

Notable changes will be documented in the [CHANGELOG](./CHANGELOG.md) file

## License

[BSD 2-Clause](./LICENSE) (c) 2020-2022 Namkhai B.
