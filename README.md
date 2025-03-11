# DMA Tools RS

A Windows GUI tool written in Rust for flashing firmware and reading device DNA from FPGA boards using CH347 and RS232 interfaces.

## Why

This tool provides a user-friendly interface for common FPGA operations like firmware flashing (burning) and DNA reading.

This tool aims to:

- Simplify the firmware flashing process with an intuitive GUI
- Provide clear visual feedback during operations
- Handle common errors and edge cases automatically
- Support multiple interface types (CH347 and RS232)
- Monitor and validate operations to prevent hardware issues

## Features

- Firmware flashing support for multiple FPGA boards:
  - 35T boards (CH347 and RS232)
  - 75T boards (CH347 and RS232)
  - Stark100T boards (CH347)
- Device DNA reading capabilities
- Real-time operation logging
- Automatic file integrity checking
- Operation monitoring with safety checks
- Progress tracking with detailed status updates

## Controls

The tool provides a simple interface with:

- File selection for firmware
- Operation type selection (Flash/DNA Read)
- Interface selection (CH347/RS232)
- Clear operation status and progress indicators
- Detailed logging of all operations

## Installation

1. Download the latest release from the releases page
2. Extract the archive
3. Run the executable as administrator
4. Required files will be checked automatically on first run

## Usage

1. Connect your DMA card to the FPGA board via the JTAG port
2. Place your firmware (.bin) file in the same folder as the executable
3. Launch the application as administrator
4. Select your operation:
   - For flashing: Choose your .bin file and select "Flash"
   - For DNA reading: Select "Read DNA"
5. Choose your interface type (CH347 or RS232)
6. Monitor the progress in the status window

**Note:** Administrator privileges are required for proper hardware access.

## Building from Source

```bash
git clone https://github.com/sh1ftd/dma-tools-rs.git
```

```bash
cd dma-tools-rs
```

```bash
cargo build --release
```

## Requirements

- Windows OS (64-bit)
- CH347 or RS232 interface hardware
- Appropriate drivers installed for your interface

## License

AGPL-3.0 License

## Acknowledgments

- Built with [Rust](https://www.rust-lang.org/) and [egui](https://github.com/emilk/egui)
- Uses [OpenOCD](https://openocd.org/) for FPGA operations (CH347)
