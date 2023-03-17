#![allow(dead_code)]
#![allow(unused_imports)]
pub mod types;

use types::{ArticyFile, Pin, Type, Id, Model, Connection, Hierarchy};

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
    pub visited: Vec<Id>,
    pub finished: Vec<Id>,
    pub cursor: Option<Id>
}

#[derive(Debug)]
pub enum Error {
    IdNotFound,
    NoModel,
    NoMainFlow,
    NoHierarchy,

    NoCursor,
    NoDefaultPackage,
    NoOutputConnected
}

#[derive(Debug)]
pub enum Outcome<'a> {
    Advanced(&'a Model),
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
            cursor: None,
            visited: vec![],
            finished: vec![]
        }
    }

    pub fn start<'a>(&mut self, id: Id) -> Result<(), Error> {
        self.cursor = Some(
            self.file
                .get_default_package()
                .ok_or(Error::NoDefaultPackage)?
                .models
                .iter()
                .find(|model| model.id() == id)
                .ok_or(Error::NoModel)?
                .id()
                .clone()
        );

        match self.get_current_model() {
            Ok(Model::FlowFragment(flow_fragment)) => {
                let dialogue = self.file.get_dialogues_in_flow(&flow_fragment.id)
                    .iter()
                    .nth(2)
                    .ok_or(Error::NoModel)?
                    .to_owned()
                    .clone();

                // FIXME: Maybe don't rely on clone?
                let path = self.get_hierarchy_path_from_model(&Model::Dialogue(dialogue.clone()))?;

                // FIXME: Maybe dont assume we'll start with a piece of dialogue?
                let start_dialogue_fragment_id = self.get_hierarchy(path)
                    .ok_or(Error::NoHierarchy)?
                    .children
                    .as_ref()
                    .ok_or(Error::NoHierarchy)?
                    .iter()
                    .find(
                        |node| if let Type::DialogueFragment = node.kind {
                            true
                        } else {
                            false
                        }
                    )
                    .ok_or(Error::NoHierarchy)?
                    .id
                    .clone();

                self.cursor = Some(start_dialogue_fragment_id);
            },
            Ok(Model::Dialogue(_dialogue)) => {
                let path = self.get_hierarchy_path_from_model(self.get_current_model()?)?;
                let start_dialogue_fragment_id = self.get_hierarchy(path)
                    .ok_or(Error::NoHierarchy)?
                    .children
                    .as_ref()
                    .ok_or(Error::NoHierarchy)?
                    .iter()
                    .find(
                        |node| if let Type::DialogueFragment = node.kind {
                            true
                        } else {
                            false
                        }
                    )
                    .ok_or(Error::NoHierarchy)?
                    .id
                    .clone();

                self.cursor = Some(start_dialogue_fragment_id);
            },
            Ok(_) => {}
            Err(error) => Err(error)?
        }

        Ok(())
    }

    pub fn get_hierarchy(&self, path: Vec<Id>) -> Option<&Hierarchy> {
        let path = path.iter();
        let mut current_node = &self.file.hierarchy;

        for id in path {
            current_node = current_node
                .children
                .as_ref()?
                .iter()
                .find(|node| &node.id == id)?;
        }

        Some(current_node)
    }

    pub fn get_hierarchy_path_from_model(&self, model: &Model) -> Result<Vec<Id>, Error> {
        let main_flow_id = &self.file.get_main_flow().unwrap().id;
        let mut path = vec![model.id(), model.parent()];
        let mut cursor = model.parent();

        while &cursor != main_flow_id {
            let model = self.file
                .get_default_package()
                .ok_or(Error::NoDefaultPackage)?
                .models
                .iter()
                .find(|model| model.id() == cursor)
                .ok_or(Error::NoModel)?;


            path.push(model.parent());
            cursor = model.parent()
        }

        path.reverse();

        Ok(path)
    }


    pub fn get_current_model(&self) -> Result<&Model, Error> {
        let cursor = self.cursor.as_ref().ok_or(Error::NoCursor)?;

        Ok(
            self
                .file
                .packages
                .first()
                .unwrap()
                .models
                .iter()
                .find(|model| model.id() == *cursor)
                .ok_or(Error::NoModel)?
        )
    }

    pub fn get_available_connections(&self) -> Result<Vec<&Model>, Error> {
        Ok(
            self.get_current_model()?
                .output_pins()
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
                                        let target_model = self.file.get_default_package().unwrap()
                                            .models
                                            .iter()
                                            .find(|model| model.id() == connection.target)
                                            .unwrap();

                                        let target_pin = target_model
                                            .input_pins()
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

    // TODO: Choose using Id's
    pub fn choose(&mut self, index: usize) -> Result<Outcome, Error> {
        match self.get_available_connections()
            .ok()
            .ok_or(Error::NoOutputConnected)?
            .iter()
            .nth(index) {
            Some(choice) => {
                self.cursor = Some(choice.id());
                let model = self.get_current_model().unwrap();

                Ok(Outcome::Advanced(&model))
            },
            None => self.advance()
        }
    }

    pub fn post_advance(&mut self) -> Result<Outcome, Error> {
        Ok(
            match self.get_current_model().ok().ok_or(Error::NoModel)? {
                Model::Dialogue(_) => Outcome::EndOfDialogue,
                Model::Hub(_) => {
                    let choices = self.get_available_connections().ok()
                        .ok_or(Error::NoOutputConnected)?;

                    Outcome::WaitingForChoice(choices)
                },
                Model::Condition(_) => return self.advance(),
                _ => Outcome::Advanced(&self.get_current_model().unwrap())
            }
        )
    }

    pub fn advance(&mut self) -> Result<Outcome, Error> {
        let cursor = self.cursor.as_ref().ok_or(Error::NoCursor)?;
        let model = self.file
            .get_default_package()
            .ok_or(Error::NoDefaultPackage)?
            .models
            .iter()
            .find(|model| model.id() == *cursor)
            .ok_or(Error::NoModel)?;

        match model {
            Model::Dialogue(_) => {
                Ok(Outcome::EndOfDialogue)
            },
            Model::DialogueFragment(fragment) => {
                self.cursor = Some(
                    fragment 
                        .output_pins
                        .first()
                        .ok_or(Error::NoOutputConnected)?
                        .connections
                        .as_ref()
                        .ok_or(Error::NoOutputConnected)?
                        .first()
                        .ok_or(Error::NoOutputConnected)?
                        .target
                        .clone()
                );

                self.post_advance()
            },
            // Serves as a point for choices
            Model::Hub(_) => {
                let choices = self.get_available_connections().ok()
                    .ok_or(Error::NoOutputConnected)?;

                Ok(Outcome::WaitingForChoice(choices))
            },
            // TODO: Implement FlowFragment for triggering things in-game?
            Model::FlowFragment(_flow_fragment) => todo!(),

            Model::Condition(condition) => {
                let result = match eval_boolean_with_context(&condition.expression, &self.state) {
                    Ok(result) => result,
                    _ => false
                };

                let connections = condition
                    .output_pins
                    .first()
                    .ok_or(Error::NoOutputConnected)?
                    .connections
                    .as_ref()
                    .ok_or(Error::NoOutputConnected)?;

                println!("[Condition] Input ({expression}); Outcome: {result}\n", expression = condition.expression);

                self.cursor = Some(
                    if result {
                        connections
                            .first()
                            .ok_or(Error::NoOutputConnected)?
                            .target
                            .clone()
                    } else {
                        connections
                            .last()
                            .ok_or(Error::NoOutputConnected)?
                            .target
                            .clone()
                    }
                );

                self.post_advance()
            },
            kind => unimplemented!("Forgot to implement type {kind:?} for Interpreter::advance")
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
    fn parses_example_project() {
        let json = std::fs::read_to_string("./craftcraft.json")
            .expect("to be able to read the file");

        let _articy_file: ArticyFile = serde_json::from_str(&json)
            .expect("to be able to parse articy data");
    }

    #[test]
    fn get_list_of_objects_definitions() {
        let json = std::fs::read_to_string("./craftcraft.json")
            .expect("to be able to read the file");

        let file: ArticyFile = serde_json::from_str(&json)
            .expect("to be able to parse articy data");


        for object in &file.object_definitions {
            println!("{:?}", object.kind)
        }

        println!("Custom:");
    }

    #[test]
    fn get_list_of_models() {
        let json = std::fs::read_to_string("./craftcraft.json")
            .expect("to be able to read the file");

        let file: ArticyFile = serde_json::from_str(&json)
            .expect("to be able to parse articy data");

        for model in &file.get_default_package().unwrap().models {
            match model {
                Model::Hub(model) => {
                    println!("{:#?}", model);
                    break
                },
                _ => {}
            }
        }

    }
}
