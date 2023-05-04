#![allow(dead_code)]

use serde::{Deserialize, Deserializer};
use serde_json::Value;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct ArticyFile {
    pub settings: Settings,
    pub project: Project,
    pub global_variables: Vec<GlobalVariable>,
    pub object_definitions: Vec<Object>,
    pub packages: Vec<Package>,
    pub script_methods: Vec<ScriptMethod>,
    pub hierarchy: Hierarchy
}

impl ArticyFile {
    pub fn get_default_package(&self) -> Option<&Package> {
        self.packages
            .iter()
            .find(|package| package.is_default_package)
    }

    pub fn get_main_flow(&self) -> Option<&Hierarchy> {
        self.hierarchy
            .children
            .as_ref()?
            .iter()
            .find(
                |item| if let Type::Flow = item.kind {
                    true
                } else {
                    false
                }
            )
    }

    pub fn get_dialogues_in_flow(&self, flow_id: &Id) -> Vec<&Dialogue> {
        match self.get_default_package() {
            Some(package) => package
                .models
                .iter()
                .filter_map(
                    |model| if let Model::Dialogue(dialogue) = model {
                        if &dialogue.parent == flow_id {
                            Some(dialogue)
                        } else {
                            None
                        }
                    } else{
                        None
                    }
                )
                .collect::<Vec<&Dialogue>>(),
            None => vec![]
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Settings {
    #[serde(rename(deserialize = "set_Localization"))]
    #[serde(deserialize_with = "string_to_bool")]
    set_localization: bool,
    // set_text_formatter: String?
    #[serde(rename(deserialize = "set_IncludedNodes"))]
    #[serde(deserialize_with = "string_list_to_node_type_vector")]
    set_included_nodes: Vec<NodeType>,
    #[serde(rename(deserialize = "set_UseScriptSupport"))]
    #[serde(deserialize_with = "string_to_bool")]
    set_use_script_support: bool,
    export_version: String,
}


fn string_to_bool<'de, D>(deserializer: D) -> Result<bool, D::Error> where D: Deserializer<'de> {
    match Deserialize::deserialize(deserializer)? {
        "True" | "true" => Ok(true),
        "False" | "false" => Ok(false),
        // TODO: Implement a proper Result::Err return value, instead of defaulting to false
        _ => {
            println!("Couldn't deserialize a &str into a bool, defaulting to `false`");
            Ok(false)
        }
    }
}

fn string_list_to_node_type_vector<'de, D>(deserializer: D) -> Result<Vec<NodeType>, D::Error> where D: Deserializer<'de> {
    let string: &str = Deserialize::deserialize(deserializer)?;

    Ok(
        string.split(",")
            .map(
                |element| {
                    match element.trim() {
                        "Settings" => NodeType::Settings,
                        "Project" => NodeType::Project,
                        "GlobalVariables" => NodeType::GlobalVariables,
                        "ObjectDefinitions" => NodeType::ObjectDefinitions,
                        "Packages" => NodeType::Packages,
                        "ScriptMethods" => NodeType::ScriptMethods,
                        "Hierarchy" => NodeType::Hierarchy,
                        "Assets" => NodeType::Assets,

                        // TODO: Implement a proper Result::Err return value, instead of defaulting to Unknown
                        _ => NodeType::Unknown
                    }
                }
            )
            .collect()
      )
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Project {
    name: String,
    detail_name: String,
    guid: String, // TODO: Maybe use guid struct?
    technical_name: String
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct GlobalVariable {
    namespace: String,
    description: String,
    variables: Vec<Variable>
}

#[derive(Deserialize, Debug, Clone)]
#[serde(try_from = "Value")]
pub struct Variable {
    name: String,
    value: VariableValue,
    description: String
}

#[derive(Debug, Clone)]
pub enum DeserializationError {
    KeyNotFound,
    UnexpectedType
}

impl std::fmt::Display for DeserializationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO: Find a way to write the proper error to this string
        write!(
            f, 
            "DeserializationError::{}", 
            match *self {
                DeserializationError::KeyNotFound => "KeyNotFound",
                DeserializationError::UnexpectedType => "UnexpectedType"
            }
        )
    }
}

impl TryFrom<Value> for Variable {
    type Error = DeserializationError;

    fn try_from(value: Value) -> Result<Variable, Self::Error> {

        let variable_value = value.get("Value")
            .ok_or(DeserializationError::KeyNotFound)?
            .as_str()
            .ok_or(DeserializationError::UnexpectedType)?;

        Ok(
            Variable {
                name: value.get("Variable")
                    .ok_or(DeserializationError::KeyNotFound)?
                    .as_str()
                    .ok_or(DeserializationError::UnexpectedType)?
                    .to_string(),

                value: match value.get("Type")
                    .ok_or(DeserializationError::KeyNotFound)?
                    .as_str()
                    .ok_or(DeserializationError::UnexpectedType)? {
                        "Boolean" => match variable_value {
                            "True" | "true" => VariableValue::Boolean(true),
                            "False" | "false" => VariableValue::Boolean(false),
                            _ => panic!("Invalid value for boolean: \"{variable_value}\""),
                        },
                        "Integer" => match variable_value.parse::<i32>() {
                            Ok(integer) => VariableValue::Integer(integer),
                            Err(_) => panic!("Invalid value for boolean: \"{variable_value}\"")
                        },
                        "String" => VariableValue::String(variable_value.to_string()),
                    _type => unimplemented!("Didn't implement type \"{_type}\" for VariableValue")
                },

                description: value.get("Description")
                    .ok_or(DeserializationError::KeyNotFound)?
                    .as_str()
                    .ok_or(DeserializationError::UnexpectedType)?
                    .to_string()
            }
        )
    }
}

// TODO: Perhaps combine Type + Value together?
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub enum VariableType {
    Boolean
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub enum VariableValue {
    // TODO: Remove Unknown and add deserialization error to be exhaustive
    Unknown,

    Boolean(bool),
    Integer(i32),
    String(String)
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Object {
    pub class: Type,
    #[serde(rename(deserialize = "Type"))]
    pub kind: Type,
    pub properties: Option<Vec<ObjectProperty>>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct ObjectProperty {
    property: String,
    #[serde(rename(deserialize = "Type"))]
    property_type: Type,
    item_type: Option<Type>
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase", from = "&str")]
pub enum Type {
    Rect,
    PreviewImageViewBoxModes,
    Point,
    Color,
    InputPin,
    OutputPin,
    Size,
    PreviewImage,
    Transformation,
    OutgoingConnection,
    IncomingConnection,
    LocationAnchor,
    LocationAnchorSize,
    ShapeType,
    SelectabilityModes,
    VisibilityModes,
    OutlineStyle,
    PathCaps,
    FlowFragment,
    Dialogue,
    DialogueFragment,
    Hub,
    Spot,
    Zone,
    Comment,
    Jump,
    Entity,
    Location,
    LocationText,
    LocationImage,
    Path,
    Link,
    Asset,
    Condition,
    Instruction,
    Document,
    TextObject,
    UserFolder,
    Id,
    Float,
    Flow,

    Custom(String)
}

impl From<&str> for Type {
    fn from(from: &str) -> Type {
        match from {
            "Rect" => Type::Rect,
            "PreviewImageViewBoxModes" => Type::PreviewImageViewBoxModes,
            "Point" => Type::Point,
            "Color" => Type::Color,
            "InputPin" => Type::InputPin,
            "OutputPin" => Type::OutputPin,
            "Size" => Type::Size,
            "PreviewImage" => Type::PreviewImage,
            "Transformation" => Type::Transformation,
            "OutgoingConnection" => Type::OutgoingConnection,
            "IncomingConnection" => Type::IncomingConnection,
            "LocationAnchor" => Type::LocationAnchor,
            "LocationAnchorSize" => Type::LocationAnchorSize,
            "ShapeType" => Type::ShapeType,
            "SelectabilityModes" => Type::SelectabilityModes,
            "VisibilityModes" => Type::VisibilityModes,
            "OutlineStyle" => Type::OutlineStyle,
            "PathCaps" => Type::PathCaps,
            "FlowFragment" => Type::FlowFragment,
            "Dialogue" => Type::Dialogue,
            "DialogueFragment" => Type::DialogueFragment,
            "Hub" => Type::Hub,
            "Spot" => Type::Spot,
            "Zone" => Type::Zone,
            "Comment" => Type::Comment,
            "Jump" => Type::Jump,
            "Entity" => Type::Entity,
            "Location" => Type::Location,
            "LocationText" => Type::LocationText,
            "LocationImage" => Type::LocationImage,
            "Path" => Type::Path,
            "Link" => Type::Link,
            "Asset" => Type::Asset,
            "Condition" => Type::Condition,
            "Instruction" => Type::Instruction,
            "Document" => Type::Document,
            "TextObject" => Type::TextObject,
            "UserFolder" => Type::UserFolder,
            "Flow" => Type::Flow,
            "id" | "Id" => Type::Id,
            "float" | "Float" => Type::Float,

            other => {
                // NOTE: This line is nice for catching additional keywords
                // NOTE: Maybe implement an error when finding a term that is expressed in PascalCase
                Type::Custom(other.to_string())
            }
        }
    }
}


#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Package {
    pub name: String,
    pub description: String,
    pub is_default_package: bool,
    pub models: Vec<Model>
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase", try_from = "Value")]
pub enum Model {
    FlowFragment(FlowFragment),
    DialogueFragment(DialogueFragment),
    Hub(Hub),
    Dialogue(Dialogue),
    Comment(Comment),
    Condition(Condition),
    UserFolder(UserFolder),

    Custom(String, Value)
}

impl Model {
    pub fn id(&self) -> Id {
        match self {
            Model::FlowFragment(FlowFragment { id, .. }) |
            Model::DialogueFragment(DialogueFragment { id, .. }) |
            Model::Hub(Hub { id, .. }) |
            Model::Dialogue(Dialogue { id, .. }) |
            Model::Comment(Comment { id, .. }) |
            Model::Condition(Condition { id, .. }) |
            Model::UserFolder(UserFolder{ id, .. }) => id.clone(),
            Model::Custom(..) => unimplemented!("No Id guaranteed inside of custom model: {self:?}, implement!")
        }
    }

    pub fn parent(&self) -> Id {
        match self {
            Model::FlowFragment(FlowFragment { parent, .. }) |
            Model::DialogueFragment(DialogueFragment { parent, .. }) |
            Model::Hub(Hub { parent, .. }) |
            Model::Dialogue(Dialogue { parent, .. }) |
            Model::Comment(Comment { parent, .. }) |
            Model::Condition(Condition { parent, .. }) |
            Model::UserFolder(UserFolder{ parent, .. }) => parent.clone(),
            Model::Custom(..) => unimplemented!("No Id guaranteed inside of custom model {self:?}!")
        }
    }


    pub fn text(&self) -> String {
        match self {
            Model::FlowFragment(FlowFragment { text, .. }) |
            Model::DialogueFragment(DialogueFragment { text, .. }) |
            Model::Hub(Hub { text, .. }) |
            Model::Dialogue(Dialogue { text, .. }) |
            Model::Comment(Comment { text, .. }) |
            Model::Condition(Condition { text, .. }) => text.to_string(),
            Model::UserFolder(_) | 
            Model::Custom(..) => unimplemented!("No text guaranteed inside of {self:?}, implement!")
        }
    }

    pub fn display_name(&self) -> String {
        match self {
            Model::FlowFragment(FlowFragment { display_name, .. }) |
            Model::Hub(Hub { display_name, .. }) |
            Model::Dialogue(Dialogue { display_name, .. }) |
            Model::Condition(Condition { display_name, .. }) => display_name.to_string(),

            Model::DialogueFragment(_) |
            Model::UserFolder(_) | 
            Model::Comment(_) |
            Model::Custom(..) => unimplemented!("No display_name guaranteed inside of {self:?}")
        }
    }

    pub fn input_pins(&self) -> &Vec<Pin> {
        match self {
            Model::FlowFragment(FlowFragment { input_pins, .. }) |
            Model::DialogueFragment(DialogueFragment { input_pins, .. }) |
            Model::Hub(Hub { input_pins, .. }) |
            Model::Dialogue(Dialogue { input_pins, .. }) |
            Model::Condition(Condition { input_pins, .. }) => input_pins,

            Model::UserFolder(_) |
            Model::Comment(_) |
            Model::Custom(..) => unimplemented!("No input_pin guaranteed in {self:?}")
        }
    }

    pub fn output_pins(&self) -> &Vec<Pin> {
        match self {
            Model::FlowFragment(FlowFragment { output_pins, .. }) |
            Model::DialogueFragment(DialogueFragment { output_pins, .. }) |
            Model::Hub(Hub { output_pins, .. }) |
            Model::Dialogue(Dialogue { output_pins, .. }) |
            Model::Condition(Condition { output_pins, .. }) => output_pins,

            Model::UserFolder(_) |
            Model::Comment(_) |
            Model::Custom(..) => unimplemented!("No output_pin guaranteed in {self:?}")
        }
    }

}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct DialogueFragment {
    pub id: Id,
    pub parent: Id,
    pub technical_name: String,

    pub menu_text: String,
    pub stage_directions: String,
    pub speaker: Id,
    pub split_height: f32,
    pub color: Color,
    pub text: String,
    pub external_id: Id,
    pub position: Point,
    pub size: Size,
    pub z_index: f32,
    pub short_id: ShortId,

    pub input_pins: Vec<Pin>,
    pub output_pins: Vec<Pin>
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Hub {
    pub id: Id,
    pub parent: Id,
    pub technical_name: String,

    pub display_name: String,
    pub color: Color,
    pub text: String,
    pub external_id: Id,
    pub position: Point,
    pub z_index: f32,
    pub size: Size,
    pub short_id: ShortId,

    pub input_pins: Vec<Pin>,
    pub output_pins: Vec<Pin>
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct FlowFragment {
    pub parent: Id,
    pub id: Id,
    pub technical_name: String,

    pub preview_image: PreviewImage,
    pub attachments: Vec<Attachment>,
    pub display_name: String,
    pub color: Color,
    pub text: String,
    pub external_id: Id,
    pub position: Point,
    pub size: Size,
    pub z_index: f32,
    pub short_id: ShortId,

    pub input_pins: Vec<Pin>,
    pub output_pins: Vec<Pin>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Dialogue {
    pub id: Id,
    pub parent: Id,
    pub technical_name: String,

    pub preview_image: PreviewImage,
    pub attachments: Vec<Attachment>,
    pub display_name: String,
    pub text: String,
    pub color: Color,
    pub position: Point,
    pub size: Size,
    pub z_index: f32,
    pub short_id: ShortId,

    pub input_pins: Vec<Pin>,
    pub output_pins: Vec<Pin>
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Comment {
    pub id: Id,
    pub parent: Id,
    pub technical_name: String,

    pub created_by: Author,
    // FIXME: Use chrono for date format
    pub created_on: String,
    pub color: Color,
    pub text: String,
    pub external_id: Id,
    pub position: Point,
    pub z_index: f32,
    pub size: Size,
    pub short_id: ShortId
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Condition {
    pub id: Id,
    pub parent: Id,
    pub technical_name: String,

    pub display_name: String,
    pub text: String,
    pub expression: String,
    pub color: Color,
    pub position: Point,
    pub size: Size,
    pub z_index: f32,
    pub short_id: ShortId,

    pub input_pins: Vec<Pin>,
    pub output_pins: Vec<Pin>
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct UserFolder {
    id: Id,
    parent: Id,
    technical_name: String,
}


impl TryFrom<Value> for Model {
    type Error = DeserializationError;

    fn try_from(value: Value) -> Result<Model, Self::Error> {
        let properties = value.get("Properties")
            .ok_or(DeserializationError::KeyNotFound)?;

        match value.get("Type")
            .ok_or(DeserializationError::KeyNotFound)?
            .as_str()
            .ok_or(DeserializationError::UnexpectedType)? {

            "DialogueFragment" => Ok(Model::DialogueFragment(serde_json::from_value(properties.clone()).unwrap())),
            "Hub" => Ok(Model::Hub(serde_json::from_value(properties.clone()).unwrap())),
            "Dialogue" => Ok(Model::Dialogue(serde_json::from_value(properties.clone()).unwrap())),
            "FlowFragment" => Ok(Model::FlowFragment(serde_json::from_value(properties.clone()).unwrap())),
            "Comment" => Ok(Model::Comment(serde_json::from_value(properties.clone()).unwrap())),
            "Condition" => Ok(Model::Condition(serde_json::from_value(properties.clone()).unwrap())),
            "UserFolder" => Ok(Model::UserFolder(serde_json::from_value(properties.clone()).unwrap())),

            kind => Ok(Model::Custom(kind.to_owned(), properties.clone()))
        }
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Id(pub String);

impl Id {
    pub fn to_inner(&self) -> String {
        self.0.to_owned()
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Author(pub String);


#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Attachment;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct PreviewImage {
    view_box: Rectangle,
    mode: PreviewImageMode,
    asset: AssetId
}

#[derive(Deserialize, Debug, Clone)]
pub struct Rectangle {
    x: f32,
    y: f32,
    w: f32,
    h: f32
}

#[derive(Deserialize, Debug, Clone)]
pub enum PreviewImageMode {
    FromAsset
}

#[derive(Deserialize, Debug, Clone)]
pub struct AssetId(String);

#[derive(Deserialize, Debug, Clone)]
pub struct Color {
    r: f32, 
    g: f32,
    b: f32
}

#[derive(Deserialize, Debug, Clone)]
pub struct ExternalId(String);

#[derive(Deserialize, Debug, Clone)]
pub struct Point {
    x: f32,
    y: f32
}

#[derive(Deserialize, Debug, Clone)]
pub struct Size {
    w: f32,
    h: f32
}

#[derive(Deserialize, Debug, Clone)]
pub struct ShortId(u32);

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Pin {
   pub text: String,
   pub id: Id,
   pub owner: Id,
   pub connections: Option<Vec<Connection>>
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Connection {
    pub label: String,
    pub target_pin: Id,
    pub target: Id
}

#[derive(Deserialize, Debug, Clone)]
pub struct ScriptMethod;

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub enum NodeType {
    Unknown,

    Settings,
    Project,
    GlobalVariables,
    ObjectDefinitions,
    Packages,
    ScriptMethods,
    Hierarchy,
    Assets
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Hierarchy {
    pub id: Id,
    pub technical_name: String,
    #[serde(rename(deserialize = "Type"))]
    pub kind: Type,
    pub children: Option<Vec<Hierarchy>>
}
