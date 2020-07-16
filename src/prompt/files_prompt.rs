use crate::color::reset_display;
use crate::git::{Git, GitStatus, GitStatusItem, GitStatusType};
use crate::Config;
use crate::TermBuffer;
use crossterm::{self as ct, style, InputEvent, KeyEvent};
use std::iter;

#[derive(Debug)]
pub struct FilesPrompt<'a> {
    config: &'a Config,
    checked: Vec<bool>,
    focused_index: u16,
    options: GitStatus,
    git: &'a Git,
}

pub enum FilesPromptResult {
    Files(Vec<String>),
    Escape,
    Terminate,
}

impl<'a> FilesPrompt<'a> {
    pub fn new(config: &'a Config, git: &'a Git, options: GitStatus) -> Self {
        FilesPrompt {
            config,
            checked: (0..options.len()).map(|_| false).collect(),
            focused_index: 0,
            options,
            git,
        }
    }

    pub fn run(mut self) -> FilesPromptResult {
        let mut buffer = TermBuffer::new();

        let input = crossterm::input();
        let mut sync_stdin = input.read_sync();

        let figlet = self
            .config
            .get_figlet()
            .expect("Ensure figlet_file points to a valid file, or remove it.");

        let mut first_iteration = true;
        loop {
            let event = if first_iteration {
                first_iteration = false;
                None
            } else {
                match sync_stdin.next() {
                    Some(e) => Some(e),
                    _ => continue,
                }
            };

            match event {
                Some(InputEvent::Keyboard(KeyEvent::Ctrl('c'))) => {
                    return FilesPromptResult::Terminate;
                }
                Some(InputEvent::Keyboard(KeyEvent::Char(' '))) => {
                    let index = self.focused_index as usize;
                    if index == 0 {
                        let set_to = !self.checked.iter().all(|&x| x);

                        for item in self.checked.iter_mut() {
                            *item = set_to;
                        }
                    } else {
                        self.checked[index - 1] = !self.checked[index - 1];
                    }
                }
                Some(InputEvent::Keyboard(KeyEvent::Char('d'))) => {
                    let index = self.focused_index as usize;
                    let files = if index == 0 {
                        vec![]
                    } else {
                        let option = self
                            .options
                            .iter()
                            .nth(index - 1)
                            .expect("diff should match a file");
                        vec![option.file_name().to_string()]
                    };

                    let _r = self.git.diff_less(files);
                }
                Some(InputEvent::Keyboard(KeyEvent::Enter)) => {
                    let selected = self
                        .options
                        .iter()
                        .enumerate()
                        .filter_map(|(i, file)| Some(file).filter(|_| self.checked[i]))
                        .map(Into::into)
                        .collect();
                    return FilesPromptResult::Files(selected);
                }

                Some(InputEvent::Keyboard(KeyEvent::Esc)) => {
                    return FilesPromptResult::Escape;
                }
                Some(InputEvent::Keyboard(KeyEvent::Up)) => {
                    self.focused_index = match self.focused_index {
                        0 => 0,
                        x => x.saturating_sub(1),
                    };
                }
                Some(InputEvent::Keyboard(KeyEvent::Down)) => {
                    let total = self.options.len() as u16 + 1;

                    self.focused_index += 1;
                    if self.focused_index >= total {
                        self.focused_index = total.saturating_sub(1);
                    }
                }
                None => {}
                _ => continue,
            };

            let mut header = figlet.create_vec();
            figlet.write_to_buf_color("<glint>", header.as_mut_slice(), |s| {
                ct::style(s).with(ct::Color::Magenta).to_string()
            });

            for line in header {
                buffer.push_line(line);
            }

            let prompt_pre = "Toggle files to commit (with <space>, or tap 'd' for diff):";
            let underscores = "-".repeat(prompt_pre.len());
            buffer.push_line("");
            buffer.push_line(prompt_pre);
            buffer.push_line(format!("{}{}", underscores, reset_display()));

            let y_offset = buffer.lines() + self.focused_index;

            let focused_color = ct::Color::Blue;
            let default_color = ct::Color::White;

            let status_untracked = style('+').with(ct::Color::Rgb { r: 96, g: 218, b: 177 });
            let status_modified = style('•').with(ct::Color::Rgb { r: 96, g: 112, b: 218 });
            let status_deleted = style('-').with(ct::Color::Rgb { r: 218, g: 96, b: 118 });
            let status_none = style(' ').with(default_color);

            // Padded limit (never overflows by 1 item)
            let total = self.options.len();
            let max = 15;
            let take = if total > max { max - 3 } else { total };

            for (i, git_status_item) in iter::once(&GitStatusItem::new("<all>".to_owned()))
                .chain(self.options.iter().map(|item| item))
                .enumerate()
                .take(take + 1)
            {
                let line_color = if i as u16 == self.focused_index { focused_color } else { default_color };

                let checked = if i == 0 {
                    self.checked.iter().all(|&x| x)
                } else {
                    self.checked[i - 1]
                };
                let prefix = style(if checked { '☑' } else { '□' }).with(line_color);

                let file_status = match *git_status_item.status() {
                    GitStatusType::Untracked => &status_untracked,
                    GitStatusType::Modified => &status_modified,
                    GitStatusType::Deleted => &status_deleted,
                    _ => &status_none,
                };

                let file_name = style(git_status_item.file_name()).with(line_color);

                let line = format!(
                    "{} {} {}{}",
                    prefix,
                    file_status,
                    file_name,
                    reset_display()
                );
                buffer.push_line(line);
            }

            if take < total {
                let diff = total - take;
                buffer.push_line(format!("and {} more", diff));
            }

            buffer.set_next_cursor((0, y_offset));
            buffer.render_frame();
            buffer.flush();
        }
    }
}
