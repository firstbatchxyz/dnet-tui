# dnet-tui

A terminal user interface (TUI) application built with Rust for [dnet](https://github.com/firstbatchxyz/dnet).

## Installation

Install using `cargo`:

```sh
cargo install https://github.com/firstbatchxyz/dnet-tui.git
```

> You can install from source as well:
>
> ```sh
> git clone https://github.com/firstbatchxyz/dnet-tui.git
> cd dnet-tui
> cargo build --release
> ```

## Usage

Run the application:

```sh
dnet-tui
```

> To run from source:
>
> ```sh
> cargo run
> ```

## Contributions

The code is structured so that all "windows" are thought of as their own modules, and they implement the required methods via `impl App` within their own file, with respect to visibility. Each window should also have a `*View` enum (for the sub-windows if required) and a `*State` struct that is an attribute of `AppState`.

Within each `impl App` we expect the following methods:

- `draw_*` to handle drawing on screen (called via `terminal.draw`)
- `tick_*` to handle ticks (effect within the running loop)
- `handle_*` to handle inputs from the user

## License

See the [LICENSE](LICENSE) file for details.
