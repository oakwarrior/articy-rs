#[allow(unreachable_code)]
use serde::de::Error as SerdeError;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Value};
use std::collections::HashMap;

use serde_enum_str::{
    Deserialize_enum_str as DeserializeString, Serialize_enum_str as SerializeString,
};
use strum_macros::IntoStaticStr;

#[derive(Debug)]
pub enum Error {
    IdNotFound,
    NoModel,
    NoMainFlow,
    NoHierarchy,

    NoCursor,
    NoDefaultPackage,
    NoOutputConnected,
    FailedToSetState,
    FailedToGetState,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct File {
    pub settings: Settings,
    pub project: Project,
    pub global_variables: Vec<GlobalVariable>,
    pub object_definitions: Vec<Object>,
    pub packages: Vec<Package>,
    pub script_methods: Vec<ScriptMethod>,
    pub hierarchy: Hierarchy,
}

impl File {
    pub fn from_buffer(bytes: &Vec<u8>) -> Self {
        serde_json::from_value(Value::Object(convert_map_to_snake_case(
            serde_json::from_slice::<Value>(bytes)
                .expect("to be able to parse articy data into serde_json Value")
                .as_object()
                .expect("the articy data to be an object at the root"),
        )))
        .expect("to parse snake cased articy data as a File")
    }

    pub fn get_default_package(&self) -> &Package {
        self.packages
            .iter()
            .find(|package| package.is_default_package)
            .expect(r#"for Articy export to have a "default" Package"#)
    }

    pub fn get_main_flow(&self) -> Option<&Hierarchy> {
        self.hierarchy.children.as_ref()?.iter().find(|item| {
            if let Type::Flow = item.kind {
                true
            } else {
                false
            }
        })
    }

    pub fn get_models_of_type(&self, kind: &str) -> Vec<&Model> {
        // FIXME: Perhaps iterate ALL of the available packages instead of assuming only one
        self.get_default_package()
            .models
            .iter()
            .filter(|model| match model {
                Model::Custom(custom_kind, _) => custom_kind == kind,
                _ => kind == Into::<&str>::into(*model),
            })
            .collect::<Vec<&Model>>()
    }

    pub fn get_models(&self) -> Vec<&Model> {
        // FIXME: Perhaps iterate ALL of the available packages instead of assuming only one
        self.get_default_package()
            .models
            .iter()
            .collect::<Vec<&Model>>()
    }

    pub fn get_dialogues_in_flow(&self, flow_id: &Id) -> Vec<&Model> {
        self.get_default_package()
            .models
            .iter()
            .filter_map(|model| {
                if let Model::Dialogue { parent, .. } = model {
                    if parent == flow_id {
                        return Some(model);
                    }
                }

                None
            })
            .collect::<Vec<&Model>>()
    }

    pub fn get_hierarchy(&self, path: Vec<Id>) -> Option<&Hierarchy> {
        let path = path.iter();
        let mut current_node = &self.hierarchy;

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
        let main_flow_id = &self.get_main_flow().ok_or(Error::NoMainFlow)?.id;
        let mut path = vec![model.id(), model.parent()];
        let mut cursor = model.parent();

        while &cursor != main_flow_id {
            let model = self
                .get_default_package()
                .models
                .iter()
                .find(|model| model.id() == cursor);
            // .ok_or(Error::NoModel)?;

            if let Some(model) = model {
                path.push(model.parent());
                cursor = model.parent()
            } else {
                break;
            }
        }

        path.reverse();

        Ok(path)
    }

    pub fn get_first_dialogue_fragment_of_dialogue(&self, model: &Model) -> Result<Id, Error> {
        let path = self.get_hierarchy_path_from_model(model)?;

        let start_dialogue_fragment_id = self
            .get_hierarchy(path)
            .ok_or(Error::NoHierarchy)?
            .children
            .as_ref()
            .ok_or(Error::NoHierarchy)?
            .iter()
            .find(|node| match node.kind {
                Type::DialogueFragment | Type::Condition | Type::Hub | Type::FlowFragment => true,
                _ => false,
            })
            .ok_or(Error::NoHierarchy)?
            .id
            .clone();

        Ok(start_dialogue_fragment_id)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Settings {
    #[serde(deserialize_with = "string_to_bool")]
    set_localization: bool,
    // set_text_formatter: String?
    #[serde(deserialize_with = "string_list_to_node_type_vector")]
    set_included_nodes: Vec<NodeType>,
    #[serde(deserialize_with = "string_to_bool")]
    set_use_script_support: bool,
    export_version: String,
}

fn string_to_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let string: String = Deserialize::deserialize(deserializer)?;
    match string.as_ref() {
        "True" | "true" => Ok(true),
        "False" | "false" => Ok(false),
        // TODO: Implement a proper Result::Err return value, instead of defaulting to false
        _ => {
            println!("Couldn't deserialize a &str into a bool, defaulting to `false`");
            Ok(false)
        }
    }
}

fn string_list_to_node_type_vector<'de, D>(deserializer: D) -> Result<Vec<NodeType>, D::Error>
where
    D: Deserializer<'de>,
{
    let string: String = Deserialize::deserialize(deserializer)?;

    Ok(string
        .split(",")
        .map(|element| {
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
                _ => NodeType::Unknown,
            }
        })
        .collect())
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Project {
    name: String,
    detail_name: String,
    guid: String, // TODO: Maybe use guid struct?
    technical_name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GlobalVariable {
    namespace: String,
    description: String,
    variables: Vec<Variable>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(try_from = "Value")]
pub struct Variable {
    name: String,
    value: VariableValue,
    description: String,
}

#[derive(Debug, Clone)]
pub enum DeserializationError {
    KeyNotFound,
    UnexpectedType,
}

impl std::fmt::Display for DeserializationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO: Find a way to write the proper error to this string
        write!(
            f,
            "DeserializationError::{}",
            match *self {
                DeserializationError::KeyNotFound => "KeyNotFound",
                DeserializationError::UnexpectedType => "UnexpectedType",
            }
        )
    }
}

impl TryFrom<Value> for Variable {
    type Error = DeserializationError;

    fn try_from(value: Value) -> Result<Variable, Self::Error> {
        let variable_value = value
            .get("value")
            .ok_or(DeserializationError::KeyNotFound)?
            .as_str()
            .ok_or(DeserializationError::UnexpectedType)?;

        Ok(Variable {
            name: value
                .get("variable")
                .ok_or(DeserializationError::KeyNotFound)?
                .as_str()
                .ok_or(DeserializationError::UnexpectedType)?
                .to_string(),

            value: match value
                .get("type")
                .ok_or(DeserializationError::KeyNotFound)?
                .as_str()
                .ok_or(DeserializationError::UnexpectedType)?
            {
                "Boolean" => match variable_value {
                    "True" | "true" => VariableValue::Boolean(true),
                    "False" | "false" => VariableValue::Boolean(false),
                    _ => panic!("Invalid value for boolean: \"{variable_value}\""),
                },
                "Integer" => match variable_value.parse::<i32>() {
                    Ok(integer) => VariableValue::Integer(integer),
                    Err(_) => panic!("Invalid value for boolean: \"{variable_value}\""),
                },
                "String" => VariableValue::String(variable_value.to_string()),
                _type => unimplemented!("Didn't implement type \"{_type}\" for VariableValue"),
            },

            description: value
                .get("description")
                .ok_or(DeserializationError::KeyNotFound)?
                .as_str()
                .ok_or(DeserializationError::UnexpectedType)?
                .to_string(),
        })
    }
}

// TODO: Perhaps combine Type + Value together?
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum VariableType {
    Boolean,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum VariableValue {
    // TODO: Remove Unknown and add deserialization error to be exhaustive
    Unknown,

    Boolean(bool),
    Integer(i32),
    String(String),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Object {
    pub class: Type,
    #[serde(rename(deserialize = "type"))]
    pub kind: Type,
    pub properties: Option<Vec<ObjectProperty>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ObjectProperty {
    property: String,
    #[serde(rename(deserialize = "type"))]
    property_type: Type,
    item_type: Option<Type>,
}

#[derive(SerializeString, DeserializeString, Debug, Clone)]
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
    #[serde(alias = "id")]
    Id,
    #[serde(alias = "float")]
    Float,
    Flow,
    Primitive,
    ArticyObject,
    Array,
    #[serde(alias = "string")]
    String,

    #[serde(other)]
    Custom(String),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Package {
    pub name: String,
    pub description: String,
    pub is_default_package: bool,
    #[serde(deserialize_with = "deserialize_model")]
    pub models: Vec<Model>,
}

#[derive(Serialize, Deserialize, Debug, Clone, IntoStaticStr)]
#[serde(
//     // // rename_all(deserialize = "PascalCase"),
    tag = "type",
    content = "properties"
)]
pub enum Model {
    DialogueFragment {
        id: Id,
        parent: Id,
        technical_name: String,

        menu_text: String,
        stage_directions: String,
        speaker: Id,
        split_height: f32,
        color: Color,
        text: String,
        external_id: Id,
        position: Point,
        size: Size,
        z_index: f32,
        short_id: ShortId,

        input_pins: Vec<Pin>,
        output_pins: Vec<Pin>,

        template: Option<HashMap<String, Value>>,
    },

    Hub {
        id: Id,
        parent: Id,
        technical_name: String,

        display_name: String,
        color: Color,
        text: String,
        external_id: Id,
        position: Point,
        z_index: f32,
        size: Size,
        short_id: ShortId,

        input_pins: Vec<Pin>,
        output_pins: Vec<Pin>,
    },

    FlowFragment {
        parent: Id,
        id: Id,
        technical_name: String,

        preview_image: PreviewImage,
        attachments: Vec<Attachment>,
        display_name: String,
        color: Color,
        text: String,
        external_id: Id,
        position: Point,
        size: Size,
        z_index: f32,
        short_id: ShortId,

        input_pins: Vec<Pin>,
        output_pins: Vec<Pin>,
    },

    Dialogue {
        id: Id,
        parent: Id,
        technical_name: String,

        preview_image: PreviewImage,
        attachments: Vec<Attachment>,
        display_name: String,
        external_id: Id,
        text: String,
        color: Color,
        position: Point,
        size: Size,
        z_index: f32,
        short_id: ShortId,
        input_pins: Vec<Pin>,
        output_pins: Vec<Pin>,
    },

    Entity {
        id: Id,
        parent: Id,
        technical_name: String,

        preview_image: PreviewImage,
        attachments: Vec<Attachment>,
        display_name: String,
        external_id: Id,
        text: String,
        color: Color,
        position: Point,
        size: Size,
        z_index: f32,
        short_id: ShortId,
    },

    Comment {
        id: Id,
        parent: Id,
        technical_name: String,

        created_by: Author,
        // FIXME: Use chrono for date format
        created_on: String,
        color: Color,
        text: String,
        external_id: Id,
        position: Point,
        z_index: f32,
        size: Size,
        short_id: ShortId,
    },

    Condition {
        id: Id,
        parent: Id,
        technical_name: String,

        display_name: String,
        external_id: Id,
        text: String,
        expression: String,
        color: Color,
        position: Point,
        size: Size,
        z_index: f32,
        short_id: ShortId,

        input_pins: Vec<Pin>,
        output_pins: Vec<Pin>,
    },

    UserFolder {
        id: Id,
        parent: Id,
        technical_name: String,
        external_id: Id,
    },

    Custom(String, Value),
}

use convert_case::{Case, Casing};

fn convert_map_to_snake_case(map: &Map<String, Value>) -> Map<String, Value> {
    let mut tmp = Vec::with_capacity(map.len());
    let mut new_map = Map::new();
    for (key, val) in map.into_iter() {
        tmp.push((key.to_case(Case::Snake), val));
    }
    for (key, val) in tmp {
        match val {
            Value::Object(object) => {
                new_map.insert(key, Value::Object(convert_map_to_snake_case(object)));
            }
            Value::Array(array) => {
                new_map.insert(
                    key,
                    Value::Array(
                        array
                            .into_iter()
                            .map(|value| match value {
                                Value::Object(object) => {
                                    Value::Object(convert_map_to_snake_case(object))
                                }
                                _ => value.clone(),
                            })
                            .collect::<Vec<Value>>(),
                    ),
                );
            }
            _ => {
                new_map.insert(key, val.clone());
            }
        }
    }

    new_map
}

fn deserialize_model<'de, D>(deserializer: D) -> Result<Vec<Model>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Value::deserialize(deserializer)?
        .as_array()
        .ok_or(DeserializationError::UnexpectedType)
        .map_err(D::Error::custom)?
        .iter()
        .map(|item| {
            // NOTE: This code makes sure that a Model can fallback to a Custom, if you notice certain models going Custom that shouldn't (e.g they're part of the Model enum list), log the `_error` and check the error message.

            let item = if let Some(template) = item.get("template") {
                let mut item = item.clone();

                item.get_mut("properties")
                    .unwrap()
                    .as_object_mut()
                    .unwrap()
                    .insert("template".to_owned(), template.clone());

                item
            } else {
                item.to_owned()
            };

            serde_json::from_value(item.clone()).unwrap_or_else(|_error| {
                // println!("ERROR: {:?} {error:#?}", item.get("type"));
                let properties = convert_map_to_snake_case(
                    item.get("properties")
                        .expect("properties to be part of a Model Value")
                        .clone()
                        .as_object()
                        .unwrap(),
                );

                let kind = item
                    .get("type")
                    .expect("Type to be part of a Model Value")
                    .as_str()
                    .expect("Type to be of type &str")
                    .to_owned();

                Model::Custom(kind, Value::Object(properties))
            })
        })
        .collect::<Vec<Model>>())
}

impl Model {
    pub fn id(&self) -> Id {
        match self {
            Model::FlowFragment { id, .. }
            | Model::DialogueFragment { id, .. }
            | Model::Hub { id, .. }
            | Model::Dialogue { id, .. }
            | Model::Comment { id, .. }
            | Model::Condition { id, .. }
            | Model::UserFolder { id, .. }
            | Model::Entity { id, .. } => id.clone(),

            Model::Custom(_, value) => match value.get("id") {
                Some(value) => match value.as_str() {
                    Some(id) => Id(id.to_owned()),
                    None => Id("Custom Model did not have Id".to_owned()),
                },
                None => Id("Custom Model did not have Id".to_owned()),
            },
        }
    }

    pub fn external_id(&self) -> Id {
        match self {
            Model::FlowFragment { external_id, .. }
            | Model::DialogueFragment { external_id, .. }
            | Model::Hub { external_id, .. }
            | Model::Dialogue { external_id, .. }
            | Model::Comment { external_id, .. }
            | Model::Condition { external_id, .. }
            | Model::UserFolder { external_id, .. }
            | Model::Entity { external_id, .. } => external_id.clone(),

            Model::Custom(_, value) => match value.get("external_id") {
                Some(value) => match value.as_str() {
                    Some(external_id) => Id(external_id.to_owned()),
                    None => Id("Custom Model did not have external_id".to_owned()),
                },
                None => Id("Custom Model did not have external_id".to_owned()),
            },
        }
    }

    pub fn parent(&self) -> Id {
        match self {
            Model::FlowFragment { parent, .. }
            | Model::DialogueFragment { parent, .. }
            | Model::Hub { parent, .. }
            | Model::Dialogue { parent, .. }
            | Model::Comment { parent, .. }
            | Model::Condition { parent, .. }
            | Model::Entity { parent, .. }
            | Model::UserFolder { parent, .. } => parent.clone(),

            Model::Custom(_, value) => match value.get("parent") {
                Some(value) => match value.as_str() {
                    Some(id) => Id(id.to_owned()),
                    None => Id("Custom Model did not have Parent Id".to_owned()),
                },
                None => Id("Custom Model did not have Parent Id".to_owned()),
            },
        }
    }

    pub fn text(&self) -> Option<String> {
        match self {
            Model::FlowFragment { text, .. }
            | Model::DialogueFragment { text, .. }
            | Model::Hub { text, .. }
            | Model::Dialogue { text, .. }
            | Model::Comment { text, .. }
            | Model::Entity { text, .. }
            | Model::Condition { text, .. } => Some(text.to_string()),
            Model::UserFolder { .. } | Model::Custom(..) => None,
        }
    }

    pub fn display_name(&self) -> Option<String> {
        match self {
            Model::FlowFragment { display_name, .. }
            | Model::Hub { display_name, .. }
            | Model::Dialogue { display_name, .. }
            | Model::Entity { display_name, .. }
            | Model::Condition { display_name, .. } => Some(display_name.to_string()),

            Model::DialogueFragment { .. }
            | Model::UserFolder { .. }
            | Model::Comment { .. }
            | Model::Custom(..) => None,
        }
    }

    pub fn input_pins(&self) -> Option<&Vec<Pin>> {
        match self {
            Model::FlowFragment { input_pins, .. }
            | Model::DialogueFragment { input_pins, .. }
            | Model::Hub { input_pins, .. }
            | Model::Dialogue { input_pins, .. }
            | Model::Condition { input_pins, .. } => Some(input_pins),

            Model::UserFolder { .. }
            | Model::Comment { .. }
            | Model::Entity { .. }
            | Model::Custom(..) => None,
        }
    }

    pub fn output_pins(&self) -> Option<&Vec<Pin>> {
        match self {
            Model::FlowFragment { output_pins, .. }
            | Model::DialogueFragment { output_pins, .. }
            | Model::Hub { output_pins, .. }
            | Model::Dialogue { output_pins, .. }
            | Model::Condition { output_pins, .. } => Some(output_pins),

            Model::UserFolder { .. }
            | Model::Entity { .. }
            | Model::Comment { .. }
            | Model::Custom(..) => None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Id(pub String);

impl Id {
    pub fn to_inner(&self) -> String {
        self.0.to_owned()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Author(pub String);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Attachment;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PreviewImage {
    view_box: Rectangle,
    mode: PreviewImageMode,
    asset: AssetId,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Rectangle {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum PreviewImageMode {
    FromAsset,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AssetId(String);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Color {
    r: f32,
    g: f32,
    b: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExternalId(String);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Point {
    x: f32,
    y: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Size {
    w: f32,
    h: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ShortId(u32);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Pin {
    pub text: String,
    pub id: Id,
    pub owner: Id,
    // NOTE: Sometimes certain pins don't have connections, default to an empty Vec<Connection> then (vec![])
    #[serde(default)]
    pub connections: Vec<Connection>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Connection {
    pub label: String,
    pub target_pin: Id,
    pub target: Id,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScriptMethod;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum NodeType {
    Unknown,

    Settings,
    Project,
    GlobalVariables,
    ObjectDefinitions,
    Packages,
    ScriptMethods,
    Hierarchy,
    Assets,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Hierarchy {
    pub id: Id,
    pub technical_name: String,
    #[serde(rename(deserialize = "type"))]
    pub kind: Type,
    pub children: Option<Vec<Hierarchy>>,
}
