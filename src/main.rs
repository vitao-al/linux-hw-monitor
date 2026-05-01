use gio::prelude::ApplicationExtManual;
use linux_hw_monitor::application::build_application;

fn main() {
    let app = build_application();
    app.run();
}
