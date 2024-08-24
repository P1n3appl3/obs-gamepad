use std::{
    collections::HashMap,
    io::{BufRead, BufReader},
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
    time::Duration,
};

use log::{error, trace};
use serialport::{FlowControl, SerialPortType};

use crate::gamepad::{Backend, InputState, Inputs};

#[derive(Debug)]
pub struct Haybox {
    ids: Vec<u8>,
    rx: Receiver<u64>,
}

impl Backend for Haybox {
    type InitState = (String, u32);
    type Err = ();

    fn init((path, rate): Self::InitState, inputs: &Inputs) -> Result<Self, Self::Err> {
        let port = serialport::new(&path, rate)
            .flow_control(FlowControl::Hardware)
            .timeout(Duration::from_millis(100))
            .open()
            .map_err(|e| log::error!("Port '{}' not available: {}", &path, e))?;

        let ids = inputs.buttons.iter().map(|b| b.id).collect();
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            let mut buf = vec![0u8; 28];
            let mut reader = BufReader::new(port);
            while reader.read_until(b'\n', &mut buf).is_ok() {
                let mut temp: u64 = 0;
                for (i, &b) in buf.iter().enumerate() {
                    if b == b'1' {
                        temp |= 1 << i;
                    } else if b == b'\n' {
                        break;
                    }
                }
                if tx.send(temp).is_err() {
                    break;
                }
                buf.clear()
            }
        });
        Ok(Haybox { ids, rx })
    }

    fn poll(&mut self, state: &mut InputState) -> bool {
        let mut changed = false;
        let mut latest = match self.rx.try_recv() {
            Ok(n) => n,
            Err(e) => {
                if e == TryRecvError::Disconnected {
                    error!("Serial reader thread died");
                }
                return false;
            }
        };
        let mut i = 0;
        while let Ok(n) = self.rx.try_recv() {
            latest = n;
            i += 1;
        }
        trace!("found {i} extra states");

        for (i, n) in self.ids.iter().enumerate() {
            let new = latest & 1 << n != 0;
            let old = state.buttons[i];
            state.buttons[i] = new;
            changed |= new != old;
        }
        changed
    }

    fn reload(&mut self, inputs: &Inputs) {
        self.ids = inputs.buttons.iter().map(|b| b.id).collect();
    }
}

/// returns name and description of all known serial ports
pub fn get_ports() -> HashMap<String, String> {
    let mut all_ports = HashMap::new();
    if let Ok(ports) = serialport::available_ports() {
        for port in &ports {
            match &port.port_type {
                SerialPortType::UsbPort(i) => {
                    all_ports.insert(
                        port.port_name.clone(),
                        format!(
                            "USB {:04x}:{:04x} {} {}",
                            i.vid,
                            i.pid,
                            i.manufacturer.clone().unwrap_or_default(),
                            i.product.clone().unwrap_or_default()
                        ),
                    );
                }
                SerialPortType::Unknown => {
                    all_ports.insert(port.port_name.clone(), "(???)".to_owned());
                }
                _ => {}
            }
        }
    }
    all_ports
}
