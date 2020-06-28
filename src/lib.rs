//! # Silicon Labs USB Xpress driver
//!
//! [![Build Status](https://github.com/fMeow/silabs_usb_xpress/workflows/CI%20%28Linux%29/badge.svg?branch=master)](https://github.com/fMeow/silabs_usb_xpress/actions)
//! [![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
//! [![Crates.io](https://img.shields.io/crates/v/silabs_usb_xpress.svg)](https://crates.io/crates/silabs_usb_xpress)
//! [![silabs_usb_xpress](https://docs.rs/silabs_usb_xpress/badge.svg)](https://docs.rs/silabs_usb_xpress)
//!
//! This library port API from [SiUSBXp](http://www.etheus.net/SiUSBXp_Linux_Driver),
//! which is an open source port to SiUSBXp.dll, supplied with SiLabs USBXpress.
//! The underlying USB backend is libusb, which enable the cross platform
//! compilation.
//!
//! # Usage
//!
//! Add to your `Cargo.toml`:
//!
//! ``` toml
//! [dependencies]
//! silabs_usb_xpress = "0.2"
//! ```
//!
//! You must have `pkg-config` and `cc` available.
//!
//! # Example
//! ```rust, ignore
//! # use silabs_usb_xpress::{SiHandle, product_string, devices_count,
//! ProductStringType, timeouts, set_timeouts};
//! # use std::time::Duration;
//!
//! # fn main(){
//! // get device count
//! let num = devices_count();
//!
//! // print serial number for all devices
//! let if_sn = product_string(0, ProductStringType::SerialNumber);
//!
//! // print VID for selected devices
//! let pst = ProductStringType::VID;
//! let if_vid = product_string(0, pst);
//!
//! // get timeouts
//! let t = timeouts().unwrap();
//!
//! // set timeouts
//! set_timeouts(Duration::from_millis(500), None).unwrap();
//!
//! // open handle
//! let mut handle = SiHandle::open(0).unwrap();
//!
//! // write to device handle
//! let v = vec![0x55, 0x80, 0x00, 0x01, 0x01, 0xAA];
//! handle.write(&v);
//!
//! // read 7 bytes from device handle
//! let read_res = handle.read(7);
//!
//! // close device
//! handle.close();
//! # }
//! ```
//!
//! # License
//! [![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
use std::{error::Error, fmt, fmt::Formatter, mem::MaybeUninit, time::Duration};

use si_usb_xp::*;

#[allow(dead_code)]
mod si_usb_xp {
    include!("bindings.rs");
}

/// Returns the number of devices connected
///
/// This function returns the number of devices connected to the host.
///
/// - Supported Devices
///
/// C8051F320/1/6/7, C8051F340/1/2/3/4/5/6/7/8/9/A/B/C/D,
/// C8051F380/1/2/3/4/5/6/7, C8051T320/1/2/3/6/7, C8051T620/1/2/3,
/// CP2101/2/3/4/5/8/9/
pub fn devices_count() -> Result<usize, SilabsUsbXpressError> {
    let (status, num) = unsafe {
        let mut num = MaybeUninit::uninit();
        let status = SI_GetNumDevices(num.as_mut_ptr());
        (status, num.assume_init())
    };
    match status as u32 {
        SI_SUCCESS => Ok(num as usize),
        SI_DEVICE_NOT_FOUND => Err(SilabsUsbXpressError::DeviceNotFound),
        _ => unreachable!(),
    }
}

#[derive(Copy, Clone, Debug)]
pub enum ProductStringType {
    SerialNumber = 0,
    Description = 1,
    LinkName = 2,
    VID = 3,
    PID = 4,
}

/// Returns a descriptor for a device
///
/// This function returns a null terminated serial number (S/N) string or
/// product description string for the device specified by an index passed in
/// DeviceNum. The index for the first device is 0 and the last device is the
/// value returned by SI_GetNumDevices – 1.
///
/// - Supported Devices
///
/// C8051F320/1/6/7, C8051F340/1/2/3/4/5/6/7/8/9/A/B/C/D,
/// C8051F380/1/2/3/4/5/6/7, C8051T320/1/2/3/6/7, C8051T620/1/2/3,
/// CP2101/2/3/4/5/8/9
pub fn product_string(
    device_ix: usize,
    product_string_type: ProductStringType,
) -> Result<String, SilabsUsbXpressError> {
    let mut buffer: [i8; 256] = [0i8; 256];
    let status = unsafe {
        SI_GetProductString(
            device_ix as i32,
            buffer.as_mut_ptr(),
            product_string_type as i32,
        )
    };
    match status as u32 {
        SI_SUCCESS => {
            let mut string = String::from_utf8(buffer.iter().map(|&c| c as u8).collect())
                .unwrap()
                .trim_end_matches("\0")
                .to_owned();
            match product_string_type {
                ProductStringType::PID | ProductStringType::VID => {
                    if string.len() < 4 {
                        for _ in 0..4 - string.len() {
                            string.insert(0, '0');
                        }
                    }
                    Ok(string.to_uppercase())
                }
                _ => Ok(string),
            }
        }
        SI_DEVICE_NOT_FOUND => Err(SilabsUsbXpressError::DeviceNotFound),
        _ => unreachable!(),
    }
}

pub struct SiHandle {
    inner: *mut SiPrivate,
    device_ix: usize,
}

impl SiHandle {
    /// Opens a device and returns a handle
    ///
    /// Opens a device (using device number as returned by SI_GetNumDevices) and
    /// returns a handle which will be used for subsequent accesses.
    ///
    /// - Supported Devices
    ///
    /// C8051F320/1/6/7, C8051F340/1/2/3/4/5/6/7/8/9/A/B/C/D,
    /// C8051F380/1/2/3/4/5/6/7, C8051T320/1/2/3/6/7, C8051T620/1/2/3,
    /// CP2101/2/3/4/5/8/9
    pub fn open(device_ix: usize) -> Result<Self, SilabsUsbXpressError> {
        let mut handle: MaybeUninit<*mut SiPrivate> = MaybeUninit::uninit();
        let (status, handle) = unsafe {
            let status = SI_Open(device_ix as i32, handle.as_mut_ptr());
            (status, handle.assume_init())
        };
        match status as u32 {
            SI_SUCCESS => Ok(SiHandle {
                inner: handle,
                device_ix: device_ix,
            }),
            SI_INVALID_HANDLE => Err(SilabsUsbXpressError::InvalidSiHandle),
            SI_SYSTEM_ERROR_CODE => Err(SilabsUsbXpressError::SystemErrorCode),
            SI_GLOBAL_DATA_ERROR => Err(SilabsUsbXpressError::GlobalDataError),
            _ => unreachable!(),
        }
    }

    /// Cancels pending IO and closes a device
    ///
    /// Closes an open device using the handle provided by SI_Open and sets the
    /// handle to INVALID_HANDLE_VALUE.
    ///
    /// - Supported Devices
    ///
    /// C8051F320/1/6/7, C8051F340/1/2/3/4/5/6/7/8/9/A/B/C/D,
    /// C8051F380/1/2/3/4/5/6/7, C8051T320/1/2/3/6/7, C8051T620/1/2/3,
    /// CP2101/2/3/4/5/8/9
    pub fn close(self) -> Result<(), SilabsUsbXpressError> {
        let status = unsafe { SI_Close(self.inner) };
        match status as u32 {
            SI_SUCCESS => Ok(()),
            SI_INVALID_HANDLE => Err(SilabsUsbXpressError::InvalidSiHandle),
            SI_SYSTEM_ERROR_CODE => Err(SilabsUsbXpressError::SystemErrorCode),
            SI_GLOBAL_DATA_ERROR => Err(SilabsUsbXpressError::GlobalDataError),
            _ => unreachable!(),
        }
    }

    /// Reads a block of data from a device
    ///
    /// Reads the available number of bytes into the supplied buffer and
    /// retrieves the number of bytes that were read (this can be less than
    /// the number of bytes requested). This function returns synchronously
    /// if the overlapped object is set to NULL (this happens by default)
    /// but will not block system execution. If an initialized OVERLAPPED
    /// object is passed then the function returns immediately. If the read
    /// completed then the status will be SI_SUCCESS but if I/O is still
    /// pending then it will return STATUS_IO_PENDING. If STATUS_IO_PENDING
    /// is returned, the OVERLAPPED object can then be waited on using
    /// WaitForSingleObject(), and retrieve data or cancel using
    /// GetOverlappedResult() (as documented on MSDN by Microsoft) or
    /// SI_CancelIo() respectively. This functionality allows for multiple reads
    /// to be issued and waited on at a time. If any data is available when
    /// SI_Read is called it will return so check NumBytesReturned to
    /// determine if all requested data was returned. To make sure that
    /// SI_Read returns the requested number of bytes use SI_CheckRxQueue().
    ///
    /// - Supported Devices
    ///
    /// C8051F320/1/6/7, C8051F340/1/2/3/4/5/6/7/8/9/A/B/C/D,
    /// C8051F380/1/2/3/4/5/6/7, C8051T320/1/2/3/6/7, C8051T620/1/2/3,
    /// CP2101/2/3/4/5/8/9
    pub fn read(&mut self, bytes_to_read: usize) -> Result<Vec<u8>, SilabsUsbXpressError> {
        let mut buffer = Vec::with_capacity(bytes_to_read);
        // let mut buffer: [i8;256] = [0;256];
        let status = unsafe {
            let mut bytes_returned = MaybeUninit::uninit();
            let status = SI_Read(
                self.inner,
                buffer.as_mut_slice().as_mut_ptr(),
                bytes_to_read as i32,
                bytes_returned.as_mut_ptr(),
                MaybeUninit::uninit().as_mut_ptr(),
            );
            buffer.set_len(bytes_returned.assume_init() as usize);
            status
        };
        match status as u32 {
            SI_SUCCESS => Ok(buffer.iter().map(|&c| c as u8).collect()),
            SI_READ_ERROR => Err(SilabsUsbXpressError::ReadError),
            SI_INVALID_HANDLE => Err(SilabsUsbXpressError::InvalidSiHandle),
            SI_READ_TIMED_OUT => Err(SilabsUsbXpressError::ReadTimeOut),
            SI_IO_PENDING => Err(SilabsUsbXpressError::IoPending),
            SI_SYSTEM_ERROR_CODE => Err(SilabsUsbXpressError::SystemErrorCode),
            SI_INVALID_REQUEST_LENGTH => Err(SilabsUsbXpressError::InvalidRequestLength),
            SI_DEVICE_IO_FAILED => Err(SilabsUsbXpressError::DeviceIoFailed),
            _ => unreachable!(),
        }
    }

    /// Writes a block of data to a device
    ///
    /// On USB MCU devices, this function flushes both the receive buffer in the
    /// USBXpress device driver and the transmit buffer in the device.
    ///
    /// **Note**: Parameter 2 and 3 of `SI_Write` have no effect and any
    /// value can be passed when used with USB MCU devices.
    ///
    /// On CP210x devices, this function operates in accordance with parameters
    /// 2 and 3. If parameter 2 (FlushTransmit) is non-zero, the CP210x
    /// device’s UART transmit buffer is flushed. If parameter 3
    /// (FlushReceive) is non-zero, the CP210x device’s UART receive buffer
    /// is flushed. If parameters 2 and 3 are both non-zero, then both the
    /// CP210x device UART transmit buffer and UART receive buffer are
    /// flushed.
    ///
    /// - Supported Devices
    ///
    /// C8051F320/1/6/7, C8051F340/1/2/3/4/5/6/7/8/9/A/B/C/D,
    /// C8051F380/1/2/3/4/5/6/7, C8051T320/1/2/3/6/7, C8051T620/1/2/3,
    /// CP2101/2/3/4/5/8/9
    pub fn write(&mut self, to_write: &Vec<u8>) -> Result<usize, SilabsUsbXpressError> {
        let mut buffer: Vec<i8> = to_write.iter().map(|&c| c as i8).collect();
        let (status, bytes_written) = unsafe {
            let mut bytes_written = MaybeUninit::uninit();
            let status = SI_Write(
                self.inner,
                buffer.as_mut_ptr(),
                to_write.len() as i32,
                bytes_written.as_mut_ptr(),
                MaybeUninit::uninit().as_mut_ptr(),
            );
            (status, bytes_written.assume_init())
        };
        match status as u32 {
            SI_SUCCESS => Ok(bytes_written as usize),
            SI_WRITE_ERROR => Err(SilabsUsbXpressError::WriteError),
            SI_INVALID_REQUEST_LENGTH => Err(SilabsUsbXpressError::InvalidRequestLength),
            SI_INVALID_HANDLE => Err(SilabsUsbXpressError::InvalidSiHandle),
            SI_WRITE_TIMED_OUT => Err(SilabsUsbXpressError::WriteTimeOut),
            SI_IO_PENDING => Err(SilabsUsbXpressError::IoPending),
            SI_SYSTEM_ERROR_CODE => Err(SilabsUsbXpressError::SystemErrorCode),
            SI_DEVICE_IO_FAILED => Err(SilabsUsbXpressError::DeviceIoFailed),
            _ => unreachable!(),
        }
    }

    /// Allows sending low-level commands to the device driver
    ///
    /// Interface for any miscellaneous device control functions. A separate
    /// call to SI_DeviceIOControl is required for each input or output
    /// operation. A single call cannot be used to perform both an input and
    /// output operation simultaneously. Refer to DeviceIOControl function
    /// definition on MSDN Help for more details.
    ///
    /// - Supported Devices
    ///
    /// C8051F320/1/6/7, C8051F340/1/2/3/4/5/6/7/8/9/A/B/C/D,
    /// C8051F380/1/2/3/4/5/6/7, C8051T320/1/2/3/6/7, C8051T620/1/2/3
    pub fn device_io_control() {
        unimplemented!()
    }

    /// Flushes the TX and RX buffers for a device
    ///
    /// On USB MCU devices, this function flushes both the receive buffer in the
    /// USBXpress device driver and the transmit buffer in the device.
    ///
    /// **Note**: Parameter 2 and 3 of `SI_FlushBuffers` have no effect and any
    /// value can be passed when used with USB MCU devices.
    ///
    /// On CP210x devices, this function operates in accordance with parameters
    /// 2 and 3. If parameter 2 (FlushTransmit) is non-zero, the CP210x
    /// device’s UART transmit buffer is flushed. If parameter 3
    /// (FlushReceive) is non-zero, the CP210x device’s UART receive buffer
    /// is flushed. If parameters 2 and 3 are both non-zero, then both the
    /// CP210x device UART transmit buffer and UART receive buffer are
    /// flushed.
    ///
    /// - Supported Devices
    ///
    /// C8051F320/1/6/7, C8051F340/1/2/3/4/5/6/7/8/9/A/B/C/D,
    /// C8051F380/1/2/3/4/5/6/7, C8051T320/1/2/3/6/7, C8051T620/1/2/3,
    /// CP2101/2/3/4/5/8/9
    pub fn flush_buffers(&mut self) -> Result<(), SilabsUsbXpressError> {
        let status = unsafe { SI_FlushBuffers(self.inner, 1i8, 1i8) };
        match status as u32 {
            SI_SUCCESS => Ok(()),
            SI_INVALID_HANDLE => Err(SilabsUsbXpressError::InvalidSiHandle),
            SI_SYSTEM_ERROR_CODE => Err(SilabsUsbXpressError::SystemErrorCode),
            _ => unreachable!(),
        }
    }

    /// Returns the number of bytes in a device's RX queue
    ///
    /// Returns the number of bytes in the receive queue and a status value that
    /// indicates if an overrun (SI_QUEUE_OVERRUN) has occurred and if the RX
    /// queue is ready (SI_QUEUE_READY) for reading. Upon indication of an
    /// Overrun condition it is recommended that data transfer be stopped
    /// and all buffers be flushed using the SI_FlushBuffers command.
    pub fn check_rx_queue(&mut self) -> Result<(usize, usize), SilabsUsbXpressError> {
        let (status, num_bytes_in_queue, queue_status) = unsafe {
            let mut num_bytes_in_queue = MaybeUninit::uninit();
            let mut queue_status = MaybeUninit::uninit();
            let status = SI_CheckRXQueue(
                self.inner,
                num_bytes_in_queue.as_mut_ptr(),
                queue_status.as_mut_ptr(),
            );
            (
                status,
                num_bytes_in_queue.assume_init(),
                queue_status.assume_init(),
            )
        };
        match status as u32 {
            SI_SUCCESS => Ok((num_bytes_in_queue as usize, queue_status as usize)),
            SI_INVALID_HANDLE => Err(SilabsUsbXpressError::InvalidSiHandle),
            SI_DEVICE_IO_FAILED => Err(SilabsUsbXpressError::DeviceIoFailed),
            _ => unreachable!(),
        }
    }
}

impl fmt::Debug for SiHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SiHandle")
            .field("device_ix", &self.device_ix)
            .finish()
    }
}

/// Sets read and write block timeouts
///
/// Sets the read and write timeouts. Timeouts are used for SI_Read and SI_Write
/// when called synchronously (OVERLAPPED* o is set to NULL). The default value
/// for timeouts is 1000ms.
///
/// - Supported Devices
///
/// C8051F320/1/6/7, C8051F340/1/2/3/4/5/6/7/8/9/A/B/C/D,
/// C8051F380/1/2/3/4/5/6/7, C8051T320/1/2/3/6/7, C8051T620/1/2/3,
/// CP2101/2/3/4/5/8/9
pub fn set_timeouts<R: Into<Option<Duration>>, W: Into<Option<Duration>>>(
    read: R,
    write: W,
) -> Result<(), SilabsUsbXpressError> {
    let status = unsafe {
        SI_SetTimeouts(
            read.into().unwrap_or(Duration::from_secs(1)).as_millis() as i32,
            write.into().unwrap_or(Duration::from_secs(1)).as_millis() as i32,
        )
    };

    match status as u32 {
        SI_SUCCESS => Ok(()),
        SI_DEVICE_IO_FAILED => Err(SilabsUsbXpressError::DeviceIoFailed),
        _ => unreachable!(),
    }
}

#[derive(Debug)]
pub struct Timeout {
    read: Duration,
    write: Duration,
}

impl Timeout {
    pub fn read_timeout(&self) -> Duration {
        self.read
    }
    pub fn write_timeout(&self) -> Duration {
        self.write
    }
}

/// Gets read and write block timeouts
///
/// Returns the current read and write timeouts. If a timeout value is None in
/// Rust, it has been set to wait 1000ms; otherwise the timeouts are specified
/// in milliseconds.
///
/// - Supported Devices
///
/// C8051F320/1/6/7, C8051F340/1/2/3/4/5/6/7/8/9/A/B/C/D,
/// C8051F380/1/2/3/4/5/6/7, C8051T320/1/2/3/6/7, C8051T620/1/2/3,
/// CP2101/2/3/4/5/8/9
pub fn timeouts() -> Result<Timeout, SilabsUsbXpressError> {
    let (status, read, write) = unsafe {
        let mut read = MaybeUninit::uninit();
        let mut write = MaybeUninit::uninit();
        let status = SI_GetTimeouts(read.as_mut_ptr(), write.as_mut_ptr());
        (status, read.assume_init(), write.assume_init())
    };

    match status as u32 {
        SI_SUCCESS => Ok(Timeout {
            read: Duration::from_millis(read as u64),
            write: Duration::from_millis(write as u64),
        }),
        SI_DEVICE_IO_FAILED => Err(SilabsUsbXpressError::DeviceIoFailed),
        _ => unreachable!(),
    }
}

#[derive(Debug)]
pub enum SilabsUsbXpressError {
    ConnectionError,
    InvalidSiHandle,
    DeviceNotFound,
    SystemErrorCode,
    GlobalDataError,
    ReadError,
    ReadTimeOut,
    IoPending,
    InvalidRequestLength,
    DeviceIoFailed,
    WriteError,
    WriteTimeOut,
}

impl fmt::Display for SilabsUsbXpressError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&format!("{:?}", self))
    }
}

impl Error for SilabsUsbXpressError {}
