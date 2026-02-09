use dioxus::prelude::*;
use std::path::PathBuf;

use crate::state::data_model::TableData;
use crate::ui::table::Table;
use crate::ui::toolbar::Toolbar;

const STYLES: Asset = asset!("/assets/styles.css");

#[component]
pub fn App() -> Element {
    let data = use_signal(TableData::new);
    let file_path = use_signal::<Option<PathBuf>>(|| None);
    let error_message = use_signal::<Option<String>>(|| None);

    rsx! {
        document::Stylesheet { href: STYLES }
        div { class: "app",
            Toolbar { data, file_path, error_message }
            Table { data: data.read().clone() }
        }
    }
}
