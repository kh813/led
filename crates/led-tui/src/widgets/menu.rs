use led_core::Action;

#[derive(Debug, Clone)]
pub enum MenuItem {
    Action {
        label: String,
        action: Action,
        shortcut: Option<String>,
    },
    Toggle {
        label: String,
        action: Action,
        checked: bool,
        is_radio: bool,
    },
    Submenu {
        label: String,
        menu: Menu,
    },
    Separator,
}

#[derive(Debug, Clone)]
pub struct Menu {
    pub label: String,
    pub items: Vec<MenuItem>,
}

impl Menu {
    pub fn new(label: &str, items: Vec<MenuItem>) -> Self {
        Self {
            label: label.to_string(),
            items,
        }
    }
}
