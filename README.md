<!-- cargo-sync-readme start -->

# Silicon Labs USB Xpress driver

This library port API from [SiUSBXp](http://www.etheus.net/SiUSBXp_Linux_Driver),
which is an open source port to SiUSBXp.dll, supplied with SiLabs USBXpress.
The underlying USB backend is libusb, which enable the cross platform
compilation.

# Usage

Add to your `Cargo.toml`:

``` toml
[dependencies]
silabs_usb_xpress = "0.1"
```

You must have `pkg-config` and `cc` available.

# Example
```rust, ignore

// get device count
let num = devices_count();

// print serial number for all devices
let if_sn = product_string(0, ProductStringType::SerialNumber);

// print VID for selected devices
let pst = ProductStringType::VID;
let if_vid = product_string(0, pst);

// get timeouts
let t = timeouts().unwrap();

// set timeouts
set_timeouts(Duration::from_millis(500), None).unwrap();

// open handle
let mut handle = open(0).unwrap();

// write to device handle
let v = vec![0x55, 0x80, 0x00, 0x01, 0x01, 0xAA];
write(&mut handle, &v);

// read 7 bytes from device handle
let read_res = read(&mut handle, 7);

// close device
close(handle.unwrap());
```

# License
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)

<!-- cargo-sync-readme end -->
