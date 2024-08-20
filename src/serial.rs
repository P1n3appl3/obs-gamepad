use std::{
    io::{BufRead, BufReader},
    sync::atomic::AtomicU32,
    sync::atomic::Ordering::Relaxed,
    thread,
    time::Duration,
};

use serialport::{FlowControl, SerialPortType};

pub fn print_ports(start_at: usize) {
    if let Ok(ports) = serialport::available_ports() {
        for (id, port) in (start_at..).zip(&ports) {
            match &port.port_type {
                SerialPortType::UsbPort(i) => {
                    println!(
                        "{}: {} USB {:x}:{:x} {} {}",
                        id,
                        port.port_name,
                        i.vid,
                        i.pid,
                        i.manufacturer.clone().unwrap_or_default(),
                        i.product.clone().unwrap_or_default()
                    );
                }
                SerialPortType::Unknown => {
                    println!("{}: {} (???)", id, port.port_name)
                }
                _ => {}
            }
        }
    }
}

static LATEST: AtomicU32 = AtomicU32::new(0);

impl SerialGamepad {
    pub fn new(path: &str, rate: u32) -> Option<Self> {
        let port = serialport::new(path, rate)
            .flow_control(FlowControl::Hardware)
            .timeout(Duration::from_millis(500))
            .open()
            .map_err(|e| log::error!("Port '{}' not available: {}", &path, e))
            .ok()?;

        let thread = thread::spawn(|| {
            let mut reader = BufReader::new(port);
            let mut buf = vec![0u8; 32];
            while reader.read_until(b'\n', &mut buf).is_ok() {
                let mut temp = 0;
                for (i, &b) in buf.iter().enumerate() {
                    if b == b'1' {
                        temp |= 1 << i;
                    } else if b == b'\n' {
                        break;
                    }
                }
                LATEST.store(temp, Relaxed);
            }
        });
        Some(SerialGamepad { thread })
    }

    pub fn get_state() -> [bool; 32] {
        let cur = LATEST.load(Relaxed);
        let mut temp = [false; 32];
        (0..32).for_each(|i| temp[i] = cur & 1 << i != 0);
        temp
    }
}

#[derive(Debug)]
pub struct SerialGamepad {
    thread: thread::JoinHandle<()>,
}
