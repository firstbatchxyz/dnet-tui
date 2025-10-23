# dnet-tui

A terminal user interface (TUI) application built with Rust.

## Installation

### Using Cargo

```sh
cargo install  https://github.com/firstbatchxyz/dnet-tui.git
```

### From Source

```sh
git clone https://github.com/firstbatchxyz/dnet-tui.git
cd dnet-tui
cargo build --release
```

## Usage

Run the application:

```sh
dnet-tui
```

## Contributions

The code is structured so that all "windows" are thought of as their own modules, and they implement the required methods via `impl App` and `impl AppState` within their own file, with respect to visibility.

## License

See the [LICENSE](LICENSE) file for details.
