use crate::{gatherer::app_gatherer::AppGatherer, notes::NoteTaker};
use std::{fmt::Display, io, path::Path};

fn build_input_message<T>(choises: &[T]) -> String
where
    T: Display,
{
    let mut message = String::from("Choose an option:\n");
    choises.iter().enumerate().for_each(|(index, choise)| {
        message += &format!("{}. {}\n", index, choise);
    });
    message.to_string()
}

fn choose_with_input<T>(choises: &mut [T]) -> &mut T
where
    T: Display,
{
    let message = build_input_message(choises);
    println!("{}", message);
    let stdin = io::stdin();
    let choices_num = choises.len();
    let chosen_index;
    loop {
        let input = &mut String::new();
        stdin.read_line(input).expect("failed to read stdin");
        if let Ok(choise_index) = input.trim().parse::<usize>() {
            if choise_index < choices_num {
                chosen_index = choise_index;
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
    choises
        .get_mut(chosen_index)
        .expect("chosen_index is smaller then vector length")
}

fn new_note(app_gatherer: &AppGatherer, note_taker: &mut NoteTaker, num: usize) {
    println!("enter a new note");
    let note = &mut String::new();
    let stdin = io::stdin();
    stdin.read_line(note).expect("failed to read stdin");
    let mut last_processes = app_gatherer.get_last_processes(num);
    let process = choose_with_input(&mut last_processes);
    note_taker.add_note(&note.trim(), process);
}

fn show_current(app_gatherer: &AppGatherer, note_taker: &NoteTaker, num: usize) {
    match app_gatherer.get_current() {
        Some(current) => {
            println!("current: {}", current.get_title());
            let app_notes = note_taker.get_app_notes(current.get_title());
            app_notes.iter().take(num).for_each(|note| {
                println!("  - {}", note.text);
            });
        }
        None => println!("no app currently detected"),
    }
}

fn show_last_apps(app_gatherer: &AppGatherer, note_taker: &NoteTaker, num: usize) {
    let last_processes = app_gatherer.get_last_processes(num);
    println!("last {} apps:", last_processes.len());
    last_processes.iter().enumerate().for_each(|(index, item)| {
        println!("{}. {}", index, item.get_title());
        let app_notes = note_taker.get_app_notes(item.get_title());
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

pub fn run_app<'g>(app_gatherer: &AppGatherer, data_path: &Path) {
    let mut note_taker = NoteTaker::new(data_path);
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
            NewNote(_) => new_note(app_gatherer, &mut note_taker, num),
            ShowCurrent(_) => show_current(app_gatherer, &note_taker, num),
            ShowLast(_) => show_last_apps(app_gatherer, &note_taker, num),
            Quit(_) => {
                println!("quitting");
                break;
            }
        }
    }
}
