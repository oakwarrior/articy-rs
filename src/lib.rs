pub mod types;

use std::rc::Rc;

use types::{Error, File, Id, Model, Type};

pub use evalexpr::Value as StateValue;
use evalexpr::{
    eval_boolean_with_context, eval_with_context_mut, Context, ContextWithMutableVariables,
    HashMapContext,
};

pub struct Interpreter {
    pub file: Rc<File>,
    pub state: HashMapContext,
    pub visited: Vec<Id>,
    pub finished: Vec<Id>,
    pub cursor: Option<Id>,
}

#[derive(Debug, Clone)]
pub enum Outcome<'a> {
    Advanced(&'a Model),
    WaitingForChoice(Vec<&'a Model>),
    Stopped,
    EndOfDialogue,
}

impl Interpreter {
    pub fn new(file: Rc<File>) -> Self {
        Interpreter {
            file,
            state: HashMapContext::new(),
            cursor: None,
            visited: vec![],
            finished: vec![],
        }
    }

    pub fn set_state(&mut self, key: &str, value: StateValue) -> Result<(), Error> {
        self.state
            .set_value(key.to_owned(), value)
            .ok()
            .ok_or(Error::FailedToSetState)
    }

    pub fn get_state(&self, key: &str) -> Option<&StateValue> {
        self.state.get_value(key)
    }

    pub fn start<'a>(&mut self, id: Id) -> Result<(), Error> {
        self.cursor = Some(
            self.file
                .get_default_package()
                .models
                .iter()
                .find(|model| model.id() == id)
                .ok_or(Error::NoModel)?
                .id()
                .clone(),
        );

        match self.get_current_model() {
            Ok(Model::FlowFragment { id, .. }) => {
                let dialogue = self
                    .file
                    .get_dialogues_in_flow(&id)
                    .first()
                    .ok_or(Error::NoModel)?
                    .to_owned()
                    .clone();

                let path = self.file.get_hierarchy_path_from_model(&dialogue)?;

                // FIXME: Maybe dont assume we'll start with a piece of dialogue?
                let start_dialogue_fragment_id = self
                    .file
                    .get_hierarchy(path)
                    .ok_or(Error::NoHierarchy)?
                    .children
                    .as_ref()
                    .ok_or(Error::NoHierarchy)?
                    .iter()
                    .find(|node| match node.kind {
                        Type::DialogueFragment
                        | Type::Condition
                        | Type::Hub
                        | Type::FlowFragment => true,
                        _ => false,
                    })
                    .ok_or(Error::NoHierarchy)?
                    .id
                    .clone();

                self.cursor = Some(start_dialogue_fragment_id);
            }
            Ok(Model::Dialogue { .. }) => {
                let start_dialogue_fragment_id = self
                    .file
                    .get_first_dialogue_fragment_of_dialogue(self.get_current_model().unwrap())?;
                self.cursor = Some(start_dialogue_fragment_id);
            }
            Ok(_) => {}
            Err(error) => Err(error)?,
        }

        Ok(())
    }

    pub fn get_current_model(&self) -> Result<&Model, Error> {
        let cursor = self.cursor.as_ref().ok_or(Error::NoCursor)?;

        Ok(self
            .file
            .get_default_package()
            .models
            .iter()
            .find(|model| model.id() == *cursor)
            .ok_or(Error::NoModel)?)
    }

    pub fn get_model(&self, id: Id) -> Result<&Model, Error> {
        Ok(self
            .file
            .get_default_package()
            .models
            .iter()
            .find(|model| model.id() == id)
            .ok_or(Error::NoModel)?)
    }

    pub fn get_available_connections_at_cursor(&self) -> Result<Vec<&Model>, Error> {
        let cursor = self.cursor.as_ref().ok_or(Error::NoCursor)?;
        self.get_available_connections(cursor)
    }
    pub fn get_available_connections(&self, model_id: &Id) -> Result<Vec<&Model>, Error> {
        let model = self.get_model(model_id.clone())?;

        Ok(model
            .output_pins()
            .expect("Model to have output pins")
            .iter()
            .filter_map(|pin| {
                Some(
                    pin.connections
                        .iter()
                        .filter_map(|connection| {
                            let target_model = self
                                .file
                                .get_default_package()
                                .models
                                .iter()
                                .find(|model| model.id() == connection.target)?;

                            let target_pin = target_model
                                .input_pins()
                                .expect("Target model to have input pins")
                                .iter()
                                .find(|pin| pin.id == connection.target_pin)?;

                            match target_pin.text.as_ref() {
                                "" => Some(target_model),
                                expression => {
                                    match eval_boolean_with_context(expression, &self.state) {
                                        Ok(outcome) => match outcome {
                                            true => Some(target_model),
                                            false => None,
                                        },
                                        Err(_) => None,
                                    }
                                }
                            }
                        })
                        .collect::<Vec<&Model>>(),
                )
            })
            .flatten()
            .collect::<Vec<&Model>>())
    }

    pub fn choose(&mut self, id: Id) -> Result<Outcome, Error> {
        match self
            .get_available_connections_at_cursor()
            .ok()
            .ok_or(Error::NoOutputConnected)?
            .iter()
            .filter_map(|choice| {
                let expression = &choice.input_pins()?.first()?.text;

                match (
                    expression.is_empty(),
                    eval_boolean_with_context(expression, &self.state),
                ) {
                    (true, _) | (false, Ok(true)) => Some(choice),
                    _ => None,
                }
            })
            .find(|choice| choice.id() == id)
        {
            Some(choice) => {
                self.cursor = Some(choice.id());
                let model = self
                    .get_current_model()
                    .expect("model to be succesfully selected after choice");

                Ok(Outcome::Advanced(&model))
            }
            None => self.advance(),
        }
    }

    pub fn advance(&mut self) -> Result<Outcome, Error> {
        let cursor = self.cursor.as_ref().ok_or(Error::NoCursor)?;
        let model = self
            .file
            .get_default_package()
            .models
            .iter()
            .find(|model| model.id() == *cursor)
            .ok_or(Error::NoModel)?;

        match model {
            Model::Dialogue { .. } => Ok(Outcome::EndOfDialogue),
            Model::DialogueFragment { output_pins, .. } => {
                let connections = self
                    .get_available_connections_at_cursor()
                    .ok()
                    .ok_or(Error::NoOutputConnected)?
                    .len();

                if connections > 1 {
                    return Ok(Outcome::WaitingForChoice(
                        self.get_available_connections_at_cursor()
                            .ok()
                            .ok_or(Error::NoOutputConnected)?,
                    ));
                } else {
                    self.cursor = Some(
                        output_pins
                            .first()
                            .ok_or(Error::NoOutputConnected)?
                            .connections
                            .first()
                            .ok_or(Error::NoOutputConnected)?
                            .target
                            .clone(),
                    );
                }

                self.post_advance()
            }
            // Serves as a point for choices
            Model::Hub { .. } => {
                let choices = self
                    .get_available_connections_at_cursor()
                    .ok()
                    .ok_or(Error::NoOutputConnected)?;

                Ok(Outcome::WaitingForChoice(choices))
            }
            // TODO: Implement FlowFragment for triggering things in-game?
            Model::FlowFragment { .. } => {
                todo!("FlowFragment still needs to be implemented in articy-rs")
            }

            Model::Condition {
                expression,
                output_pins,
                ..
            } => {
                let result = match eval_boolean_with_context(&expression, &self.state) {
                    Ok(result) => result,
                    _ => false,
                };

                println!("[Condition] Input ({expression}); Outcome: {result}");

                self.cursor = Some(if result {
                    output_pins
                        .first()
                        .ok_or(Error::NoOutputConnected)?
                        .connections
                        .first()
                        .ok_or(Error::NoOutputConnected)?
                        .target
                        .clone()
                } else {
                    output_pins
                        .last()
                        .ok_or(Error::NoOutputConnected)?
                        .connections
                        .first()
                        .ok_or(Error::NoOutputConnected)?
                        .target
                        .clone()
                });

                self.post_advance()
            }

            Model::Instruction {
                expression,
                output_pins,
                ..
            } => {
                let result = eval_with_context_mut(&expression, &mut self.state);

                println!("[Instruction] Input ({expression}); Outcome: {result:#?}");

                self.cursor = Some(
                    output_pins
                        .first()
                        .ok_or(Error::NoOutputConnected)?
                        .connections
                        .first()
                        .ok_or(Error::NoOutputConnected)?
                        .target
                        .clone(),
                );

                self.post_advance()
            }

            kind => unimplemented!("Forgot to implement type {kind:?} for Interpreter::advance"),
        }
    }

    pub fn post_advance(&mut self) -> Result<Outcome, Error> {
        Ok(match self.get_current_model().ok().ok_or(Error::NoModel)? {
            Model::Dialogue { .. } => Outcome::EndOfDialogue,
            Model::Hub { .. } => {
                let choices = self
                    .get_available_connections_at_cursor()
                    .ok()
                    .ok_or(Error::NoOutputConnected)?;

                Outcome::WaitingForChoice(choices)
            }
            Model::Condition { .. } => return self.advance(),
            _ => Outcome::Advanced(self.get_current_model().ok().ok_or(Error::NoModel)?),
        })
    }

    /// Goes through all of the nodes until meeting some that force it to stop,
    /// will not tell you what outcome though since that would require looping with a &mut self 😓
    pub fn exhaust_maximally(&mut self) -> Result<(), Error> {
        loop {
            match self.advance()? {
                // TODO: If there are any state changes applied due to specific node types, be sure to apply them here as well?
                Outcome::Advanced(..) => continue,
                _ => break Ok(()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use types::*;

    #[test]
    fn parses_example_project() {
        let json =
            std::fs::read_to_string("./craftcraft.json").expect("to be able to read the file");

        let _articy_file: File =
            serde_json::from_str(&json).expect("to be able to parse articy data");
    }

    #[test]
    fn get_list_of_objects_definitions() {
        let json =
            std::fs::read_to_string("./craftcraft.json").expect("to be able to read the file");

        let file: File = serde_json::from_str(&json).expect("to be able to parse articy data");

        for object in &file.object_definitions {
            println!("{:?}", object.kind)
        }
    }

    #[test]
    fn get_list_of_models() {
        let json =
            std::fs::read_to_string("./craftcraft.json").expect("to be able to read the file");

        let file: File = serde_json::from_str(&json).expect("to be able to parse articy data");

        let models = file.get_models_of_type("Dialogue");

        println!("models: {models:#?}");
    }
}
