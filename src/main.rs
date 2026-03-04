use std::process::Command;
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, Box as GtkBox, Button, Scale, Adjustment, Label, Orientation};

fn main() {
    let app = Application::builder()
        .application_id("com.github.waybar-headsetcontrol")
        .build();

    app.connect_activate(|app| {
        build_ui(app);
    });

    app.run();
}

fn build_ui(app: &Application) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Headset Control")
        .default_width(300)
        .default_height(150)
        .build();

    let vbox = GtkBox::new(Orientation::Vertical, 10);
    vbox.set_margin_top(10);
    vbox.set_margin_bottom(10);
    vbox.set_margin_start(10);
    vbox.set_margin_end(10);

    // Battery status label
    let battery_label = Label::new(None);
    update_battery_label(&battery_label);
    vbox.pack_start(&battery_label, false, false, 0);

    // Sidetone label
    let sidetone_label = Label::new(Some("Sidetone Level"));
    sidetone_label.set_halign(gtk::Align::Start);
    vbox.pack_start(&sidetone_label, false, false, 0);

    // Sidetone slider
    let adjustment = Adjustment::new(64.0, 0.0, 128.0, 1.0, 10.0, 0.0);
    let slider = Scale::new(Orientation::Horizontal, Some(&adjustment));
    slider.set_draw_value(true);
    slider.set_value_pos(gtk::PositionType::Right);
    slider.set_digits(0);

    slider.connect_value_changed(|scale| {
        let value = scale.value() as i32;
        let _ = Command::new("headsetcontrol")
            .arg("-s")
            .arg(value.to_string())
            .spawn();
    });

    vbox.pack_start(&slider, false, false, 0);

    // Close button
    let close_btn = Button::with_label("Close");
    close_btn.connect_clicked({
        let window = window.clone();
        move |_| {
            window.close();
        }
    });
    vbox.pack_start(&close_btn, false, false, 0);

    window.add(&vbox);
    window.show_all();
}

fn update_battery_label(label: &Label) {
    let output = Command::new("headsetcontrol")
        .arg("-b")
        .output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);

            if let Some(pos) = stdout.find("Level:") {
                let rest = &stdout[pos + 6..];
                let num_str: String = rest.chars()
                    .skip_while(|c| !c.is_digit(10))
                    .take_while(|c| c.is_digit(10))
                    .collect();

                if let Ok(level) = num_str.parse::<i32>() {
                    let icon = if level > 50 { "🟢" } else if level >= 15 { "🟡" } else { "🔴" };
                    label.set_text(&format!("{} Battery: {}%", icon, level));
                    return;
                }
            }

            label.set_text("❓ Battery: Unknown");
        }
        Err(_) => label.set_text("❌ Error reading battery"),
    }
}
