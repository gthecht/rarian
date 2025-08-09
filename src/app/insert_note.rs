use std::sync::mpsc::{channel, Sender};

use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent},
    layout::{Alignment, Rect},
    style::Stylize,
    symbols::border,
    text::{Line, Text},
    widgets::{block::Title, Block, Paragraph, Widget},
};
use ulid::Ulid;

use crate::{gatherer::app_gatherer::ActiveProcessEvent, Signals};

pub enum InputMode {
    Normal,
    Editing,
}

pub struct InsertWindow {
    state_machine_tx: Sender<Signals>,
    pub input: String,
    pub character_index: usize,
    pub editing_note: Option<Ulid>,
}

impl InsertWindow {
    pub fn new(state_machine_tx: Sender<Signals>) -> InsertWindow {
        InsertWindow {
            state_machine_tx,
            input: String::new(),
            character_index: 0,
            editing_note: None,
        }
    }

    fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.character_index.saturating_add(1);
        self.character_index = self.clamp_cursor(cursor_moved_right);
    }

    fn move_cursor_to_end(&mut self) {
        let cursor_moved_to_end = self.input.len();
        self.character_index = self.clamp_cursor(cursor_moved_to_end);
    }

    fn enter_char(&mut self, new_char: char) {
        let index = self.byte_index();
        self.input.insert(index, new_char);
        self.move_cursor_right();
    }

    /// Returns the byte index based on the character position.
    ///
    /// Since each character in a string can be contain multiple bytes, it's necessary to calculate
    /// the byte index based on the index of the character.
    fn byte_index(&self) -> usize {
        self.input
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.character_index)
            .unwrap_or(self.input.len())
    }

    fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.character_index.saturating_sub(1);
        self.character_index = self.clamp_cursor(cursor_moved_left);
    }

    fn move_cursor_home(&mut self) {
        let cursor_moved_home = 0;
        self.character_index = self.clamp_cursor(cursor_moved_home);
    }

    fn delete_char(&mut self) {
        let is_not_cursor_leftmost = self.character_index != 0;
        if is_not_cursor_leftmost {
            // Method "remove" is not used on the saved text for deleting the selected char.
            // Reason: Using remove on String works on bytes instead of the chars.
            // Using remove would require special care because of char boundaries.

            let current_index = self.character_index;
            let from_left_to_current_index = current_index - 1;

            // Getting all characters before the selected character.
            let before_char_to_delete = self.input.chars().take(from_left_to_current_index);
            // Getting all characters after selected character.
            let after_char_to_delete = self.input.chars().skip(current_index);

            // Put all characters together except the selected one.
            // By leaving the selected one out, it is forgotten and therefore deleted.
            self.input = before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left();
        }
    }

    fn delete_char_forward(&mut self) {
        let current_character_index = self.character_index;
        self.move_cursor_right();
        if self.character_index > current_character_index {
            self.delete_char();
        }
    }

    fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.input.chars().count())
    }

    fn reset_cursor(&mut self) {
        self.character_index = 0;
    }

    fn submit_message(&mut self) {
        match self.editing_note {
            Some(note_id) => self.edit_note(note_id),
            None => self.new_note(),
        }
        self.input.clear();
        self.reset_cursor();
        self.editing_note = None;
    }

    fn new_note(&self) {
        let (tx, rx) = channel::<Option<ActiveProcessEvent>>();
        self.state_machine_tx.send(Signals::CurrentApp(tx)).unwrap();
        match rx.recv().expect("main thread is alive") {
            Some(current) => {
                self.state_machine_tx
                    .send(Signals::NewNote(
                        self.input.clone().trim().to_string(),
                        vec![current.get_title().to_string()],
                    ))
                    .unwrap();
            }
            None => {
                println!("What do we do when there is no current process?");
            }
        }
    }

    fn edit_note(&self, note_id: Ulid) {
        self.state_machine_tx
            .send(Signals::EditNote(
                note_id,
                self.input.clone().trim().to_string(),
            ))
            .unwrap();
    }

    pub(crate) fn handle_key_event(&mut self, key_event: KeyEvent) -> InputMode {
        match key_event.code {
            KeyCode::Enter => {
                self.submit_message();
                InputMode::Normal
            }
            KeyCode::Char(to_insert) => {
                self.enter_char(to_insert);
                InputMode::Editing
            }
            KeyCode::Backspace => {
                self.delete_char();
                InputMode::Editing
            }
            KeyCode::Delete => {
                self.delete_char_forward();
                InputMode::Editing
            }
            KeyCode::Left => {
                self.move_cursor_left();
                InputMode::Editing
            }
            KeyCode::Right => {
                self.move_cursor_right();
                InputMode::Editing
            }
            KeyCode::End => {
                self.move_cursor_to_end();
                InputMode::Editing
            }
            KeyCode::Home => {
                self.move_cursor_home();
                InputMode::Editing
            }
            KeyCode::Esc => InputMode::Normal,
            _ => InputMode::Editing,
        }
    }
}

impl Widget for &InsertWindow {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = Title::from(" new note ".bold());
        let block = Block::bordered()
            .title(title.alignment(Alignment::Center))
            .border_set(border::THICK);
        Paragraph::new(Text::from(Line::from(self.input.as_str())))
            .block(block)
            .render(area, buf);
    }
}
