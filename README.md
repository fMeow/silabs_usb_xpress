<!-- cargo-sync-readme start -->

# Silicon Labs USB Xpress driver

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![Crates.io](https://img.shields.io/crates/v/silabs_usb_xpress.svg)](https://crates.io/crates/silabs_usb_xpress)
[![silabs_usb_xpress](https://docs.rs/silabs_usb_xpress/badge.svg)](https://docs.rs/silabs_usb_xpress)

| OS        | Status    |
| ----      | ----      |
| Linux     | [![Linux Build Status](https://github.com/fMeow/silabs_usb_xpress/workflows/CI%20%28Linux%29/badge.svg?branch=master)](https://github.com/fMeow/silabs_usb_xpress/actions?query=workflow%3A%22CI+%28Linux%29%22+branch%3Amaster)|
| Windows (MSVC) | [![Windows Build Status](https://github.com/fMeow/silabs_usb_xpress/workflows/CI%20%28Windows%29/badge.svg?branch=master)](https://github.com/fMeow/silabs_usb_xpress/actions?query=workflow%3A%22CI+%28Windows%29%22+branch%3Amaster)|

This library port API from [SiUSBXp](http://www.etheus.net/SiUSBXp_Linux_Driver),
which is an open source port to SiUSBXp.dll, supplied with SiLabs USBXpress.
The underlying USB backend is libusb, which enable the cross platform
compilation.

# Usage

Add to your `Cargo.toml`:

``` toml
[dependencies]
silabs_usb_xpress = "0.2"
```

This crate is compatible with Unix and Windows. For unix system,
`pkg-config` are required to link `libusb`. For windows, you must have [vcpkg](https://github.com/microsoft/vcpkg)
installed, hook up user-wide integration and install `libusb-win32` with it.

To pack a available driver in Windows, use [libusbk' inf wizard](https://osdn.net/projects/sfnet_libusb-win32/downloads/libusb-win32-releases/libusbK-inf-wizard.exe/).

# Example
```rust, ignore

// get device count
let num = devices_count();

// print serial number for selected devices
let if_sn = product_string(0, ProductStringType::SerialNumber);

// print VID for selected devices
let pst = ProductStringType::VID;
let if_vid = product_string(0, pst);

// get timeouts
let t = timeouts().unwrap();

// set timeouts
set_timeouts(Duration::from_millis(500), None).unwrap();

// open handle
let mut handle = SiHandle::open(0).unwrap();

// write to device handle
let v = vec![0x55, 0x80, 0x00, 0x01, 0x01, 0xAA];
handle.write(&v);

// read 7 bytes from device handle
let read_res = handle.read(7);

// close device
handle.close();
```

# License
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)

<!-- cargo-sync-readme end -->
