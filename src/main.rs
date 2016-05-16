// Temper client in Rust
//
// Copyright (C) 2015 Felix Obenhuber <felix@obenhuber.de>
//
// based on pcsensor.c by Michitaka Ohno and updatede to deal with negative temperatures
// using changes presented by Torbjørn Hergum in the temper1
// pcsensor.c by Michitaka Ohno (c) 2011 (elpeo@mars.dti.ne.jp)
// based oc pcsensor.c by Juan Carlos Perez (c) 2011 (cray@isp-sl.com)
// based on Temper.c by Robert Kavaler (c) 2009 (relavak.com)
// All rights reserved.
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions are met:
//     * Redistributions of source code must retain the above copyright
//       notice, this list of conditions and the following disclaimer.
//
// THIS SOFTWARE IS PROVIDED BY Felix Obenhuber ''AS IS'' AND ANY
// EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
// WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
// DISCLAIMED. IN NO EVENT SHALL Robert kavaler BE LIABLE FOR ANY
// DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES
// (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES;
// LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND
// ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
// (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
// SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
//

extern crate libusb;

use std::process;
use std::time::Duration;

const VID: u16 = 0x0c45;
const PID: u16 = 0x7401;

const INTERFACES: &'static [u8] = &[0x00, 0x01];
const TEMP_CONTROL: &'static [u8] = &[0x01, 0x80, 0x33, 0x01, 0x00, 0x00, 0x00, 0x00];

fn main() {
    match libusb::Context::new() {
        Ok(mut context) => {
            match open_device(&mut context, VID, PID) {
                Some((_, _, mut handle)) => {
                    match read_temperature(&mut handle) {
                        Some(t) => {
                            println!("{}", t);
                            process::exit(0)
                        }
                        None => {}
                    };
                }
                None => {
                    println!("could not find device {:04x}:{:04x}", VID, PID);
                    process::exit(1)
                }
            }
        }
        Err(e) => panic!("could not initialize libusb: {}", e),
    }
    process::exit(1)
}

fn open_device(context: &mut libusb::Context,
               vid: u16,
               pid: u16)
               -> Option<(libusb::Device, libusb::DeviceDescriptor, libusb::DeviceHandle)> {
    let devices = match context.devices() {
        Ok(d) => d,
        Err(_) => return None,
    };

    for mut device in devices.iter() {
        let device_desc = match device.device_descriptor() {
            Ok(d) => d,
            Err(_) => continue,
        };

        if device_desc.vendor_id() == vid && device_desc.product_id() == pid {
            match device.open() {
                Ok(handle) => return Some((device, device_desc, handle)),
                Err(_) => continue,
            }
        }
    }

    None
}

fn control_transfer(handle: &mut libusb::DeviceHandle,
                    request_type: u8,
                    request: u8,
                    value: u16,
                    index: u16,
                    buf: &[u8]) {
    match handle.write_control(request_type,
                               request,
                               value,
                               index,
                               buf,
                               Duration::from_secs(1)) {
        Ok(_) => {}
        Err(e) => panic!("could not write control: {}", e),
    }
}

fn read_interrupt(handle: &mut libusb::DeviceHandle, buf: &mut [u8]) {
    match handle.read_interrupt(0x82, buf, Duration::from_secs(1)) {
        Ok(_) => {}
        Err(err) => panic!("failed to read interrupt: {}", err),
    }
}

fn read_temperature(handle: &mut libusb::DeviceHandle) -> Option<f32> {
    match handle.reset() {
        Ok(_) => {}
        Err(e) => panic!("failed to reset device: {}", e),
    }

    for i in INTERFACES {
        let interface = *i;
        match handle.kernel_driver_active(interface) {
            Ok(true) => {
                match handle.detach_kernel_driver(interface) {
                    Ok(_) => {}
                    Err(e) => panic!("failed to detach kernel driver{}: {}", interface, e),
                }
            }
            Ok(false) => {}
            Err(e) => panic!("failed to query kernel driver state {}: {}", interface, e),
        }
    }

    match handle.set_active_configuration(0x01) {
        Ok(_) => {}
        Err(e) => panic!("could not set active confifguration: {}", e),
    }

    for i in INTERFACES {
        let interface = *i;
        match handle.claim_interface(interface) {
            Ok(_) => {}
            Err(e) => panic!("failed to claim interface {}: {}", interface, e),
        }
    }

    let buffer = &mut [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];

    control_transfer(handle, 0x21, 0x09, 0x0201, 0x00, &[0x01, 0x01]);

    control_transfer(handle, 0x21, 0x09, 0x0200, 0x01, TEMP_CONTROL);
    read_interrupt(handle, buffer);

    control_transfer(handle,
                     0x21,
                     0x09,
                     0x0200,
                     0x01,
                     &[0x01, 0x82, 0x77, 0x01, 0x00, 0x00, 0x00, 0x00]);
    read_interrupt(handle, buffer);

    control_transfer(handle,
                     0x21,
                     0x09,
                     0x0200,
                     0x01,
                     &[0x01, 0x86, 0xff, 0x01, 0x00, 0x00, 0x00, 0x00]);
    read_interrupt(handle, buffer);
    read_interrupt(handle, buffer);

    control_transfer(handle, 0x21, 0x09, 0x0200, 0x01, TEMP_CONTROL);
    read_interrupt(handle, buffer);

    if buffer.len() >= 4 {
        let b2 = buffer[2] as u16;
        let b3 = buffer[3] as u16;
        let mut temperature = ((b3 & 0x00FF) + (b2 << 8)) as i32;

        // msb means the temperature is negative -- less than 0 Celsius -- and in 2'complement form.
        // We can't be sure that the host uses 2's complement to store negative numbers
        // so if the temperature is negative, we 'manually' get its magnitude
        // by explicity getting it's 2's complement and then we return the negative of that.
        //
        if (b2 & 0x80) != 0 {
            // return the negative of magnitude of the temperature
            temperature = -((temperature ^ 0xffff) + 1);
        }

        Some((temperature as f32) * 125.0 / 32000.0)
    } else {
        None
    }
}
