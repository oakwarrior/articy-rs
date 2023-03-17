#![allow(dead_code)]
#![allow(unused_imports)]

use std::io;
use io::Write;

use articy::types::{self, ArticyFile, Pin, Id, Type, Model};
use articy::{Interpreter, Outcome};

use evalexpr::{
    eval_boolean_with_context_mut, 
    eval_boolean_with_context, 
    eval_with_context_mut, 
    HashMapContext, 
    Value, 
    ContextWithMutableVariables,
    EvalexprError as EvalError
};


fn main() {
    let json = std::fs::read_to_string("./craftcraft.json")
        .expect("to be able to read the file");

    let articy_file: ArticyFile = serde_json::from_str(&json)
        .expect("to be able to parse articy data");

    // let start_id = Id("0x010000010000029F".into());
    let start_id = articy_file
        .get_default_package()
        .unwrap()
        .models
        .first()
        .unwrap()
        .id();

    let mut interpreter = Interpreter::new(articy_file);

    // println!("RESULT: {}", eval_with_context_mut(r#"game.finished = false"#, &mut interpreter.state).unwrap());

    // DAY 1
    interpreter.start(start_id).unwrap();

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    'game: loop {
        let model = interpreter.get_current_model().unwrap();
        let kind = format!("{model:?}");
        let kind = kind.split('(').next().unwrap();

        println!("\x1b[38;2;100;100;100mID: {:?}; Type: {kind}\x1b[0m", interpreter.cursor.as_ref().unwrap());
        println!("{}", model.text());

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
            },
            "available" | "avail" | "a" => {
                display_choices(&interpreter)
            },
            "choose" | "choice" | "c" => {
                let choice = match buffer.next()
                    .unwrap_or("-1")
                    .parse::<usize>() {
                    Ok(result) => result,
                    _ => {
                        println!("invalid choice");
                        continue
                    }
                };

                interpreter.choose(choice).unwrap();
            },
            "" => match interpreter.advance().unwrap() {
                Outcome::Advanced(_) => {},
                Outcome::WaitingForChoice(_choices) => {
                    display_choices(&interpreter)
                },
                Outcome::Stopped | Outcome::EndOfDialogue => break 'game
            },
            _ => {}
        }
    }
}

fn display_choices(interpreter: &Interpreter) {
    let models = interpreter.get_available_connections().unwrap();

    let mut choice = 0;
    println!("\nAvailable choices:\n---");
    for model in models {
        println!(
            "({choice}): {node_name} {condition}",
            condition = match model
                .input_pins()
                .first() // NOTE: Assuming that the first input pin is the one we care about
                .unwrap()
                .text.as_str() {
                "" => "".to_string(),
                expression => {
                    let outcome = match eval_boolean_with_context(expression, &interpreter.state) {
                        Ok(outcome) => outcome,
                        Err(_) => false
                    };
                    format!("({expression} ({outcome}))")
                }
            },
            node_name = match model {
                Model::DialogueFragment(fragment) => fragment.text.clone(),
                _ => model.display_name()
            }
        );

        choice += 1;
    }

    println!("\n");
}

