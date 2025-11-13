# Rustbot Development Guide

## Quick Start

### Prerequisites
- Rust 1.70+ (install from https://rustup.rs)
- Git

### Running with Hot-Reload

The best way to develop is using `cargo-watch` which automatically rebuilds and restarts the app when you make changes:

```bash
# Install cargo-watch (only needed once)
cargo install cargo-watch

# Run with hot-reload
cargo watch -x run
```

This will:
- Watch all Rust source files for changes
- Automatically recompile when you save
- Restart the application with your changes

### Manual Build and Run

If you prefer to run manually:

```bash
# Build the project
cargo build

# Run the application
cargo run
```

### Development Workflow

1. Start hot-reload: `cargo watch -x run`
2. Edit code in `src/` directory
3. Save your changes
4. Application automatically rebuilds and restarts
5. Test your changes in the UI

### Project Structure

```
rustbot/
├── Cargo.toml          # Dependencies and project config
├── src/
│   └── main.rs         # Main application entry point
├── docs/               # Documentation
│   ├── PRD/
│   │   └── development-plan.md
│   └── design/
│       └── ARCHITECTURE.md
└── README.md           # Project overview
```

## Current Status: POC Phase

This is currently a basic chat UI with mock responses. The real OpenRouter API integration is coming next.

### What Works Now
- ✅ Native egui window launches
- ✅ Chat input field
- ✅ Send button and Enter key support
- ✅ Message display with color-coded roles
- ✅ Auto-scroll to latest message
- ✅ Mock echo responses

### What's Coming Next
- 🔄 OpenRouter API integration
- 🔄 Real Claude streaming responses
- 🔄 Configuration management
- 🔄 Error handling

## Troubleshooting

### Application won't compile
```bash
# Clean build artifacts and rebuild
cargo clean
cargo build
```

### cargo-watch not found
```bash
# Install it
cargo install cargo-watch
```

### Window doesn't appear
- Check console output for errors
- Ensure you have graphics drivers installed
- Try running in release mode: `cargo run --release`

## Tips

- **Fast iteration:** Keep `cargo watch -x run` running in a terminal while you code
- **Check logs:** Application logs appear in the terminal where you ran the command
- **Force rebuild:** If hot-reload seems stuck, press Ctrl+C and restart `cargo watch`

## Next Steps

See `docs/PRD/development-plan.md` for the full development roadmap.
