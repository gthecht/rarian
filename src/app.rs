use crate::{gatherer::app_gatherer::AppGatherer, notes};
use std::io;

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

fn build_input_message(actions: &[Action]) -> String {
    let mut message = String::from("Pick an option:\n");
    actions.iter().enumerate().for_each(|(index, action)| {
        message += &format!("{}. {}\n", index, action.name);
    });
    message.to_string()
}

fn new_note() -> ContinueInput {
    println!("enter a new note");
    let note = &mut String::new();
    let stdin = io::stdin();
    stdin.read_line(note).expect("failed to read stdin");
    notes::new_note(&note);
    return ContinueInput::Continue;
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

fn show_last<'g>(app_gatherer: &'g AppGatherer) -> impl Fn() -> ContinueInput + 'g {
    move || {
        let log = app_gatherer.get_log();
        if let Some(last) = log.last() {
            println!("previous: {}", last.get_title());
        } else {
            println!("currently there are no apps in log");
        }
        return ContinueInput::Continue;
    }
}

fn quit() -> ContinueInput {
    println!("quitting");
    return ContinueInput::Break;
}

pub fn run_app<'g>(app_gatherer: &'g AppGatherer) {
    let show_current = show_current(app_gatherer);
    let show_last = show_last(app_gatherer);
    let actions = vec![
        Action::new("new note", &new_note),
        Action::new("show current app", &show_current),
        Action::new("show last app", &show_last),
        Action::new("quit", &quit),
    ];
    let input_message = build_input_message(&actions);
    let stdin = io::stdin();
    let mut run = ContinueInput::Continue;
    while run == ContinueInput::Continue {
        let input = &mut String::new();
        println!("{}", input_message);
        stdin.read_line(input).expect("failed to read stdin");
        if let Ok(action_index) = input.trim().parse::<usize>() {
            match actions.get(action_index) {
                Some(action) => run = (action.func)(),
                None => println!(
                    "please enter an integer corresponding to action. Maximum of {}",
                    actions.len() - 1,
                ),
            }
        } else {
            println!("couldn't parse to usize, try again");
        }
    }
}
