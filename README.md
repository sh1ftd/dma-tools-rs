# dma-tools-rs

Windows desktop utility for programming SPI flash on Xilinx Artix-7 FPGAs (35T / 75T / 100T devices) attached over JTAG from a DMA setup. The UI orchestrates bundled [OpenOCD](https://openocd.org/) builds (`openocd-347.exe` for WCH CH347, `openocd.exe` for FTDI-based RS232) with project-specific TAP/flash and DNA scripts, progress parsing, and driver helpers.

Licensed under AGPL-3.0 (see `LICENSE`).

## Capabilities

- **Bitstream programming**: Copies the selected `.bin` to a fixed temp name and runs OpenOCD with the matching `xc7a*T` flash config (`CH347` or `RS232_*` variants).
- **Device DNA read**: Runs DNA init configs; surfaces parsed values in the UI. CH347 DNA read shares one profile for all three densities; RS232 uses density-specific configs.
- **Runtime validation**: On startup the app verifies the expected `OpenOCD/` and `tools/` payload next to the executable (executables, Cygwin/USB DLLs, `bit/` bitstreams, `flash/` and `DNA/` TCL/CFG fragments, bundled driver installers, and `memflow-base`).
- **Driver menu**: Elevated FTDI INF install (`pnputil`), Zadig launch, CH347 installer—each expects the bundled paths under `tools\`.
- **PCILeech check**: Runs `tools\memflow-base\memflow-base.exe -c pcileech --headless` and interprets stdout for a sanity signal (targets the same DMA toolchain context as flashing).
- **Localization**: Built-in strings for English, Chinese (Simplified), German, Portuguese, and Arabic (with shaping/bidi handled in-app).
- **Optional branding build**: `cargo build --release --features branding` for alternate window title/icon (see `src/branding/`).

## Requirements

- Windows 10 or 11, x86-64.
- Administrator elevation when installing or replacing drivers from the Drivers screen.
- CH347 USB driver or FTDI/VCP stack as appropriate for the selected transport; JTAG adapters must match the chosen OpenOCD profile.
- Full **release distribution**: the binary alone is insufficient; keep the `OpenOCD\` and `tools\` directory trees identical to what the embedded file checklist expects.

## Layout (runtime)

Deploy (or extract) so the executable sits beside:

- `OpenOCD\` — `openocd-347.exe`, `openocd.exe`, required DLLs, `bit\bscan_spi_xc7a*.bit`, `flash\*.cfg`, `DNA\init_*.cfg`.
- `tools\` — `zadig-2.9.exe`, `CH341PAR_USB_DRIVER.EXE`, `FTDIBUS3\ftdibus3.Inf`, `memflow-base\memflow-base.exe`.

Firmware images must use the `.bin` extension. The scanner searches the executable directory, current working directory, and a few conventional subfolders (`resources`, `bin`, `firmware`, `fw`); duplicates are collapsed by filename. An optional setting can delete the original `.bin` after a successful flash.

## Usage (summary)

1. Connect JTAG between the adapter (CH347 or FTDI path) and the target FPGA.
2. Ensure drivers match the adapter and selected mode.
3. Run the executable; resolve any reported missing-bundle files before continuing.
4. Choose flash vs DNA read, then the density and transport (CH347 vs RS232).
5. For programming, pick the `.bin`; monitor the log and completion state. Typical programming runs several minutes depending on image size and link quality.

## Building from source

Requires a recent Rust toolchain with **Edition 2024** support.

```bash
git clone https://github.com/sh1ftd/dma-tools-rs.git
cd dma-tools-rs
cargo build --release
```

`target\release\dma-tools-rs.exe` still needs the same `OpenOCD\` and `tools\` trees at runtime. Debug builds resolve some paths under `target\debug` and the repo root for development convenience.

Optional branding:

```bash
cargo build --release --features branding
```

## Stack

Rust, [`eframe`](https://github.com/emilk/egui) / [`egui`](https://github.com/emilk/egui) with the `wgpu` backend, bundled OpenOCD builds and scripts maintained alongside this repository.
