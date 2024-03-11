use std::io;

#[derive(Debug, Eq, PartialEq)]
enum ContinueInput {
    Continue,
    Break,
}

struct Action {
    name: String,
    func: fn() -> ContinueInput,
}

impl Action {
    fn new(name: &str, func: fn() -> ContinueInput) -> Self {
        Self {
            name: String::from(name),
            func: func,
        }
    }
}

fn new_note() -> ContinueInput {
    println!("enter a new note");
    return ContinueInput::Continue;
}

fn quit() -> ContinueInput {
    println!("quitting");
    return ContinueInput::Break;
}

fn build_input_message(actions: &[Action]) -> String {
    let mut message = String::from("Pick an option:\n");
    actions.iter().enumerate().for_each(|(index, action)| {
        message += &format!("{}. {}\n", index, action.name);
    });
    message.to_string()
}

pub fn run_app() {
    let actions = vec![
        Action::new("new note", new_note as fn() -> ContinueInput),
        Action::new("quit", quit as fn() -> ContinueInput),
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
                    "please enter an integer corresponding to action: {}",
                    action_index
                ),
            }
        } else {
            println!("couldn't parse to usize, try again");
        }
    }
}
