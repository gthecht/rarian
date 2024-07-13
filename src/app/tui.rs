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
    layout::{Alignment, Rect},
    style::Stylize,
    symbols::border,
    text::{Line, Text},
    widgets::{
        block::{Position, Title},
        Block, Paragraph, Widget,
    },
    Frame, Terminal,
};

use crate::{gatherer::app_gatherer::ActiveProcessEvent, notes::Note, StateMachine};

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

#[derive(Debug)]
pub struct TuiApp {
    state_machine_tx: Sender<StateMachine>,
    num: usize,
    exit: bool,
}

impl TuiApp {
    pub fn new(state_machine_tx: Sender<StateMachine>, num: usize) -> TuiApp {
        TuiApp {
            state_machine_tx,
            num,
            exit: false,
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
        frame.render_widget(self, frame.size());
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

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            KeyCode::Char('Q') => self.exit(),
            _ => {}
        }
    }

    fn exit(&mut self) {
        self.exit = true;
        self.state_machine_tx.send(StateMachine::Quit).unwrap();
    }
}

impl Widget for &TuiApp {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match show_current(self.state_machine_tx.clone(), self.num) {
            Some((title, notes)) => {
                let block = Block::bordered()
                    .title(Title::from(title.bold()).alignment(Alignment::Center))
                    .border_set(border::THICK);

                let notes_text = notes
                    .iter()
                    .map(|note| Line::from(note.text.clone()))
                    .collect::<Vec<Line>>();
                Paragraph::new(notes_text)
                    .block(block)
                    .render(area, buf);
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

fn show_current(state_machine_tx: Sender<StateMachine>, num: usize) -> Option<(String, Vec<Note>)> {
    let (tx, rx) = channel::<Option<ActiveProcessEvent>>();
    state_machine_tx.send(StateMachine::CurrentApp(tx)).unwrap();
    match rx.recv().expect("main thread is alive") {
        Some(current) => {
            let title = current.get_title();
            let (tx, rx) = channel::<Vec<Note>>();
            state_machine_tx
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

fn show_last_apps(state_machine_tx: Sender<StateMachine>, num: usize) {
    let (tx, rx) = channel::<Vec<ActiveProcessEvent>>();
    state_machine_tx
        .send(StateMachine::RecentApps(num, tx))
        .unwrap();
    let last_processes = rx.recv().expect("main thread is alive");
    println!("last {} apps:", last_processes.len());
    last_processes.iter().enumerate().for_each(|(index, item)| {
        println!("{}. {}", index, item.get_title());
        let (tx, rx) = channel::<Vec<Note>>();
        state_machine_tx
            .send(StateMachine::GetAppNotes(item.get_title().to_string(), tx))
            .unwrap();
        let app_notes = rx.recv().expect("main thread is alive");
        app_notes.iter().take(num).for_each(|note| {
            println!("  - {}", note.text);
        });
    });
    println!();
}

pub fn run_app(state_machine_tx: Sender<StateMachine>) {
    let mut terminal = init().expect("crossterm init failed");
    let mut tui_app = TuiApp::new(state_machine_tx, 5);
    tui_app.run(&mut terminal).expect("app run failed");
    restore().expect("terminal restore failed");
}
