/// A Dria & DNET ASCII art banner for the menu screen.
pub const MENU_BANNER: [&str; 18] = [
    "                                                                             ",
    "      00000    000000                                                        ",
    "   000    000000000000000   0000000000000000      000000000000          00000",
    " 000       000000   000000000   00000    00000 000    00000            000000",
    "00        00000     000000     00000    00000000     00000           00000000",
    "00       00000     0000000    00000    00000000     00000           00 000000",
    "00      00000     0000000    0000000000000  000    00000          00  0000000",
    " 000   00000     0000000    00000   000000  00    000000        000   000000 ",
    "      00000      00000     00000    00000   00   00000000      00    0000000 ",
    "     00000     000000     000000   000000    00 000000  0000000000000 00000  ",
    "    00000    0000000     00000     00000 0     000000      000        00000  ",
    " 0000000   00000       0000000    00000000  000000000    000        0000000  ",
    "",
    "",
    " ⠀⠀⣠⣤⠐⣦⡀⠀⠴⠢⣤⣄⠀⢀⠄⠀⠀⢠⣶⠂⠀⢐⠆⢀⡤⢠⣤⠂⢤",
    " ⠀⣰⡟⠀⢠⣿⠁⠀⠀⠌⢹⣿⢀⠎⠀⡄⢠⣿⠃⡴⠀⠀⠀⠊⢀⣾⠃⠀⠁",
    "⢀⣰⡟⢀⡴⠟⠁⠀⢀⠈⠀⠘⣿⠏⠀⠀⣰⣿⡁⢀⡰⠀⠀⠀⣠⣿⠃⠀⠀⠀",
    crate::constants::VERSION,
];

/// Available models for loading.
pub const AVAILABLE_MODELS: &[&str] = &[
    // qwen 4b
    "Qwen/Qwen3-4B-MLX-4bit",
    "Qwen/Qwen3-4B-MLX-8bit",
    // qwen 30b a3b
    "Qwen/Qwen3-30B-A3B-MLX-8bit",
    "Qwen/Qwen3-30B-A3B-MLX-bf16",
    "Qwen/Qwen3-30B-A3B-MLX-6bit",
    // qwen 32b
    "Qwen/Qwen3-32B-MLX-bf16",
    "Qwen/Qwen3-32B-MLX-8bit",
    "Qwen/Qwen3-32B-MLX-6bit",
    // nous
    "NousResearch/Hermes-4-70B",
    // llama 8b
    "mlx-community/Meta-Llama-3.1-8B-Instruct-4bit",
    "mlx-community/Meta-Llama-3.1-8B-Instruct-8bit",
    // llama 70b
    "mlx-community/Meta-Llama-3.1-70B-4bit",
    "mlx-community/Meta-Llama-3.1-70B-8bit",
    // gpt OSS 20b
    "openai/gpt-oss-20b",
    "mlx-community/gpt-oss-20b-MXFP4-Q4",
    "mlx-community/gpt-oss-20b-MXFP4-Q8",
    // gpt OSS 120b
    "openai/gpt-oss-120b",
    "mlx-community/gpt-oss-120b-MXFP4-Q4",
    "mlx-community/gpt-oss-120b-MXFP4-Q8",
];

/// Version from `Cargo.toml`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
