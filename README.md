# DMA Tools RS

A Windows GUI tool for FPGA firmware flashing and DNA reading, supporting CH347 and RS232 interfaces.

## Features

- Firmware flashing for multiple FPGA boards:
  - 35T (CH347/RS232)
  - 75T (CH347/RS232)
  - Stark100T (CH347)
  - 100T (RS232)
- Device DNA reading
- Real-time operation logging
- Progress tracking and status updates

## Installation

1. Download latest release
2. Extract and run as administrator
3. Required files will be checked on first run

## Quick Start

1. Connect DMA card to FPGA via JTAG
2. Place firmware (.bin) in executable directory
3. Launch as administrator
4. Select operation (Flash/DNA Read)
5. Choose interface (CH347/RS232)
6. Monitor progress

## Building from Source

```bash
git clone https://github.com/sh1ftd/dma-tools-rs.git
cd dma-tools-rs
cargo build --release
```

## Requirements

- Windows 64-bit
- CH347 or RS232 interface
- Administrator privileges

## Credits

Built with [Rust](https://www.rust-lang.org/), [egui](https://github.com/emilk/egui), and [OpenOCD](https://openocd.org/)
