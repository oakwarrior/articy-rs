#![allow(dead_code)]
#![allow(unused_imports)]
pub mod types;

use types::{ArticyFile, Pin, ModelType, Id, Model, Connection};

use evalexpr::{
    eval_boolean_with_context_mut, 
    eval_boolean_with_context, 
    eval_with_context_mut, 
    eval_with_context, 
    HashMapContext, 
    Value, 
    ContextWithMutableVariables,
    EvalexprError as EvalError
};

pub struct Interpreter {
    pub file: ArticyFile,
    pub state: HashMapContext,
    pub cursor: Option<Id>
}

#[derive(Debug)]
pub enum InterpreterError {
    IdNotFound,
    NoModelFound,
    NoExpressionFound,

    NoCursor,
    NoPackages,
    NoOutputConnected,
    NoOutputPins,
}

#[derive(Debug)]
pub enum Outcome<'a> {
    Advanced,
    WaitingForChoice(Vec<&'a Model>),
    Stopped,
    EndOfDialogue
}


impl Interpreter {
    pub fn new(file: ArticyFile) -> Self {
        Interpreter {
            file,
            // TODO: Write custom state implementation
            state: HashMapContext::new(),
            cursor: None
        }
    }

    pub fn start<'a>(&mut self, id: Id) -> Result<(), InterpreterError> {
        // FIXME: Do not assume package to navigate in
        self.cursor = if let Some(package) = self.file.packages.first() {
            if let Some(model) = package.models.iter().find(|model| model.properties.id == id) {
                Some(model.properties.id.clone())
            } else {
                None
            }
        } else {
            None
        };

        Ok(())
    }

    pub fn get_current_model(&self) -> Result<&Model, InterpreterError> {
        let cursor = self.cursor.as_ref().ok_or(InterpreterError::NoCursor)?;

        Ok(
            self
                .file
                .packages
                .first()
                .unwrap()
                .models
                .iter()
                .find(|model| model.properties.id == *cursor)
                .ok_or(InterpreterError::NoModelFound)?
        )
    }

    pub fn get_available_connections(&self) -> Result<Vec<&Model>, InterpreterError> {
        Ok(
            self.get_current_model()?
                .properties
                .output_pins
                .as_ref()
                .ok_or(InterpreterError::NoOutputPins)?
                .iter()
                .filter_map(
                    |pin| {
                        Some(
                            pin.connections
                                .as_ref()
                                .unwrap()
                                .iter()
                                .filter_map(
                                    |connection| {
                                        let target_model = self.file.packages.first().unwrap()
                                            .models
                                            .iter()
                                            .find(|model| model.properties.id == connection.target)
                                            .unwrap();

                                        let target_pin = target_model
                                            .properties
                                            .input_pins
                                            .as_ref()
                                            .unwrap()
                                            .iter()
                                            .find(|pin| pin.id == connection.target_pin)
                                            .unwrap();
                                        
                                        match target_pin.text.as_ref() {
                                            "" => Some(target_model),
                                            expression => match eval_boolean_with_context(expression, &self.state) {
                                                Ok(outcome) => match outcome {
                                                    true => Some(target_model),
                                                    false => None
                                                },
                                                Err(_) => None
                                            }
                                        }
                                    }
                                )
                                .collect::<Vec<&Model>>()
                        )
                    }
                )
                .flatten()
                .collect::<Vec<&Model>>()
        )
    }

    pub fn choose(&mut self, index: usize) -> Result<Outcome, InterpreterError> {
        match self.get_available_connections()
            .ok()
            .ok_or(InterpreterError::NoOutputConnected)?
            .iter()
            .nth(index) {
            Some(choice) => {
                self.cursor = Some(choice.properties.id.clone());

                Ok(Outcome::Advanced)
            },
            None => self.advance()
        }
    }

    pub fn advance(&mut self) -> Result<Outcome, InterpreterError> {
        // FIXME: Do not assume package to navigate in
        let cursor = self.cursor.as_ref().ok_or(InterpreterError::NoCursor)?;
        let model = self.file.packages
            .first()
            .ok_or(InterpreterError::NoPackages)?
            .models
            .iter()
            .find(|model| model.properties.id == *cursor)
            .ok_or(InterpreterError::NoModelFound)?;

        match &model.model_type {
            ModelType::Dialogue => {
                Ok(Outcome::EndOfDialogue)
            },
            ModelType::DialogueFragment => {
                let connections = model
                    .properties
                    .output_pins
                    .as_ref()
                    .ok_or(InterpreterError::NoOutputConnected)?
                    .first()
                    .ok_or(InterpreterError::NoOutputConnected)?
                    .connections
                    .as_ref()
                    .ok_or(InterpreterError::NoOutputConnected)?;

                self.cursor = Some(
                    connections
                        .first()
                        .ok_or(InterpreterError::NoOutputConnected)?
                        .target
                        .clone()
                );

                // NOTE: Find a way to generalize this?
                Ok(
                    match self.get_current_model().ok().ok_or(InterpreterError::NoModelFound)?.model_type {
                        ModelType::Dialogue => Outcome::EndOfDialogue,
                        ModelType::Hub => {
                            let choices = self.get_available_connections().ok()
                                .ok_or(InterpreterError::NoOutputConnected)?;

                            Outcome::WaitingForChoice(choices)
                        },
                        ModelType::Condition => return self.advance(),
                        _ => Outcome::Advanced
                    }
                )
            },
            // Serves as a point for choices
            ModelType::Hub => {
                let choices = self.get_available_connections().ok()
                    .ok_or(InterpreterError::NoOutputConnected)?;

                Ok(Outcome::WaitingForChoice(choices))
            },
            ModelType::Condition => {
                let expression = model.properties.expression.as_ref()
                    .ok_or(InterpreterError::NoExpressionFound)?;

                let result = match eval_boolean_with_context(&expression, &self.state) {
                    Ok(result) => result,
                    _ => false
                };

                let connections = model
                    .properties
                    .output_pins
                    .as_ref()
                    .ok_or(InterpreterError::NoOutputConnected)?
                    .first()
                    .ok_or(InterpreterError::NoOutputConnected)?
                    .connections
                    .as_ref()
                    .ok_or(InterpreterError::NoOutputConnected)?;

                println!("[Condition] Input ({expression}); Outcome: {result}\n");
                
                self.cursor = Some(
                    if result {
                        connections
                            .first()
                            .ok_or(InterpreterError::NoOutputConnected)?
                            .target
                            .clone()
                    } else {
                        connections
                            .last()
                            .ok_or(InterpreterError::NoOutputConnected)?
                            .target
                            .clone()
                    }
                );

                Ok(
                    match self.get_current_model().ok().ok_or(InterpreterError::NoModelFound)?.model_type {
                        ModelType::Dialogue => Outcome::EndOfDialogue,
                        ModelType::Hub => {
                            let choices = self.get_available_connections().ok()
                                .ok_or(InterpreterError::NoOutputConnected)?;

                            Outcome::WaitingForChoice(choices)
                        },
                        ModelType::Condition => return self.advance(),
                        _ => Outcome::Advanced
                    }
                )
            },
            _type => unimplemented!("Forgot to implement type {_type:?} for Interpreter::advance")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use types::*;
    use std::io;
    use std::io::prelude::*;

    #[test]
    fn run_basic_intepreter() {
    }
}
