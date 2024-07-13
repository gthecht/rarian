use crate::{gatherer::app_gatherer::ActiveProcessEvent, notes::Note, StateMachine};
use std::{
    fmt::Display,
    io,
    sync::mpsc::{channel, Sender},
};

fn build_input_message<T>(choices: &[T]) -> String
where
    T: Display,
{
    let mut message = String::from("Choose an option:\n");
    choices.iter().enumerate().for_each(|(index, choice)| {
        message += &format!("{}. {}\n", index, choice);
    });
    message.to_string()
}

fn choose_with_input<T>(choices: &mut [T]) -> &mut T
where
    T: Display,
{
    let message = build_input_message(choices);
    println!("{}", message);
    let stdin = io::stdin();
    let choices_num = choices.len();
    let chosen_index;
    loop {
        let input = &mut String::new();
        stdin.read_line(input).expect("failed to read stdin");
        if let Ok(choice_index) = input.trim().parse::<usize>() {
            if choice_index < choices_num {
                chosen_index = choice_index;
                break;
            } else {
                println!(
                    "Enter an integer corresponding to action. Maximum of {}",
                    choices_num
                );
            }
        } else {
            println!("couldn't parse to usize, try again")
        }
    }
    choices
        .get_mut(chosen_index)
        .expect("chosen_index is smaller then vector length")
}

fn new_note(state_machine_tx: Sender<StateMachine>, num: usize) {
    println!("enter a new note");
    let note = &mut String::new();
    let stdin = io::stdin();
    stdin.read_line(note).expect("failed to read stdin");
    let (tx, rx) = channel::<Vec<ActiveProcessEvent>>();
    state_machine_tx
        .send(StateMachine::RecentApps(num, tx))
        .unwrap();
    let mut last_processes = rx.recv().expect("main thread is alive");
    let process = choose_with_input(&mut last_processes);
    state_machine_tx
        .send(StateMachine::NewNote(
            note.trim().to_string(),
            process.get_title().to_string(),
        ))
        .unwrap();
}

fn show_current(state_machine_tx: Sender<StateMachine>, num: usize) {
    let (tx, rx) = channel::<Option<ActiveProcessEvent>>();
    state_machine_tx.send(StateMachine::CurrentApp(tx)).unwrap();
    match rx.recv().expect("main thread is alive") {
        Some(current) => {
            println!("current: {}", current.get_title());
            let (tx, rx) = channel::<Vec<Note>>();
            state_machine_tx
                .send(StateMachine::GetAppNotes(
                    current.get_title().to_string(),
                    tx,
                ))
                .unwrap();
            let app_notes = rx.recv().expect("main thread is alive");
            app_notes.iter().take(num).for_each(|note| {
                println!("  - {}", note.text);
            });
        }
        None => println!("no app currently detected"),
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

enum Action {
    NewNote(String),
    ShowCurrent(String),
    ShowLast(String),
    Quit(String),
}

impl Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Action::NewNote(string) => write!(f, "{}", string),
            Action::ShowCurrent(string) => write!(f, "{}", string),
            Action::ShowLast(string) => write!(f, "{}", string),
            Action::Quit(string) => write!(f, "{}", string),
        }
    }
}

pub fn run_app(state_machine_tx: Sender<StateMachine>) {
    let num: usize = 5;
    use Action::*;
    let mut actions = vec![
        NewNote("add a note".to_string()),
        ShowCurrent("show current app".to_string()),
        ShowLast("show last apps".to_string()),
        Quit("quit rarian".to_string()),
    ];
    loop {
        match choose_with_input(&mut actions) {
            NewNote(_) => new_note(state_machine_tx.clone(), num),
            ShowCurrent(_) => show_current(state_machine_tx.clone(), num),
            ShowLast(_) => show_last_apps(state_machine_tx.clone(), num),
            Quit(_) => {
                println!("quitting");
                state_machine_tx.send(StateMachine::Quit).unwrap();
                break;
            }
        }
    }
}
