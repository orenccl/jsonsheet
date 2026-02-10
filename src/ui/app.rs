use dioxus::prelude::*;
use std::path::PathBuf;

use crate::io::json_io;
use crate::state::i18n::Language;
use crate::state::table_state::TableState;
use crate::ui::table::Table;
use crate::ui::toolbar::Toolbar;

const STYLESHEET: &str = include_str!("../../assets/styles.css");

#[component]
pub fn App() -> Element {
    let data = use_signal(TableState::new);
    let language = use_signal(Language::default);
    let file_path = use_signal::<Option<PathBuf>>(|| None);
    let error_message = use_signal::<Option<String>>(|| None);
    let mut selected_row = use_signal::<Option<usize>>(|| None);
    let mut selected_column = use_signal::<Option<String>>(|| None);

    use_effect({
        let mut data = data;
        let mut file_path = file_path;
        let mut error_message = error_message;
        move || {
            if let Ok(path) = std::env::var("JSONSHEET_OPEN") {
                let path = PathBuf::from(path);
                match json_io::load_json(&path) {
                    Ok(rows) => {
                        data.with_mut(|state| {
                            state.replace_data(rows);
                        });
                        file_path.set(Some(path));
                        error_message.set(None);
                        selected_row.set(None);
                        selected_column.set(None);
                    }
                    Err(e) => {
                        error_message.set(Some(e.to_string()));
                    }
                }
            }
        }
    });

    rsx! {
        document::Style { "{STYLESHEET}" }
        div { class: "app",
            Toolbar { data, language, file_path, error_message, selected_row, selected_column }
            Table { data, language, selected_row, selected_column }
        }
    }
}
