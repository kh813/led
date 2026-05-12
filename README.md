# led (lightweight editor)

**led** is a modern, lightweight TUI text editor built in Rust. It aims to provide a clean, accessible, and powerful editing experience for terminal users.

## Features

- **Modern TUI Aesthetics**: Clean, high-contrast UI that feels at home in modern terminals.
- **Menu-Driven**: Top-level menu bar for easy access to all features.
- **Dialog-Based**: GUI-like dialogs for file operations and settings.
- **Internationalization (i18n)**: Support for English and Japanese out of the box.
- **Syntax Highlighting**: Fast, regex-based highlighting for many languages.
- **Tabs Support**: Open multiple files in separate tabs.
- **Vi Mode**: Optional Vi keybindings for power users.
- **OSC 52 Clipboard**: Shared clipboard support over SSH.

## Installation

### From Source

```bash
git clone https://github.com/yourname/led.git
cd led
make
```

The binary will be built in `dist/led`.

## Usage

```bash
led [FILE...]
```

For more details, see the [User Manual](MANUAL.md).

## Configuration

Configuration is stored in `~/.config/led/config.toml`. You can copy the template from `config.toml.default` to get started.

## Remote Usage (SSH)

led works great over SSH. To prevent `Ctrl+S` from freezing your terminal, it is recommended to add the following to your shell profile (`.bashrc` or `.zshrc`):

```bash
stty -ixon
```

## License

MIT
