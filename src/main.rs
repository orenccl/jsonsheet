use dioxus::prelude::*;
use jsonsheet::ui::app::App;

fn main() {
    dioxus::LaunchBuilder::desktop()
        .with_cfg(
            dioxus::desktop::Config::new().with_window(
                dioxus::desktop::WindowBuilder::new()
                    .with_title("JsonSheet")
                    .with_inner_size(dioxus::desktop::LogicalSize::new(1200.0, 800.0)),
            ),
        )
        .launch(App);
}
