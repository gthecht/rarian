use std::{
    io::{self, stdout, Stdout},
    sync::mpsc::{channel, Sender},
};

use ratatui::{
    backend::CrosstermBackend,
    buffer::Buffer,
    crossterm::{
        event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style, Stylize},
    symbols::border,
    text::{Line, Text},
    widgets::{block::Title, Block, List, Paragraph, Widget},
    Frame, Terminal,
};

use crate::{
    app::insert_note::InsertWindow, gatherer::app_gatherer::ActiveProcessEvent, notes::Note,
    StateMachine,
};

use super::insert_note::InputMode;

pub type Tui = Terminal<CrosstermBackend<Stdout>>;

pub fn init() -> io::Result<Tui> {
    execute!(stdout(), EnterAlternateScreen)?;
    enable_raw_mode()?;
    Terminal::new(CrosstermBackend::new(stdout()))
}

pub fn restore() -> io::Result<()> {
    execute!(stdout(), LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}

pub struct TuiApp {
    state_machine_tx: Sender<StateMachine>,
    exit: bool,
    input_mode: InputMode,
    insert_note_window: InsertWindow,
    notes_window: NotesWindow,
    last_apps_window: LastAppsWindow,
    help_window: HelpWindow,
}

impl TuiApp {
    pub fn new(state_machine_tx: Sender<StateMachine>) -> TuiApp {
        TuiApp {
            state_machine_tx: state_machine_tx.clone(),
            exit: false,
            input_mode: InputMode::Normal,
            insert_note_window: InsertWindow::new(state_machine_tx.clone()),
            notes_window: NotesWindow::new(state_machine_tx.clone()),
            last_apps_window: LastAppsWindow::new(state_machine_tx.clone()),
            help_window: HelpWindow::new(),
        }
    }

    pub fn run(&mut self, terminal: &mut Tui) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.render_frame(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn render_frame(&self, frame: &mut Frame) {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(frame.size());
        let notes_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Percentage(20), Constraint::Percentage(80)])
            .split(layout[1]);
        frame.render_widget(&self.last_apps_window, layout[0]);
        frame.render_widget(&self.notes_window, notes_layout[1]);
        match self.input_mode {
            InputMode::Normal => frame.render_widget(&self.help_window, notes_layout[0]),
            _ => frame.render_widget(&self.insert_note_window, notes_layout[0]),
        }
        match self.input_mode {
            InputMode::Normal =>
                // Hide the cursor. `Frame` does this by default, so we don't need to do anything here
                {}

            InputMode::Editing => {
                // Make the cursor visible and ask ratatui to put it at the specified coordinates after
                // rendering
                #[allow(clippy::cast_possible_truncation)]
                frame.set_cursor(
                    // Draw the cursor at the current position in the input field.
                    // This position is can be controlled via the left and right arrow key
                    notes_layout[0].x + self.insert_note_window.character_index as u16 + 1,
                    // Move one line down, from the border to the input line
                    notes_layout[0].y + 1,
                );
            }
        }
    }

    fn handle_events(&mut self) -> io::Result<()> {
        if event::poll(std::time::Duration::from_millis(16))? {
            match event::read()? {
                // it's important to check that the event is a key press event as
                // crossterm also emits key release and repeat events on Windows.
                Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                    self.handle_key_event(key_event)
                }
                _ => {}
            };
        }
        Ok(())
    }

    fn handle_normal_mode_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('i') => self.input_mode = InputMode::Editing,
            KeyCode::Char('q') => self.exit(),
            KeyCode::Char('Q') => self.exit(),
            _ => {}
        }
    }

    fn handle_editing_mode_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Esc => self.input_mode = InputMode::Normal,
            _ => {
                self.input_mode = self.insert_note_window.handle_key_event(key_event);
            }
        }
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match self.input_mode {
            InputMode::Normal => self.handle_normal_mode_key_event(key_event),
            InputMode::Editing => self.handle_editing_mode_key_event(key_event),
        }
    }

    fn exit(&mut self) {
        self.exit = true;
        self.state_machine_tx.send(StateMachine::Quit).unwrap();
    }
}

struct LastAppsWindow {
    state_machine_tx: Sender<StateMachine>,
}

impl LastAppsWindow {
    pub fn new(state_machine_tx: Sender<StateMachine>) -> LastAppsWindow {
        LastAppsWindow { state_machine_tx }
    }

    fn show_last_apps(&self, num: usize) -> Vec<ActiveProcessEvent> {
        let (tx, rx) = channel::<Vec<ActiveProcessEvent>>();
        self.state_machine_tx
            .send(StateMachine::RecentApps(num, tx))
            .unwrap();
        let last_processes = rx.recv().expect("main thread is alive");
        last_processes.into_iter().take(num).collect()
    }
}

impl Widget for &LastAppsWindow {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = " latest apps ";
        let last_apps = self.show_last_apps(area.rows().count() - 2);
        let list = last_apps
            .iter()
            .map(|app_window| app_window.get_title())
            .collect::<List>()
            .block(
                Block::bordered()
                    .title(Title::from(title.bold()).alignment(Alignment::Center))
                    .border_set(border::THICK),
            )
            .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
            .highlight_symbol(">>")
            .repeat_highlight_symbol(true);

        list.render(area, buf);
    }
}

struct NotesWindow {
    state_machine_tx: Sender<StateMachine>,
}

impl NotesWindow {
    pub fn new(state_machine_tx: Sender<StateMachine>) -> NotesWindow {
        NotesWindow { state_machine_tx }
    }

    fn show_current(&self, num: usize) -> Option<(String, Vec<Note>)> {
        let (tx, rx) = channel::<Option<ActiveProcessEvent>>();
        self.state_machine_tx
            .send(StateMachine::CurrentApp(tx))
            .unwrap();
        match rx.recv().expect("main thread is alive") {
            Some(current) => {
                let title = current.get_title();
                let (tx, rx) = channel::<Vec<Note>>();
                self.state_machine_tx
                    .send(StateMachine::GetAppNotes(
                        current.get_title().to_string(),
                        tx,
                    ))
                    .unwrap();
                let app_notes = rx.recv().expect("main thread is alive");
                Some((title.to_owned(), app_notes.into_iter().take(num).collect()))
            }
            None => None,
        }
    }
}

impl Widget for &NotesWindow {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match self.show_current(area.rows().count() - 2) {
            Some((title, notes)) => {
                let title = format!(" {} ", title);
                let block = Block::bordered()
                    .title(Title::from(title.bold()).alignment(Alignment::Center))
                    .border_set(border::THICK);

                let notes_text = notes
                    .iter()
                    .map(|note| Line::from(format!(" - {}", note.text.clone())))
                    .collect::<Vec<Line>>();
                Paragraph::new(notes_text).block(block).render(area, buf);
            }
            None => {
                let title = Title::from(" no app currently detected ".bold());
                let block = Block::bordered()
                    .title(title.alignment(Alignment::Center))
                    .border_set(border::THICK);
                Paragraph::new(Text::default())
                    .centered()
                    .block(block)
                    .render(area, buf);
            }
        }
    }
}

struct HelpWindow {}

impl HelpWindow {
    pub fn new() -> HelpWindow {
        HelpWindow {}
    }
}

impl Widget for &HelpWindow {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let help_message = Line::from(
            "i = enter insert mode; q = quit; ESC = escape insert mode; ENTER = add new note",
        );
        let title = Title::from(" help window ".bold());
        let block = Block::bordered()
            .title(title.alignment(Alignment::Center))
            .border_set(border::THICK);
        Paragraph::new(help_message).block(block).render(area, buf);
    }
}

pub fn run_app(state_machine_tx: Sender<StateMachine>) {
    let mut terminal = init().expect("crossterm init failed");
    let mut tui_app = TuiApp::new(state_machine_tx);
    tui_app.run(&mut terminal).expect("app run failed");
    restore().expect("terminal restore failed");
}
