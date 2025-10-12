use std::io::Write;

use crossterm::{
    QueueableCommand as _,
    cursor::{self},
    style,
};

use crate::{
    color::{Color, blend_color},
    debug,
    editor::{Mode, Point, determine_style_for_position, render_buffer::Change},
    theme::Style,
    unicode::char_display_width,
};

use super::{Editor, render_buffer::RenderBuffer};

impl Editor {
    pub fn render(&mut self, buffer: &mut RenderBuffer) -> anyhow::Result<()> {
        self.update_gutter_width();
        let current_buffer = buffer.clone();

        let window_count = self.window_manager.windows().len();
        crate::log!("Starting render of {window_count} window(s)");
        for window_id in 0..window_count {
            self.render_window(buffer, window_id)?;
        }

        self.render_window_separators(buffer)?;

        self.render_decorations(buffer)?;

        self.update_and_render_overlays(buffer)?;

        let diff = buffer.diff(&current_buffer);
        self.render_diff(diff)?;

        Ok(())
    }

    fn render_window(&mut self, buffer: &mut RenderBuffer, window_id: usize) -> anyhow::Result<()> {
        let window_data = {
            let windows = self.window_manager.windows();
            let window_count = windows.len();

            windows
                .get(window_id)
                .map(|window| ((*window).clone(), window_count))
        };

        if let Some((window, window_count)) = window_data {
            self.render_gutter_in_window(buffer, &window)?;
            self.render_main_content_in_window(buffer, &window)?;
            if window_id < window_count - 1 {
                self.render_window_separator(buffer, &window)?;
            }
        }

        Ok(())
    }

    fn render_window_separator(
        &mut self,
        buffer: &mut RenderBuffer,
        window: &crate::window::Window,
    ) -> anyhow::Result<()> {
        let separator_style = Style {
            foreground: Some(Color::Rgb {
                r: 100,
                g: 100,
                b: 100,
            }),
            background: None,
            bold: false,
            italic: false,
        };

        let x = window.position.x + window.size.0;
        if x < self.size.0 as usize {
            for y in 0..window.size.1 {
                let terminal_y = self.window_to_terminal_y(window, y);
                buffer.set_char(x, terminal_y, '|', &separator_style, &self.theme);
            }
        }

        Ok(())
    }

    fn render_window_separators(&mut self, buffer: &mut RenderBuffer) -> anyhow::Result<()> {
        let separator_style = Style {
            foreground: Some(Color::Rgb {
                r: 100,
                g: 100,
                b: 100,
            }),
            background: None,
            bold: false,
            italic: false,
        };

        let (term_width, term_height) = (self.size.0 as usize, self.size.1 as usize);

        let windows = self.window_manager.windows();
        if windows.len() <= 1 {
            return Ok(());
        }

        let use_ascii = self.config.window_borders_ascii;

        let mut vertical_separators: Vec<(usize, usize, usize)> = Vec::new();
        let mut horizontal_separators: Vec<(usize, usize, usize)> = Vec::new();

        let mut vertical_x_positions: std::collections::HashSet<usize> =
            std::collections::HashSet::new();

        for i in 0..windows.len() {
            for j in 0..windows.len() {
                if i == j {
                    continue;
                }

                let window1 = windows[i];
                let window2 = windows[j];

                if window1.position.x + window1.size.0 + 1 == window2.position.x {
                    let x = window1.position.x + window1.size.0;
                    vertical_x_positions.insert(x);
                }
            }
        }

        // Now for each vertical separator position, find the full extent
        for x in vertical_x_positions {
            let mut min_y = term_height;
            let mut max_y = 0;

            // Find all windows that have this separator on their right edge
            for window in &windows {
                if window.position.x + window.size.0 == x {
                    min_y = min_y.min(window.position.y);
                    max_y = max_y.max(window.position.y + window.size.1);
                }
            }

            if min_y < max_y {
                vertical_separators.push((x, min_y, max_y));
            }
        }

        let mut horizontal_y_positions: std::collections::HashSet<usize> =
            std::collections::HashSet::new();

        for i in 0..windows.len() {
            for j in 0..windows.len() {
                if i == j {
                    continue;
                }
                let window1 = windows[i];
                let window2 = windows[j];

                if window1.position.y + window1.size.1 + 1 == window2.position.y {
                    let y = window1.position.y + window1.size.1;
                    horizontal_y_positions.insert(y);
                }
            }
        }

        for y in horizontal_y_positions {
            let mut min_x = term_width;
            let mut max_x = 0;

            for window in &windows {
                if window.position.y + window.size.1 == y {
                    min_x = min_x.min(window.position.x);
                    max_x = max_x.max(window.position.x + window.size.0);
                }
            }

            if min_x < max_x {
                horizontal_separators.push((y, min_x, max_x));
            }
        }

        let mut temp_grid: std::collections::HashMap<(usize, usize), char> =
            std::collections::HashMap::new();

        for (x, y_start, y_end) in &vertical_separators {
            for y in *y_start..*y_end {
                temp_grid.insert((*x, y), if use_ascii { '|' } else { '│' });
            }
        }

        for (y, x_start, x_end) in &horizontal_separators {
            for x in *x_start..*x_end {
                if let Some(existing) = temp_grid.get(&(x, *y)) {
                    if *existing == '|' || *existing == '│' {
                        temp_grid.insert((x, *y), if use_ascii { '+' } else { '┼' });
                    }
                } else {
                    temp_grid.insert((x, *y), if use_ascii { '-' } else { '─' });
                }
            }
        }

        let mut intersections = Vec::new();
        for ((x, y), ch) in &temp_grid {
            if *ch == '┼' || *ch == '+' {
                intersections.push((*x, *y, *ch));
            }
        }

        let has_vertical_component = |c: char| -> bool {
            matches!(
                c,
                '│' | '|' | '┼' | '+' | '├' | '┤' | '┬' | '┴' | '┌' | '┐' | '└' | '┘'
            )
        };

        let has_horizontal_component = |c: char| -> bool {
            matches!(
                c,
                '─' | '-' | '┼' | '+' | '┬' | '┴' | '├' | '┤' | '┌' | '┐' | '└' | '┘'
            )
        };

        let mut final_grid: std::collections::HashMap<(usize, usize), char> =
            std::collections::HashMap::new();

        for (x, y) in temp_grid.keys() {
            let connects_up = if *y > 0 {
                temp_grid
                    .get(&(*x, y.saturating_sub(1)))
                    .map(|&c| has_vertical_component(c))
                    .unwrap_or(false)
            } else {
                false
            };

            let connects_down = if *y < term_height - 1 {
                temp_grid
                    .get(&(*x, y + 1))
                    .map(|&c| has_vertical_component(c))
                    .unwrap_or(false)
            } else {
                false
            };

            let connects_left = if *x > 0 {
                temp_grid
                    .get(&(x.saturating_sub(1), *y))
                    .map(|&c| has_horizontal_component(c))
                    .unwrap_or(false)
            } else {
                false
            };

            let connects_right = if *x < term_width - 1 {
                temp_grid
                    .get(&(x + 1, *y))
                    .map(|&c| has_horizontal_component(c))
                    .unwrap_or(false)
            } else {
                false
            };

            let junction_char = if use_ascii {
                if connects_up || connects_down || connects_left || connects_right {
                    if (connects_up || connects_down) && (connects_left || connects_right) {
                        '+'
                    } else if connects_up || connects_down {
                        '|'
                    } else {
                        '-'
                    }
                } else {
                    '+'
                }
            } else {
                match (connects_up, connects_down, connects_left, connects_right) {
                    (true, true, true, true) => '┼',
                    (true, true, true, false) => '┤',
                    (true, true, false, true) => '├',
                    (true, false, true, true) => '┴',
                    (false, true, true, true) => '┬',
                    (true, false, false, true) => '└',
                    (true, false, true, false) => '┘',
                    (false, true, false, true) => '┌',
                    (false, true, true, false) => '┐',
                    (true, true, false, false) => '│',
                    (false, false, true, true) => '─',
                    (true, false, false, false) => '│',
                    (false, true, false, false) => '│',
                    (false, false, true, false) => '─',
                    (false, false, false, true) => '─',
                    (false, false, false, false) => '·',
                }
            };

            final_grid.insert((*x, *y), junction_char);
        }

        for ((x, y), char) in final_grid {
            buffer.set_char(x, y, char, &separator_style, &self.theme);
        }

        Ok(())
    }

    fn render_decorations(&mut self, buffer: &mut RenderBuffer) -> anyhow::Result<()> {
        self.draw_status_line(buffer);
        self.draw_command_line(buffer);
        Ok(())
    }

    pub fn render_diff(&mut self, change_set: Vec<Change<'_>>) -> anyhow::Result<()> {
        self.stdout.queue(cursor::Hide)?;

        let mut sorted_changes = change_set;

        sorted_changes.sort_by_key(|change| (change.y, change.x));

        let mut skip_next = false;
        for change in sorted_changes.iter() {
            if skip_next {
                skip_next = false;
                continue;
            }

            let x = change.x;
            let y = change.y;
            let cell = change.cell;

            self.stdout.queue(cursor::MoveTo(x as u16, y as u16))?;

            if let Some(background) = cell.style.background {
                let background = blend_color(
                    background,
                    self.theme
                        .style
                        .background
                        .unwrap_or(Color::Rgb { r: 0, g: 0, b: 0 }),
                );
                self.stdout
                    .queue(style::SetBackgroundColor(background.into()))?;
            } else {
                self.stdout.queue(style::SetBackgroundColor(
                    self.theme.style.background.unwrap().into(),
                ))?;
            }

            if let Some(foreground) = cell.style.foreground {
                let foreground = blend_color(
                    foreground,
                    self.theme
                        .style
                        .background
                        .unwrap_or(Color::Rgb { r: 0, g: 0, b: 0 }),
                );
                self.stdout
                    .queue(style::SetForegroundColor(foreground.into()))?;
            } else {
                self.stdout.queue(style::SetForegroundColor(
                    self.theme.style.foreground.unwrap().into(),
                ))?;
            }
            if cell.style.italic {
                self.stdout
                    .queue(style::SetAttribute(style::Attribute::Italic))?;
            } else {
                self.stdout
                    .queue(style::SetAttribute(style::Attribute::NoItalic))?;
            }
            self.stdout.queue(style::Print(cell.c))?;
        }

        self.stdout.queue(cursor::Show)?;

        self.set_cursor_style()?;
        self.draw_cursor()?;
        self.stdout.flush()?;

        Ok(())
    }

    fn render_gutter_in_window(
        &mut self,
        buffer: &mut RenderBuffer,
        window: &crate::window::Window,
    ) -> anyhow::Result<()> {
        let width = self.gutter_width();
        let gutter_style = self
            .theme
            .gutter_style
            .fallback_background(&self.theme.style);

        let window_buffer = &self.buffers[window.buffer_index];

        for y in 0..window.inner_height() {
            let line_number = y + 1 + window.vtop;
            let text = if line_number <= window_buffer.len() {
                format!("{:>width$} ", line_number)
            } else {
                " ".repeat(width + 1)
            };

            let terminal_x = window.position.x;
            let terminal_y = window.position.y + y;
            buffer.set_text(terminal_x, terminal_y, &text, &gutter_style);
        }

        Ok(())
    }

    fn update_and_render_overlays(&mut self, _buffer: &mut RenderBuffer) -> anyhow::Result<()> {
        let _cursor_position = Some(Point::new(
            self.cursor_x + self.gutter_width() + 1,
            self.cursor_y,
        ));
        Ok(())
    }

    fn render_main_content_in_window(
        &mut self,
        buffer: &mut RenderBuffer,
        window: &crate::window::Window,
    ) -> anyhow::Result<()> {
        use crate::log;
        let window_buffer = &self.buffers[window.buffer_index];
        let viewport_content = window_buffer.view(window.vtop, window.inner_height());

        if viewport_content
            .chars()
            .any(|c| c as u32 >= 0x1F300 && c as u32 <= 0x1F9FF)
        {
            for (i, c) in viewport_content.chars().enumerate().take(50) {
                if c as u32 >= 0x1F300 && c as u32 <= 0x1F9FF {
                    log!("  Char {i}: '{c}' (U+{t:04x})", t = c as u32);
                }
            }
        }

        let style_info = self.highlight(&viewport_content)?;
        let theme_style = self.theme.style.clone();

        let gutter_width = self.gutter_width();
        let mut x = gutter_width + 1;
        let mut y = 0;

        for (position, c) in viewport_content.chars().enumerate() {
            if c == '\n' {
                let terminal_x = self.window_to_terminal_x(window, x);
                let terminal_y = self.window_to_terminal_y(window, y);

                if x < window.inner_width() {
                    self.fill_line_in_window(
                        buffer,
                        terminal_x,
                        terminal_y,
                        window.inner_width() - x,
                        &theme_style,
                    );
                }

                x = gutter_width + 1;
                y += 1;
                if y >= window.inner_height() {
                    break;
                }
                continue;
            }

            let char_width = char_display_width(c);

            if x + char_width > window.inner_width() {
                continue;
            }

            let style = determine_style_for_position(&style_info, position)
                .unwrap_or(self.theme.style.clone());

            let terminal_x = self.window_to_terminal_x(window, x);
            let terminal_y = self.window_to_terminal_y(window, y);

            if char_width > 1 {
                buffer.set_char(terminal_x, terminal_y, c, &style, &self.theme);

                for i in 1..char_width {
                    if x + i < window.inner_width() {
                        buffer.set_char(terminal_x + i, terminal_y, ' ', &style, &self.theme);
                    }
                }
                x += char_width;
            } else if char_width == 0 {
                // NOOP
            } else {
                buffer.set_char(terminal_x, terminal_y, c, &style, &self.theme);
                x += 1;
            }
        }

        while y < window.inner_height() {
            let terminal_x = self.window_to_terminal_x(window, gutter_width + 1);
            let terminal_y = self.window_to_terminal_y(window, y);
            self.fill_line_in_window(
                buffer,
                terminal_x,
                terminal_y,
                window.inner_width() - gutter_width - 1,
                &theme_style,
            );
            y += 1;
        }

        Ok(())
    }

    fn fill_line_in_window(
        &mut self,
        buffer: &mut RenderBuffer,
        x: usize,
        y: usize,
        width: usize,
        style: &Style,
    ) {
        for i in 0..width {
            buffer.set_char(x + i, y, ' ', style, &self.theme);
        }
    }

    /// Draws the status line at the bottom of the editor view
    ///
    /// # Arguments
    /// - `buffer`: The buffer containing the status line
    ///
    /// # Panics
    /// This function may panic if the lengths of the mode and position strings exceed `u16::MAX`,
    /// which is currently impossible for the mode string but is not necessarily impossible for the
    /// position string
    pub fn draw_status_line(&mut self, buffer: &mut RenderBuffer) {
        let mode = format!(" {:?} ", self.mode).to_uppercase();
        debug!("Mode: {mode}");

        let active_window = self.window_manager.active_window();
        let (file, position, window_indicator) = if let Some(window) = active_window {
            let window_buffer = &self.buffers[window.buffer_index];
            let dirty = if window_buffer.is_dirty() {
                " [+] "
            } else {
                ""
            };
            let file = format!(" {}{}", window_buffer.name(), dirty);
            let position = format!(
                " {}:{} ",
                window.vtop + window.cursor_y + 1,
                window.cursor_x + 1
            );

            let window_count = self.window_manager.windows().len();
            let window_indicator = if window_count > 1 {
                format!(
                    " [{}/{}]",
                    self.window_manager.active_window_id() + 1,
                    window_count
                )
            } else {
                String::new()
            };

            (file, position, window_indicator)
        } else {
            let dirty = if self.current_buffer().is_dirty() {
                " [+] "
            } else {
                ""
            };
            let file = format!(" {}{}", self.current_buffer().name(), dirty);
            let position = format!(" {}:{} ", self.vtop + self.cursor_y + 1, self.cursor_x + 1);
            (file, position, String::new())
        };

        let file_width = self.size.0
            - mode.len() as u16
            - position.len() as u16
            - window_indicator.len() as u16
            - 2;
        let y = self.size.1 as usize - 2;

        let transition_style = Style {
            foreground: self.theme.status_line_style.outer_style.background,
            background: self.theme.status_line_style.inner_style.background,
            ..Default::default()
        };

        buffer.set_text(0, y, &mode, &self.theme.status_line_style.outer_style);

        buffer.set_text(
            mode.len(),
            y,
            &self.theme.status_line_style.outer_chars[1].to_string(),
            &transition_style,
        );

        buffer.set_text(
            mode.len() + 1,
            y,
            &format!("{:<width$}", file, width = file_width as usize),
            &self.theme.status_line_style.inner_style,
        );

        buffer.set_text(
            mode.len() + 1 + file_width as usize,
            y,
            &self.theme.status_line_style.outer_chars[2].to_string(),
            &transition_style,
        );

        buffer.set_text(
            mode.len() + 2 + file_width as usize,
            y,
            &format!("{}{}", position, window_indicator),
            &self.theme.status_line_style.outer_style,
        );
    }

    fn draw_command_line(&mut self, buffer: &mut RenderBuffer) {
        let style = &self.theme.style;
        let y = self.size.1 as usize - 1;

        if !self.is_command() {
            let wc = if let Some(ref waiting_command) = self.waiting_command {
                waiting_command.clone()
            } else {
                " ".repeat(10)
            };

            if let Some(ref last_error) = self.last_error {
                let error = format!("{:width$}", last_error, width = self.size.0 as usize);
                buffer.set_text(0, self.size.1 as usize - 1, &error, style);
            } else {
                let clear_line = " ".repeat(self.size.0 as usize - 10);
                buffer.set_text(0, y, &clear_line, style);
            }

            buffer.set_text(self.size.0 as usize - 10, y, &wc, style);

            return;
        }

        let cmd_line = format!(
            ":{:width$}",
            self.command,
            width = self.size.0 as usize - self.command.len() - 1
        );

        buffer.set_text(0, self.size.1 as usize - 1, &cmd_line, style);
    }

    /// Draws the cursor based on current style and position
    ///
    /// # Errors
    /// Can return an error if `set_cursor_style` fails or if the stdout queue fails to move the
    /// cursor
    ///
    /// # Panics
    /// This function might panic if the numeric values of the cursor's target (x, y) position
    /// exceed `u16::MAX`
    pub fn draw_cursor(&mut self) -> anyhow::Result<()> {
        self.fix_cursor_position();
        self.set_cursor_style()?;
        self.check_bounds();

        let cursor_position = if let Some(window) = self.window_manager.active_window() {
            let window_cursor_x = window.cursor_x;
            let window_cursor_y = window.cursor_y;

            let display_column = if let Some(line) = self.view_line(window.vtop + window_cursor_y) {
                let line = line.trim_end_matches('\n');
                crate::unicode::char_to_column(line, window_cursor_x)
            } else {
                window_cursor_x
            };

            let terminal_x = window.position.x + self.gutter_width() + 1 + display_column;
            let terminal_y =
                window.position.y + window_cursor_y.min(window.inner_height().saturating_sub(1));
            Some((terminal_x, terminal_y))
        } else {
            let display_column = if let Some(line) = self.view_line(self.cursor_y) {
                let line = line.trim_end_matches('\n');
                crate::unicode::char_to_column(line, self.cursor_x)
            } else {
                self.cursor_x
            };
            Some(((self.vx + display_column), self.cursor_y))
        };

        if let Some((x, y)) = cursor_position {
            self.stdout.queue(cursor::MoveTo(x as u16, y as u16))?;
        } else {
            self.stdout.queue(cursor::Hide)?;
        }

        Ok(())
    }

    fn update_gutter_width(&mut self) {
        self.vx = self.gutter_width() + 1;
    }

    fn set_cursor_style(&mut self) -> anyhow::Result<()> {
        self.stdout.queue(match self.waiting_key_action {
            Some(_) => cursor::SetCursorStyle::SteadyUnderScore,
            _ => match self.mode {
                Mode::Insert => cursor::SetCursorStyle::SteadyBar,
                _ => cursor::SetCursorStyle::DefaultUserShape,
            },
        })?;

        Ok(())
    }
}
