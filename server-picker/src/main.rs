mod app;
mod server_picker;

use app::App;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("CS2 Server Picker")
            .with_inner_size([480.0, 560.0])
            .with_resizable(true),
        ..Default::default()
    };

    eframe::run_native(
        "CS2 Server Picker",
        native_options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}
