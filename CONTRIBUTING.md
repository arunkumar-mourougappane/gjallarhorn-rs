# Contributing to Gjallarhorn

Thank you for your interest in contributing to **Gjallarhorn**! We welcome contributions from everyone. By participating in this project, you help make it better for the entire community.

## Getting Started

### Prerequisites

To build and run Gjallarhorn, you need a standard Rust environment and a few system libraries.

1. **Rust**: Install the latest stable Rust toolchain via [rustup](https://rustup.rs/).
2. **System Libraries** (Linux):
    - `fontconfig` (for Slint font rendering)
    - `libxcb`, `libxkbcommon` (standard GUI deps)
    - *Optional*: NVIDIA Drivers (for GPU monitoring features via `nvml-wrapper`)

### Setup

Clone the repository:

```bash
git clone https://github.com/arunkumar-mourougappane/gjallarhorn-rs.git
cd gjallarhorn-rs
```

## Development Workflow

### Running the Application

During development, you can run the application in debug mode:

```bash
cargo run
```

**Note**: Debug builds may be slower, especially for chart rendering where thousands of SVG commands are generated. For performance testing, always use release mode:

```bash
cargo run --release
```

### Code Style & linting

We follow standard Rust community guidelines. Before submitting a PR, please ensure your code is formatted and linted.

1. **Formatting**:

    ```bash
    cargo fmt
    ```

2. **Linting**:

    ```bash
    cargo clippy
    ```

    Please address any warnings or errors reported by Clippy.

## Project Structure

- `src/`: Rust source code.
  - `src/main.rs`: Application entry point and logic.
  - `src/settings.rs`: Configuration and persistence logic.
  - `src/utils.rs`: Helper functions for color conversion and SVG generation.
- `ui/`: User Interface definitions.
  - `ui/appwindow.slint`: The main Slint UI file containing layout and styling.

## Submitting specific Changes

1. **Fork** the repository on GitHub.
2. Create a **new branch** for your feature or bug fix.
3. Commit your changes with clear, descriptive messages.
4. Push to your fork and submit a **Pull Request** (PR) to the `main` branch.
5. In your PR description, explain what changes you made and why.

## Reporting Bugs

If you find a bug, please create a GitHub Issue with:

- A description of the bug.
- Steps to reproduce it.
- Your operating system and environment details.
- (Optional) Any relevant logs or screenshots.

## License

By contributing, you agree that your contributions will be licensed under the project's **MIT License**.
