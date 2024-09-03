#![allow(dead_code)]
#![allow(unused_imports)]

use io::Write;
use std::io;

use articy::types::{self, File, Id, Model, Pin, Type};
use articy::{Interpreter, Outcome};

use evalexpr::{
    eval_boolean_with_context, eval_boolean_with_context_mut, eval_with_context_mut,
    ContextWithMutableVariables, EvalexprError as EvalError, HashMapContext, Value,
};

fn main() {
    let json = std::fs::read_to_string("./craftcraft.json").expect("to be able to read the file");

    let articy_file: File = serde_json::from_str(&json).expect("to be able to parse articy data");

    let start_id = Id("0x0100000100000529".into());

    let mut interpreter = Interpreter::new(articy_file.into());
    // let _ = interpreter.set_state("quality.groundskeeper_dagger", articy::StateValue::Int(2));
    let _ = interpreter.set_state(
        "item_selection.daywatch_weapon_choice",
        articy::StateValue::Boolean(false),
    );

    // println!("RESULT: {}", eval_with_context_mut(r#"game.finished = false"#, &mut interpreter.state).unwrap());

    println!("Starting with state:\n{:#?}\n---\n", interpreter.state);
    // DAY 1
    interpreter.start(start_id).unwrap();

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    'game: loop {
        let model = interpreter.get_current_model().unwrap();
        let kind = format!("{model:?}");
        let kind = kind.split('(').next().unwrap();

        println!(
            "ID: {:?}; Type: {kind:?}\n",
            interpreter.cursor.as_ref().unwrap()
        );
        println!("Text: {}", model.text().unwrap());

        // Wait for input
        write!(stdout, "\nPress any key to continue...\n").unwrap();
        stdout.flush().unwrap();

        // Read input into a buffer
        let mut buffer = String::new();
        stdin.read_line(&mut buffer).unwrap();

        let buffer = buffer.to_lowercase();
        let mut buffer = buffer.trim().split(' ');
        let command = buffer.next().unwrap();

        match command {
            "view" | "v" => {
                println!("Current node:\n{:#?}", interpreter.get_current_model())
            }
            "available" | "avail" | "a" => display_choices(&interpreter),
            "choose" | "choice" | "c" => {
                let choice = match buffer.next().unwrap_or("-1").parse::<usize>() {
                    Ok(result) => result,
                    _ => {
                        println!("invalid choice");
                        continue;
                    }
                };

                let id = match interpreter
                    .get_available_connections_at_cursor()
                    .unwrap_or_default()
                    .iter()
                    .nth(choice)
                {
                    Some(model) => model.id(),
                    None => {
                        println!("could not find id for that choice");
                        continue;
                    }
                };

                interpreter.choose(id).unwrap();
            }
            "" => match interpreter.advance().unwrap() {
                Outcome::Advanced(_) => {}
                Outcome::WaitingForChoice(_) => display_choices(&interpreter),
                Outcome::Stopped | Outcome::EndOfDialogue => break 'game,
            },
            _ => {}
        }
    }
}

fn display_choices(interpreter: &Interpreter) {
    let models = interpreter.get_available_connections_at_cursor().unwrap();

    let mut choice = 0;
    println!("\nAvailable choices:\n---");
    for model in models {
        println!(
            "({choice}): {node_name} {condition}",
            condition = match model
                .input_pins()
                .expect("Model to have input pins")
                .first() // NOTE: Assuming that the first input pin is the one we care about
                .unwrap()
                .text
                .as_str()
            {
                "" => "".to_string(),
                expression => {
                    let outcome = match eval_boolean_with_context(expression, &interpreter.state) {
                        Ok(outcome) => outcome,
                        Err(_) => false,
                    };
                    format!("({expression} ({outcome}))")
                }
            },
            node_name = match model {
                Model::DialogueFragment { text, .. } => text.to_owned(),
                _ => match model.display_name() {
                    Some(display_name) => display_name,
                    _ => "Unknown name".to_owned(),
                },
            }
        );

        choice += 1;
    }

    println!("\n");
}
