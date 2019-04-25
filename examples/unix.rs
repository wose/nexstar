use nexstar::{NexStar, Version};

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
        println!("Version: {}.{}", version.major, version.minor);
    }
}
