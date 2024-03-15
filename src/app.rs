use crate::{gatherer::app_gatherer::AppGatherer, notes};
use std::{fmt::Display, io};

#[derive(Debug, Eq, PartialEq)]
enum ContinueInput {
    Continue,
    Break,
}

struct Action<'a> {
    name: String,
    func: &'a dyn Fn() -> ContinueInput,
}

impl<'a> Action<'a> {
    fn new(name: &str, func: &'a dyn Fn() -> ContinueInput) -> Self {
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

fn choose_with_input<T>(choises: &[T]) -> &T
where
    T: Display,
{
    let message = build_input_message(choises);
    println!("{}", message);
    let input = &mut String::new();
    let stdin = io::stdin();
    loop {
        stdin.read_line(input).expect("failed to read stdin");
        if let Ok(choise_index) = input.trim().parse::<usize>() {
            match choises.get(choise_index) {
                Some(choise) => return choise,
                None => println!(
                    "Enter an integer corresponding to action. Maximum of {}",
                    choises.len() - 1,
                ),
            }
        } else {
            println!("couldn't parse to usize, try again")
        }
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

fn show_last<'g>(
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

fn new_note<'g>(
    app_gatherer: &'g AppGatherer,
    n: Option<usize>,
) -> impl Fn() -> ContinueInput + 'g {
    let mut num: usize = 1;
    if let Some(n) = n {
        num = n;
    }
    move || {
        println!("enter a new note");
        let note = &mut String::new();
        let stdin = io::stdin();
        stdin.read_line(note).expect("failed to read stdin");
        let last_processes = app_gatherer.get_last_processes(num);
        let process = choose_with_input(&last_processes);
        notes::new_note(&note, process);
        return ContinueInput::Continue;
    }
}

fn quit() -> ContinueInput {
    println!("quitting");
    return ContinueInput::Break;
}

pub fn run_app<'g>(app_gatherer: &'g AppGatherer) {
    let show_current = show_current(app_gatherer);
    let show_last = show_last(app_gatherer, Some(5));
    let new_note = new_note(app_gatherer, Some(5));
    let actions = vec![
        Action::new("new note", &new_note),
        Action::new("show current app", &show_current),
        Action::new("show last app", &show_last),
        Action::new("quit", &quit),
    ];
    let mut run = ContinueInput::Continue;
    while run == ContinueInput::Continue {
        let action = choose_with_input(&actions);
        run = (action.func)();
    }
}
