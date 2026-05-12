pub struct Layout {
    pub width: u16,
    pub height: u16,
    pub menu_height: u16,
    pub tab_height: u16,
    pub panel_height: u16,
    pub status_height: u16,
    pub gutter_width: u16,
    pub menu_bar_items: Vec<(String, u16, u16)>, // (label, col_start, col_end)
    pub tab_rects: Vec<(usize, u16, u16)>,       // (tab_index, col_start, col_end)
    #[allow(dead_code)]
    pub tab_scroll: usize,
}

impl Layout {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            menu_height: 1,
            tab_height: 1,
            panel_height: 0,
            status_height: 1,
            gutter_width: 0,
            menu_bar_items: Vec::new(),
            tab_rects: Vec::new(),
            tab_scroll: 0,
        }
    }

    pub fn recompute(&mut self, menus: &[crate::widgets::menu::Menu], buffers: &[led_core::buffer::Editor], active_buffer_idx: usize, show_line_numbers: bool) {
        // Recompute menu items
        self.menu_bar_items.clear();
        let mut current_x = 1;
        for menu in menus {
            let start = current_x;
            let end = current_x + menu.label.chars().count() as u16 + 2;
            self.menu_bar_items.push((menu.label.clone(), start, end));
            current_x = end;
        }

        // Recompute tabs
        self.tab_rects.clear();
        let mut current_tab_x = 1;
        // We might need to handle scrolling if there are many tabs
        for (i, buffer) in buffers.iter().enumerate() {
            let name = buffer.path.as_ref()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "[No Name]".to_string());
            let modified = if buffer.is_modified() { "[+] " } else { "" };
            let ro = if buffer.read_only { "[RO] " } else { "" };
            let label = format!(" {}{}{} × ", ro, modified, name);
            let width = label.chars().count() as u16;
            
            self.tab_rects.push((i, current_tab_x, current_tab_x + width));
            current_tab_x += width + 1;
        }

        // Gutter width
        if show_line_numbers {
            if let Some(active_buffer) = buffers.get(active_buffer_idx) {
                let line_count = active_buffer.line_count();
                self.gutter_width = (line_count.to_string().len() as u16).max(2) + 2;
            } else {
                self.gutter_width = 4;
            }
        } else {
            self.gutter_width = 0;
        }
    }

    pub fn editor_bounds(&self) -> (u16, u16, u16, u16) {
        let x = self.gutter_width;
        let y = self.menu_height + self.tab_height + self.panel_height;
        let w = self.width.saturating_sub(self.gutter_width);
        let h = self.height.saturating_sub(y).saturating_sub(self.status_height);
        (x, y, w, h)
    }

    pub fn gutter_bounds(&self) -> (u16, u16, u16, u16) {
        let x = 0;
        let y = self.menu_height + self.tab_height + self.panel_height;
        let w = self.gutter_width;
        let h = self.height.saturating_sub(y).saturating_sub(self.status_height);
        (x, y, w, h)
    }

    pub fn menu_bounds(&self) -> (u16, u16, u16, u16) {
        (0, 0, self.width, self.menu_height)
    }

    pub fn tab_bounds(&self) -> (u16, u16, u16, u16) {
        (0, self.menu_height, self.width, self.tab_height)
    }

    pub fn panel_bounds(&self) -> (u16, u16, u16, u16) {
        (0, self.menu_height + self.tab_height, self.width, self.panel_height)
    }

    pub fn status_bounds(&self) -> (u16, u16, u16, u16) {
        (0, self.height.saturating_sub(self.status_height), self.width, self.status_height)
    }

    pub fn dialog_bounds(&self, dims: (u16, u16)) -> (u16, u16, u16, u16) {
        let (dw, dh) = dims;
        let x = self.width.saturating_sub(dw) / 2;
        let y = self.height.saturating_sub(dh) / 2;
        (x, y, dw, dh)
    }
}
