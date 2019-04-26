use nexstar::{Device, NexStar};

use serial::{Baud9600, Bits8, FlowNone, ParityNone, Stop1};
use serial_embedded_hal::{PortSettings, Serial};

//use nexstar::prelude::*;

fn main() {
    println!("Opening serial port...");

    let port_settings = PortSettings {
        baud_rate: Baud9600,
        char_size: Bits8,
        parity: ParityNone,
        stop_bits: Stop1,
        flow_control: FlowNone,
    };

    println!("Serial port open");

    let port = Serial::new("/dev/ttyUSB0", &port_settings).expect("Failed to open serial port");

    let (tx, rx) = port.split();

    let mut nexstar = NexStar::new(rx, tx);

    if let Ok(version) = nexstar.version() {
        println!("HC Version: {}.{}", version.major, version.minor);
    }

    print_version(&mut nexstar, "AZM/RA Motor", Device::AzmRaMotor);
    print_version(&mut nexstar, "ALT/DEC Motor", Device::AltDecMotor);
    print_version(&mut nexstar, "GPS Unit", Device::GPSUnit);
    print_version(&mut nexstar, "RTC", Device::RTC);

    if let Ok(model) = nexstar.model() {
        println!("Model: {:?}", model)
    }
}

fn print_version<T, U>(nexstar: &mut NexStar<T, U>, name: &str, device: Device)
where
    T: embedded_hal::serial::Read<u8>,
    U: embedded_hal::serial::Write<u8>,
{
    match nexstar.device_version(device) {
        Ok(version) => println!("{} Version: {}.{}", name, version.major, version.minor),
        Err(nexstar::Error::UnexpectedResponse) => println!("{} not present.", name),
        Err(_) => println!("Communication error"),
    }
}
