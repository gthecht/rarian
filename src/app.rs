use crate::{gatherer::app_gatherer::AppGatherer, notes::NoteTaker};
use std::{fmt::Display, io};

#[derive(Debug, Eq, PartialEq)]
enum ContinueInput {
    Continue,
    Break,
}

struct Action<'a> {
    name: String,
    func: &'a mut dyn FnMut() -> ContinueInput,
}

impl<'a> Action<'a> {
    fn new(name: &str, func: &'a mut dyn FnMut() -> ContinueInput) -> Self {
        Self {
            name: String::from(name),
            func: func,
        }
    }
}

impl<'a> Display for Action<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

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
    choises.get_mut(chosen_index).expect("chosen_index is smaller then vector length")
}

fn new_note<'g>(
    app_gatherer: &'g AppGatherer,
    note_taker: &'g mut NoteTaker,
    n: Option<usize>,
) -> impl FnMut() -> ContinueInput + 'g {
    let mut num: usize = 1;
    if let Some(n) = n {
        num = n;
    }
    move || {
        println!("enter a new note");
        let note = &mut String::new();
        let stdin = io::stdin();
        stdin.read_line(note).expect("failed to read stdin");
        let mut last_processes = app_gatherer.get_last_processes(num);
        let process = choose_with_input(&mut last_processes);
        note_taker.add_note(&note.trim(), process);
        return ContinueInput::Continue;
    }
}

fn show_current<'g>(app_gatherer: &'g AppGatherer) -> impl Fn() -> ContinueInput + 'g {
    move || {
        match app_gatherer.get_current() {
            Some(current) => println!("current: {}", current.get_title()),
            None => println!("no app currently detected"),
        }
        return ContinueInput::Continue;
    }
}

fn show_last_apps<'g>(
    app_gatherer: &'g AppGatherer,
    n: Option<usize>,
) -> impl Fn() -> ContinueInput + 'g {
    let mut num: usize = 1;
    if let Some(n) = n {
        num = n;
    }
    move || {
        let last_processes = app_gatherer.get_last_processes(num);
        println!("last {} windows:", last_processes.len());
        last_processes.iter().for_each(|item| {
            println!("{}", item.get_title());
        });
        return ContinueInput::Continue;
    }
}

fn quit() -> ContinueInput {
    println!("quitting");
    return ContinueInput::Break;
}

pub fn run_app<'g>(app_gatherer: &'g AppGatherer, log_path: &str) {
    let mut note_taker = NoteTaker::new(log_path);
    let mut new_note = new_note(app_gatherer, &mut note_taker, Some(5));
    let mut show_current = show_current(app_gatherer);
    let mut show_last = show_last_apps(app_gatherer, Some(5));
    let mut quit_binding = quit;

    let mut actions = vec![
        Action::new("add a note", &mut new_note),
        Action::new("show current app", &mut show_current),
        Action::new("show last apps", &mut show_last),
        Action::new("quit rarian", &mut quit_binding),
    ];
    let mut run = ContinueInput::Continue;
    while run == ContinueInput::Continue {
        let action = choose_with_input(&mut actions);
        run = (action.func)();
    }
}
