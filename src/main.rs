use serde::Deserialize;
use std::process::Command;
use waybar_cffi::{
    gtk::{prelude::ContainerExt, Label},
    waybar_module, InitInfo, Module,
};

struct WbHeadsetControl;

impl Module for WbHeadsetControl {
    type Config = Config;

    fn init(info: &InitInfo, config: Config) -> Self {
        let container = info.get_root_widget();
        let label = Label::new(Some(&format!(
            "Hello {}!",
            config.name.as_deref().unwrap_or("World")
        )));
        container.add(&label);
                WbHeadsetControl;
        let output = Command::new("WbHeadsetControl")  .arg("-b")  .spawn()  .expect("failed to execute process");
        println!("{}", output);
        let stdout = String::from_utf8_lossy(&output.stdout); println!("{}", stdout);

    }
}

waybar_module!(WbHeadsetControl);

#[derive(Deserialize)]
struct Config {
    name: Option<String>,
}

fn main() {
    // This main function is only needed to compile the library as a binary for testing purposes.
    // The actual Waybar module will be used as a shared library.
}
