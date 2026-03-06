use serde::Deserialize;
use std::process::Command;
use waybar_cffi::{waybar_module, InitInfo, Module};
use waybar_cffi::gtk::prelude::*;
use waybar_cffi::gtk::{Label, EventBox, Container};
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
        let event_box_clone = event_box.clone();
        let container_clone = container.clone();
        glib::timeout_add_seconds_local(10, move || {
            update_battery(&label_clone, &event_box_clone, &container_clone);
            glib::ControlFlow::Continue
        });

        update_battery(&label, &event_box, &container);
        
        container.show_all();

        WbHeadsetControl
    }
}

fn update_battery(label: &Label, event_box: &EventBox, container: &Container) {
    let output = Command::new("headsetcontrol")
        .arg("-b")
        .output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            
            if stdout.contains("BATTERY_UNAVAILABLE") {
                container.remove(event_box);
                return;
            }
            
            // Make sure event_box is visible and in container
            event_box.show();
            
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
                label.set_markup(&format!("<span color='{}'></span> {}%", color, level));
                label.set_tooltip_text(Some(&format!("Battery: {}%", level)));
            } else {
                label.set_markup("<span color='gray'> ?</span>");
                label.set_tooltip_text(Some("Battery: Unknown"));
            }
        },
        Err(_) => {
            label.set_markup("<span color='red'> Err</span>");
            label.set_tooltip_text(Some("Battery: Error"));
        }
    }
}

waybar_module!(WbHeadsetControl);

#[derive(Deserialize)]
struct Config {}
