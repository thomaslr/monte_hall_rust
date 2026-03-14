# Monte Hall Paradox Simulator — Rust/WASM Edition

A fully interactive Monte Hall paradox simulator built **entirely in Rust** using [Dioxus](https://dioxuslabs.com) and compiled to **WebAssembly**. No JavaScript or TypeScript — pure Rust runs in your browser!

## Features

- 🚪 **Interactive Mode** — Play the Monty Hall game manually: pick a door, watch the host reveal a goat, then switch or stick.
- ⚡ **Turbo Warp Speed** — Run millions of simulations at near-native speed in your browser via WASM.
- 👁️ **Visual Demo** — Watch the game play out automatically with animated door reveals.
- 📊 **Live Statistics** — Real-time dashboard tracking switch vs. stick win rates.
- 📈 **Convergence Graph** — SVG chart showing probability convergence over time (towards ≈66.7% switch / ≈33.3% stick).
- 🎨 **Premium Dark Theme** — Sleek design with gradients, glassmorphism, and micro-animations.

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Language | Rust 🦀 |
| UI Framework | Dioxus 0.6 |
| Compilation Target | WebAssembly (wasm32-unknown-unknown) |
| Bundler | Trunk |
| Styling | Vanilla CSS |
| Production Server | nginx (Alpine) |
| Containerization | Docker (multi-arch) |

## Quick Start

### Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [Trunk](https://trunkrs.dev/) (`cargo install trunk`)
- wasm32 target (`rustup target add wasm32-unknown-unknown`)

### Development

```bash
trunk serve --port 8081
```

Open http://localhost:8081 in your browser.

### Run Tests

```bash
cargo test
```

### Production Build

```bash
trunk build --release
```

Static files are output to the `dist/` directory.

### Docker

```bash
docker build -t monte_hall_rust .
docker run -p 8080:80 monte_hall_rust
```

## License

MIT
