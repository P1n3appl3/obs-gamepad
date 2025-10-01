use std::collections::HashMap;

use color_eyre::eyre::{eyre, Report};
use gilrs_core::{AxisInfo, EvCode, Gilrs};
use log::error;

use crate::gamepad::{Backend, InputState, Inputs};

#[derive(Debug)]
pub struct UsbGamepad {
    handle: Gilrs,
    buttons: HashMap<EvCode, usize>,
    axes: HashMap<EvCode, AxisIndex>,
    device_id: usize,
}

#[derive(Copy, Clone, Debug)]
enum AxisIndex {
    Stick(usize, Xy),
    Single(usize),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Xy {
    X,
    Y,
}

impl UsbGamepad {
    fn load_mappings(&mut self, inputs: &Inputs) -> Option<()> {
        let g = self.handle.gamepad(self.device_id)?;
        self.buttons = inputs
            .buttons
            .iter()
            .enumerate()
            .filter_map(|(i, b)| {
                if let Some(&evcode) = g.buttons().get(b.id as usize) {
                    Some((evcode, i))
                } else {
                    error!("Couldn't find button {i}");
                    None
                }
            })
            .collect();

        self.axes.clear();
        for (i, a) in inputs.axes.iter().enumerate() {
            if let Some(&evcode) = g.axes().get(a.axis.id as usize) {
                self.axes.insert(evcode, AxisIndex::Single(i));
            } else {
                error!("Couldn't find axis {i}")
            }
        }
        for (i, s) in inputs.sticks.iter().enumerate() {
            if let Some(&evcode) = g.axes().get(s.x.id as usize) {
                self.axes.insert(evcode, AxisIndex::Stick(i, Xy::X));
            } else {
                error!("Couldn't find stick axis {i}")
            }
            if let Some(&evcode) = g.axes().get(s.y.id as usize) {
                self.axes.insert(evcode, AxisIndex::Stick(i, Xy::Y));
            } else {
                error!("Couldn't find stick axis {i}")
            }
        }
        Some(())
    }
}

impl Backend for UsbGamepad {
    type InitState = (Gilrs, usize);
    type Err = Report;

    fn init(
        (handle, device_id): Self::InitState,
        inputs: &Inputs,
    ) -> Result<Self, Self::Err> {
        let mut usb =
            Self { handle, buttons: HashMap::new(), axes: HashMap::new(), device_id };
        usb.load_mappings(inputs)
            .map(|_| usb)
            .ok_or(eyre!("couldn't open gamepad with index {device_id}"))
    }

    fn reload(&mut self, inputs: &Inputs) {
        self.load_mappings(inputs);
    }

    fn poll(&mut self, state: &mut InputState) -> bool {
        let mut modified = false;
        while let Some(ev) = self.handle.next_event() {
            let gamepad = self.handle.gamepad(self.device_id).unwrap();
            if ev.id != self.device_id {
                continue;
            }
            use gilrs_core::EventType::*;
            modified |= match ev.event {
                ev @ (ButtonPressed(code) | ButtonReleased(code)) => {
                    let new = matches!(ev, ButtonPressed(_));
                    if let Some(&i) = self.buttons.get(&code) {
                        state.buttons[i] = new;
                        true
                    } else {
                        false
                    }
                }
                AxisValueChanged(new, code) => {
                    if let Some(&idx) = self.axes.get(&code) {
                        let info = *gamepad.axis_info(code).unwrap();
                        match idx {
                            AxisIndex::Stick(i, xy) => {
                                let s = &mut state.sticks[i];
                                if xy == Xy::X {
                                    s.0 = normalized(new, info);
                                } else {
                                    s.1 = normalized(new, info);
                                };
                                true
                            }
                            AxisIndex::Single(i) => {
                                let percent = (normalized(new, info) + 1.0) / 2.0;
                                if state.axes[i] == percent {
                                    error!("they were the same???"); // TODO: remove
                                }
                                state.axes[i] = percent;
                                true
                            }
                        }
                    } else {
                        false
                    }
                }
                _ev @ (Connected | Disconnected) => {
                    // self.connected = ev == Connected;
                    true
                }
                _ => todo!(),
            }
        }
        modified
    }
}

fn remap(from: (f32, f32), to: (f32, f32), s: f32) -> f32 {
    to.0 + (s - from.0) * (to.1 - to.0) / (from.1 - from.0)
}

fn normalized(cur: i32, info: AxisInfo) -> f32 {
    remap((info.min as f32, info.max as f32), (-1.0, 1.0), cur as f32)
}

pub fn get_devices(gilrs: &Gilrs) -> HashMap<usize, String> {
    let mut devices = HashMap::new();
    let max_gamepads = gilrs.last_gamepad_hint();
    for (id, name) in
        (0..max_gamepads).filter_map(|i| gilrs.gamepad(i).map(|g| (i, g.name())))
    {
        devices.insert(id, name.to_owned());
    }
    devices
}
