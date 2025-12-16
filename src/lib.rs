use serde::Deserialize;
use waybar_cffi::{
    gtk::{prelude::ContainerExt, Label},
    waybar_module, InitInfo, Module,
};

struct wbHeadsetControl;

impl Module for wbHeadsetControl {
    type Config = Config;

    fn init(info: &InitInfo, config: Config) -> Self {
        let container = info.get_root_widget();
        let label = Label::new(Some(&format!(
            "Hello {}!",
            config.name.as_deref().unwrap_or("World")
        )));
        container.add(&label);

        wbHeadsetControl
    }
}

waybar_module!(wbHeadsetControl);

#[derive(Deserialize)]
struct Config {
    name: Option<String>,
}
