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
    style::{palette::tailwind::{BLUE, SLATE}, Color, Modifier, Style, Stylize},
    symbols::border,
    text::Line,
    widgets::{
        block::Title, Block, HighlightSpacing, List, ListItem, ListState, Paragraph, StatefulWidget, Widget
    },
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
            self.notes_window.get_current_notes_and_window();
            terminal.draw(|frame| self.render_frame(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn render_frame(&mut self, frame: &mut Frame) {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(frame.size());
        let notes_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Percentage(20), Constraint::Percentage(80)])
            .split(layout[1]);
        frame.render_widget(&self.last_apps_window, layout[0]);
        frame.render_widget(&mut self.notes_window, notes_layout[1]);
        match self.input_mode {
            InputMode::Normal => frame.render_widget(&self.help_window, notes_layout[0]),
            InputMode::Editing => frame.render_widget(&self.insert_note_window, notes_layout[0]),
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
            KeyCode::Esc | KeyCode::Left => self.notes_window.select_none(),
            KeyCode::Char('j') | KeyCode::Down => self.notes_window.select_next(),
            KeyCode::Char('k') | KeyCode::Up => self.notes_window.select_previous(),
            KeyCode::Char('g') | KeyCode::Home => self.notes_window.select_first(),
            KeyCode::Char('G') | KeyCode::End => self.notes_window.select_last(),
            KeyCode::Char('a') | KeyCode::Char('d') => self.notes_window.archive_selected(),
            KeyCode::Char('e') | KeyCode::Enter => self.notes_window.edit_selected(),
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

        Widget::render(list, area, buf);
    }
}

struct NotesWindow {
    state_machine_tx: Sender<StateMachine>,
    current_title: String,
    current_notes: Vec<Note>,
    selected_row: ListState,
}

impl NotesWindow {
    pub fn new(state_machine_tx: Sender<StateMachine>) -> NotesWindow {
        NotesWindow {
            state_machine_tx,
            current_title: String::new(),
            current_notes: Vec::new(),
            selected_row: ListState::default(),
        }
    }

    fn get_current_notes_and_window(&mut self) {
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
                self.current_title = title.to_owned();
                self.current_notes = app_notes.into_iter().collect();
            }
            None => {
                self.current_title = "no app currently detected".to_string();
                self.current_notes = Vec::new();
            }
        }
    }

    fn select_none(&mut self) {
        self.selected_row.select(None);
    }

    fn select_next(&mut self) {
        match self.selected_row.selected() {
            Some(row) if row < self.current_notes.len() - 1 => self.selected_row.select_next(),
            None if self.current_notes.len() > 0 => self.selected_row.select_next(),
            _ => {}
        }
    }

    fn select_previous(&mut self) {
        match self.selected_row.selected() {
            Some(row) if row > 0 => self.selected_row.select_previous(),
            None if self.current_notes.len() > 0 => self.selected_row.select_previous(),
            _ => {}
        }
    }

    fn select_first(&mut self) {
        match self.current_notes.len() {
            l if l > 0 => self.selected_row.select_first(),
            _ => {}
        }
    }

    fn select_last(&mut self) {
        match self.current_notes.len() {
            l if l > 0 => self.selected_row.select(Some(l - 1)),
            _ => {}
        }
    }

    fn archive_selected(&self) {
        match self.selected_row.selected() {
            Some(row) => {
                let note = self.current_notes.get(row).unwrap();
                self.state_machine_tx
                    .send(StateMachine::ArchiveNote(note.id))
                    .unwrap();
            }
            None => {}
        }
    }

    fn edit_selected(&self) {
        todo!()
    }

    fn alternate_colors(&self, i: usize) -> Color {
        const NORMAL_ROW_BG: Color = SLATE.c900;
        const ALT_ROW_BG_COLOR: Color = SLATE.c800;
        if i % 2 == 0 {
            NORMAL_ROW_BG
        } else {
            ALT_ROW_BG_COLOR
        }
    }
}

impl Widget for &mut NotesWindow {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = format!(" {} ", self.current_title);
        let block = Block::bordered()
            .title(Title::from(title.bold()).alignment(Alignment::Center))
            .border_set(border::THICK);
        let num = area.rows().count() - 2;
        let list_of_notes: Vec<ListItem> = self
            .current_notes
            .iter()
            .take(num)
            .enumerate()
            .map(|(i, note)| {
                let color = self.alternate_colors(i);
                ListItem::from(format!(" - {}", note.text)).bg(color)
            })
            .collect();

        const SELECTED_STYLE: Style = Style::new().bg(BLUE.c800).add_modifier(Modifier::BOLD);
        let list = List::new(list_of_notes)
            .block(block)
            .highlight_style(SELECTED_STYLE)
            .highlight_symbol(">")
            .highlight_spacing(HighlightSpacing::Always);
        StatefulWidget::render(list, area, buf, &mut self.selected_row);
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
