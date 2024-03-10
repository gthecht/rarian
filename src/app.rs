use console_utils::input::select;

enum Actions {
    NewNote,
    Quit
}

pub fn run_app() {
  let options = vec!["new note", "quit"];
  let selected_indices = select("choose action:", &options, false, false);
  match selected_indices {
    Some(indices) => println!("selected: {:?}", indices),
    None => println!("no selected indices")
  }
}