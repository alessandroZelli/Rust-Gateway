use regex::Regex;
use serialport::*;
use std::boxed::Box;
use std::time::Duration;

pub fn get_portname() -> String {
    let port_regex = Regex::new("/dev/ttyUSB*").unwrap();
    let portlist = serialport::available_ports().unwrap();
    let mut port_name_vector: Vec<&str> = vec![];
    let mut name = String::from("/dev/ttyUSB0");
    for i in portlist {
        if port_regex.is_match(i.port_name.as_str()) {
            return i.port_name;
        }
    }
    name
}

pub fn init_serial_port(port_name: String) -> Box<dyn SerialPort> {
    let s = serialport::SerialPortSettings {
        baud_rate: 9600,
        data_bits: DataBits::Eight,
        flow_control: FlowControl::None,
        parity: Parity::Odd,
        stop_bits: StopBits::Two,
        timeout: Duration::from_millis(200),
    };
    serialport::open_with_settings(&port_name, &s).unwrap()
}
