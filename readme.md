# set-gps-time

Sets the time of a system from a serial GPS device.

I wrote this for my dad (Greg Keys; AK4LK) so he could use a GPS
to set his system time on Mac OS. He previously had software that
could do this on Windows, but was disappointed that he couldn't
find anything for Mac OS.

He found a guide using GPSd that would work, but we ran into issues,
and GPSd seemed overkill for the task anyway, so I found some protocol
documentation and wrote this in a couple of hours.

## Supported systems

| OS      | Status      |
|---------|-------------|
| Mac OS  | Working     |
| Linux   | Untested    |
| Windows | Unsupported |

## Usage

```
Sets the system time from a serial GPS device

Usage: set-gps-time [OPTIONS] <GPS_DEVICE>

Arguments:
  <GPS_DEVICE>

Options:
  -r, --baud-rate <BAUD_RATE>
  -h, --help                   Print help
```

## Installation

Currently, the easiest way to build this tool is with rustup and cargo.
I will look into integrating with Brew and releasing a GUI, but it's a
CLI utility for now.

1. Install rustup. Instructions at https://rustup.rs/
2. In a terminal, from the project directory, run `cargo install --path .`
3. Run `set-gps-time --help` from a terminal to view usage instructions
