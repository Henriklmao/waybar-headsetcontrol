use serde::Deserialize;
use std::process::Command;
use waybar_cffi::{waybar_module, InitInfo, Module};
use gtk::prelude::*;
use gtk::{Label, EventBox};
use glib;

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

        // Click handler
        event_box.connect_button_press_event(|_, _| {
            let _ = Command::new("bash")
                .arg("-c")
                .arg("val=$(yad --scale --min-value=0 --max-value=127 --step=1 --title='Headset Sidetone' --text='Adjust Sidetone Level' --width=300 --button='Set:0' --button='Cancel:1'); if [ $? -eq 0 ] && [ -n \"$val\" ]; then waybar-headsetcontrol-cmd -s \"$val\"; fi")
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
