# J1

High-performance, minimal-overhead web browser built on the WebView2 runtime with a custom chrome bar for navigation.

## Overview

High-performance, minimal-overhead web browser built on the WebView2 runtime with a custom chrome bar for navigation.

## Features

- Dual WebView2 architecture: chrome bar + content area
- Custom HTML/CSS navigation bar with back, forward, reload, home
- URL bar with DuckDuckGo search integration
- DPI-aware window with proper HiDPI rendering
- Keyboard shortcuts: Ctrl+L (URL focus), F5 (reload), Escape (stop)
- Minimal memory footprint with no framework overhead

## Installation

### Prerequisites

- Rust
- Git

### Steps

```
git clone https://github.com/judeako402-dotcom/J1.git
cd J1
cargo build --release
./target/release/j1.exe
```

## Usage

See the project documentation for detailed usage instructions.

## Use Cases

- Embedded browser component for kiosk or dedicated display systems
- Lightweight alternative to full browsers for single-purpose browsing
- Reference implementation for WebView2 integration in Rust

## License

MIT