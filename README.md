# bawa

**bawa** is a tui game save organizer.

![demo](https://github.com/user-attachments/assets/8db6482b-0d70-4231-84e5-f0314e330298)

## Features

- Import and load save files.
- Group save files using profiles.
- Quickly search and jump using fuzzy finder.
- Presets for Dark Souls Remastered, Dark Souls II, Dark Souls III, Sekiro and Elden Ring.
- Custom games can be added given that they use a single file for save data.
- Command line interface with [shell completion](#shell-completion), which can be used for setting global key bindings.
- Customizable key bindings and theme through [`config.toml` file](#configuration)

## Dependencies

**bawa** needs [`Nerd Fonts`](https://www.nerdfonts.com/) for icons. If you don't want to use it,
you can configure the icons in [`config.toml`](#configuration).

## Installation

### Cargo

**bawa** can be installed from [crates.io](https://crates.io/crates/bawa).

```sh
cargo install bawa
```

### Arch Linux

**bawa** can be installed from the [AUR](https://aur.archlinux.org/packages/bawa)
using an [AUR helper](https://wiki.archlinux.org/title/AUR_helpers).

```sh
paru -S bawa
```

## Usage

Run `bawa` to launch the TUI.

```sh
Usage: bawa [OPTIONS] [COMMAND]

Commands:
  list     list save files
  load     load save file
  import   import save file
  rename   rename save file
  delete   delete save file
  game     manage games
  profile  manage profiles
  help     Print this message or the help of the given subcommand(s)

Options:
  -c, --config <FILE>  Path to configuration file
      --no-config      Ignore configuration file
  -h, --help           Print help
  -V, --version        Print version
```

For default key bindings, press `ctrl-h` or `F1` in the app, or refer to
the [example `config.toml` file](./example/config.toml).

## Shell Completion

**bawa** supports dynamic shell completions for `bash`, `zsh`, `fish`, `elvish` and `powershell`.

### Bash

Add the following line to your `.bashrc`:

```bash
source <(COMPLETE=bash bawa)
```

### Zsh

Add the following line to your `.zshrc`:

```zsh
source <(COMPLETE=zsh bawa)
```

### Fish

Create `~/.config/fish/completions/bawa.fish` with the following line as the content:

```fish
source (COMPLETE=fish bawa | psub)
```

### Elvish

Add the following line to `~/.config/elvish/rc.elv`:

```elvish
eval (env COMPLETE=elvish bawa | slurp)
```

### Powershell

Add the following line to `$PROFILE`:

```pwsh
env COMPLETE=powershell bawa | Out-String | Invoke-Expression
```

## Configuration

Options, key bindings and theme can be configured with a configuration file
in [TOML](https://toml.io/en/) format. By default, the platform specific path listed in the following
table will be checked for the configuration file:

| Platform | Path                                                 |
| -------- | ---------------------------------------------------- |
| Linux    | `$XDG_CONFIG_HOME`/bawa/config.toml                  |
| MacOS    | `$HOME`/Library/Application Support/bawa/config.toml |
| Windows  | `%AppData%`\bawa\config.toml                         |

A different path can be specified with the `--config` flag. Configuration files can be ignored with
the `--no-config` flag to launch the app with the default settings.

A sample configuration file with the default settings can be found in [example/config.toml](./example/config.toml).
