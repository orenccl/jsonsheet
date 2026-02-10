use dioxus::prelude::*;
use std::path::PathBuf;

use crate::io::json_io;
use crate::state::data_model::TableData;
use crate::ui::table::Table;
use crate::ui::toolbar::Toolbar;

const STYLES: Asset = asset!("/assets/styles.css");

#[component]
pub fn App() -> Element {
    let data = use_signal(TableData::new);
    let file_path = use_signal::<Option<PathBuf>>(|| None);
    let error_message = use_signal::<Option<String>>(|| None);
    let selected_row = use_signal::<Option<usize>>(|| None);
    let selected_column = use_signal::<Option<String>>(|| None);

    use_effect({
        let mut data = data;
        let mut file_path = file_path;
        let mut error_message = error_message;
        move || {
            if let Ok(path) = std::env::var("JSONSHEET_OPEN") {
                let path = PathBuf::from(path);
                match json_io::load_json(&path) {
                    Ok(rows) => {
                        data.set(rows);
                        file_path.set(Some(path));
                        error_message.set(None);
                    }
                    Err(e) => {
                        error_message.set(Some(e.to_string()));
                    }
                }
            }
        }
    });

    rsx! {
        document::Stylesheet { href: STYLES }
        div { class: "app",
            Toolbar { data, file_path, error_message, selected_row, selected_column }
            Table { data, selected_row, selected_column }
        }
    }
}
