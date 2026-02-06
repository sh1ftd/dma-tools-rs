# DMA Tools RS

A Windows GUI tool for FPGA firmware flashing and DNA reading, supporting CH347 and RS232 interfaces.

## Features

- Firmware flashing for multiple FPGA boards:
  - 35T (CH347/RS232)
  - 75T (CH347/RS232)
  - 100T (CH347/RS232)
- Device DNA reading
- Real-time operation logging
- Progress tracking and status updates

## Multi-Language Support

The tool now supports five major languages:
- English
- Chinese (简体中文)
- German (Deutsch)
- Portuguese (Português)
- Arabic (العربية)

## Prerequisites

- Windows 10/11
- Appropriate USB Drivers (CH347/FTDI)
- DMA Hardware connected via JTAG port

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
6. Wait for the operation to complete (flashing takes a few minutes)

## Building from Source

```bash
git clone https://github.com/sh1ftd/dma-tools-rs.git
cd dma-tools-rs
cargo build --release
```


## Credits

Built with [Rust](https://www.rust-lang.org/), [egui](https://github.com/emilk/egui), and [OpenOCD](https://openocd.org/)
