use serde::Deserialize;
use std::process::Command;
use waybar_cffi::{waybar_module, InitInfo, Module};
use waybar_cffi::gtk::prelude::*;
use waybar_cffi::gtk::{Label, EventBox};
use waybar_cffi::gtk::glib;

struct WbHeadsetControl;

impl Module for WbHeadsetControl {
    type Config = Config;

    fn init(info: &InitInfo, _config: Config) -> Self {
        let container = info.get_root_widget();
        
        let event_box = EventBox::new();
        let label = Label::new(Some("..."));
        label.set_use_markup(true);
        event_box.add(&label);
        container.add(&event_box);

        // Click handler - open TUI
        event_box.connect_button_press_event(|_, _| {
            let _ = Command::new("wb-headset-tui")
                .spawn();
            false.into()
        });

        let label_clone = label.clone();
        glib::timeout_add_seconds_local(10, move || {
            update_battery(&label_clone);
            glib::ControlFlow::Continue
        });

        update_battery(&label);
        
        container.show_all();

        WbHeadsetControl
    }
}

fn update_battery(label: &Label) {
    let output = Command::new("waybar-headsetcontrol-cmd")
        .arg("-b")
        .output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            
            let mut level = -1;
            if stdout.contains("Charging") {
                level = 100; 
            } else {
                if let Some(pos) = stdout.find("Level:") {
                    let rest = &stdout[pos + 6..];
                    let num_str: String = rest.chars()
                        .skip_while(|c| !c.is_digit(10))
                        .take_while(|c| c.is_digit(10))
                        .collect();
                    if let Ok(l) = num_str.parse::<i32>() {
                        level = l;
                    }
                }
            }

            if level != -1 {
                let color = if level > 50 { "green" } else if level >= 15 { "yellow" } else { "red" };
                label.set_markup(&format!("<span color='{}'></span> {}%", color, level));
            } else {
                label.set_markup("<span color='gray'> ?</span>");
            }
        },
        Err(_) => {
            label.set_markup("<span color='red'> Err</span>");
        }
    }
}

waybar_module!(WbHeadsetControl);

#[derive(Deserialize)]
struct Config {}
