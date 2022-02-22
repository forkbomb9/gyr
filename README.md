<div align="center">

  ![Logo](./assets/gyr.svg)

  [![License](https://img.shields.io/crates/l/gyr?style=flat-square)](https://gitlab.com/forkbomb9/gyr/-/blob/master/LICENSE)
  [![Latest version](https://img.shields.io/crates/v/gyr?style=flat-square)](https://crates.io/crates/gyr)
  [![Build status](https://img.shields.io/gitlab/pipeline/forkbomb9/gyr?style=flat-square)]()
  [![standard-readme compliant](https://img.shields.io/badge/readme%20style-standard-brightgreen.svg?style=flat-square)](https://github.com/RichardLitt/standard-readme)

  A _blazing fast_ TUI launcher for \*BSD and Linux

  [![asciicast](https://asciinema.org/a/FmVNnU237SCEn7aP9nMYqABxd.svg)](https://asciinema.org/a/FmVNnU237SCEn7aP9nMYqABxd)

</div>

## Table of Contents

- [Install](#install)
- [Usage](#usage)
- [TODO](#todos)
- [Contributing](#contributing)
- [Changelog](#changelog)
- [License](#license)

## Install

#### Option 1: Build from source

* Install [Rust](https://www.rust-lang.org/learn/get-started)
* Build:
    ```sh
    $ git clone https://gitlab.com/forkbomb9/gyr.git && cd gyr
    $ cargo build --release
    ```

* Copy `target/release/gyr` to somewhere in your `$PATH`

#### Option 2: Pre-built binaries

Can be found in the [releases](https://gitlab.com/forkbomb9/gyr/-/releases) page.

They available for Linux, statically built against the musl libc for `x86_64` and `aarch64`.

For FreeBSD, I'm working on writting and publishing a port (It builds fine).

## Usage

Run `gyr` from a terminal. Scroll through the app list, find some app typing chars, run selected pressing ENTER. Pretty straightforward.
Oh, yes: go to the bottom with the left arrow, top with right. Cancel pressing Esc.

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

* ~~Most used entries first~~ Done! :tada:
* Cached entries

## Contributing

Feature requests and bug reports are most welcome.

I'll accept pull requests fixing bugs or adding requested features.

## Changelog

Notable changes will be documented in the [CHANGELOG](./CHANGELOG.md) file

## License

[BSD 2-Clause](./LICENSE) (c) 2020 Namkhai B.
