use std::time::Duration;

use silabs_usb_xpress::*;

fn main() {
    // get device count
    let num = devices_count();
    println!("{:?}", num);

    // print serial number for all devices
    for i in 0..num.unwrap() {
        let if_sn = product_string(i, ProductStringType::SerialNumber);
        println!("{:?}", if_sn);
    }

    // print VID for selected devices
    let pst = ProductStringType::VID;
    let if_vid = product_string(0, pst);
    println!("{:?}", if_vid);

    // get timeouts
    let t = timeouts().unwrap();
    println!("Timeout: {:?}", t);

    set_timeouts(Duration::from_millis(500), None).unwrap();

    // get timeouts after set
    let t = timeouts().unwrap();
    println!("Timeout: {:?}", t);

    // open handle
    let mut handle = UsbXpress::open(0).unwrap();
    println!("Open ok: {:?}", handle);

    // write to device handle
    let v = vec![0x55, 0x80, 0x00, 0x01, 0x01, 0xAA];
    let write_res = handle.write(&v);
    println!("{:?}", write_res);

    // read from device handle
    let read_res = handle.read(7);
    println!("{:?}", read_res);

    // close device
    let if_close = handle.close();
    println!("Close ok: {:?}", if_close);
}
