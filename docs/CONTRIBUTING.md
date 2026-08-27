# Contributing to LavaXEmu

Thank you for your interest in contributing to LavaXEmu! This document
provides guidelines and information for contributors.

## Ways to Contribute

- **Bug reports**: File issues with clear reproduction steps
- **Feature requests**: Suggest new features or improvements
- **Code contributions**: Submit pull requests for bug fixes or new features
- **Documentation**: Improve or translate documentation
- **Game testing**: Test games and report compatibility results

## Development Setup

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (stable)
- Git

### Building

```bash
# Clone the repository
git clone https://github.com/AloysHF/LavaXEmu.git
cd LavaXEmu

# Build the standalone emulator
cargo build --release -p lavaxemu

# Build the libretro core
cargo build --release

# Run tests
cargo test --workspace
```

### Running

```bash
# Run the standalone emulator
cargo run --release -p lavaxemu -- path/to/game.lav

# Run in headless mode
cargo run --release -p lavaxemu -- path/to/game.lav --headless --frames 600
```

## Code Style

- Follow standard Rust formatting (`cargo fmt`)
- Ensure no warnings (`cargo clippy`)
- Write meaningful commit messages in English
- Add tests for new functionality
- Update documentation for user-facing changes

## Pull Request Process

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/your-feature`)
3. Make your changes
4. Run tests (`cargo test --workspace`)
5. Run linter (`cargo clippy`)
6. Commit your changes
7. Push to your fork
8. Open a pull request

## Reporting Bugs

When reporting bugs, include:

- Steps to reproduce
- Expected behavior
- Actual behavior
- System information (OS, Rust version)
- Game file name (if applicable)
- Error messages or logs

## Game Compatibility Testing

To help test game compatibility:

1. Obtain legal copies of LAV game files
2. Run the batch screenshot script:
   ```powershell
   pwsh scripts/batch-screenshots.ps1
   ```
3. Report results in the compatibility issue template
4. Include screenshots of working/non-working games

## Architecture

See [Architecture](architecture.md) for an overview of the codebase structure.

## License

By contributing, you agree that your contributions will be licensed under
the project's `GPL-2.0-or-later` license.

## Questions?

Feel free to open an issue for any questions about contributing.
