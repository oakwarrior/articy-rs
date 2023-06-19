#![allow(dead_code)]

use serde::de::Error as SerdeError;
use serde::{Deserialize, Deserializer};
use serde_json::Value;

use serde_enum_str::Deserialize_enum_str as DeserializeString;
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

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
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
                .find(|model| model.id() == cursor)
                .ok_or(Error::NoModel)?;

            path.push(model.parent());
            cursor = model.parent()
        }

        path.reverse();

        Ok(path)
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

fn string_to_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
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

fn string_list_to_node_type_vector<'de, D>(deserializer: D) -> Result<Vec<NodeType>, D::Error>
where
    D: Deserializer<'de>,
{
    let string: &str = Deserialize::deserialize(deserializer)?;

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

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Project {
    name: String,
    detail_name: String,
    guid: String, // TODO: Maybe use guid struct?
    technical_name: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct GlobalVariable {
    namespace: String,
    description: String,
    variables: Vec<Variable>,
}

#[derive(Deserialize, Debug, Clone)]
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
            .get("Value")
            .ok_or(DeserializationError::KeyNotFound)?
            .as_str()
            .ok_or(DeserializationError::UnexpectedType)?;

        Ok(Variable {
            name: value
                .get("Variable")
                .ok_or(DeserializationError::KeyNotFound)?
                .as_str()
                .ok_or(DeserializationError::UnexpectedType)?
                .to_string(),

            value: match value
                .get("Type")
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
                .get("Description")
                .ok_or(DeserializationError::KeyNotFound)?
                .as_str()
                .ok_or(DeserializationError::UnexpectedType)?
                .to_string(),
        })
    }
}

// TODO: Perhaps combine Type + Value together?
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub enum VariableType {
    Boolean,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub enum VariableValue {
    // TODO: Remove Unknown and add deserialization error to be exhaustive
    Unknown,

    Boolean(bool),
    Integer(i32),
    String(String),
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
    item_type: Option<Type>,
}

#[derive(DeserializeString, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
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

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Package {
    pub name: String,
    pub description: String,
    pub is_default_package: bool,
    #[serde(deserialize_with = "deserialize_model")]
    pub models: Vec<Model>,
}

#[derive(Deserialize, Debug, Clone, IntoStaticStr)]
#[serde(rename_all = "PascalCase", tag = "Type", content = "Properties")]
pub enum Model {
    #[serde(rename_all = "PascalCase")]
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
    },

    #[serde(rename_all = "PascalCase")]
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

    #[serde(rename_all = "PascalCase")]
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

    #[serde(rename_all = "PascalCase")]
    Dialogue {
        id: Id,
        parent: Id,
        technical_name: String,

        preview_image: PreviewImage,
        attachments: Vec<Attachment>,
        display_name: String,
        text: String,
        color: Color,
        position: Point,
        size: Size,
        z_index: f32,
        short_id: ShortId,

        input_pins: Vec<Pin>,
        output_pins: Vec<Pin>,
    },

    #[serde(rename_all = "PascalCase")]
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

    #[serde(rename_all = "PascalCase")]
    Condition {
        id: Id,
        parent: Id,
        technical_name: String,

        display_name: String,
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

    #[serde(rename_all = "PascalCase")]
    UserFolder {
        id: Id,
        parent: Id,
        technical_name: String,
    },

    Custom(String, Value),
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
            serde_json::from_value(item.clone()).unwrap_or_else(|_error| {
                let properties = item
                    .get("Properties")
                    .expect("Properties to be part of a Model Value")
                    .clone();

                let kind = item
                    .get("Type")
                    .expect("Type to be part of a Model Value")
                    .as_str()
                    .expect("Type to be of type &str")
                    .to_owned();

                Model::Custom(kind, properties)
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
            | Model::UserFolder { id, .. } => id.clone(),

            Model::Custom(_, value) => match value.get("Id") {
                Some(value) => match value.as_str() {
                    Some(id) => Id(id.to_owned()),
                    None => Id("Custom Model did not have Id".to_owned()),
                },
                None => Id("Custom Model did not have Id".to_owned()),
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
            | Model::UserFolder { parent, .. } => parent.clone(),

            Model::Custom(_, value) => match value.get("Parent") {
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
            | Model::Condition { text, .. } => Some(text.to_string()),
            Model::UserFolder { .. } | Model::Custom(..) => None,
        }
    }

    pub fn display_name(&self) -> Option<String> {
        match self {
            Model::FlowFragment { display_name, .. }
            | Model::Hub { display_name, .. }
            | Model::Dialogue { display_name, .. }
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

            Model::UserFolder { .. } | Model::Comment { .. } | Model::Custom(..) => None,
        }
    }

    pub fn output_pins(&self) -> Option<&Vec<Pin>> {
        match self {
            Model::FlowFragment { output_pins, .. }
            | Model::DialogueFragment { output_pins, .. }
            | Model::Hub { output_pins, .. }
            | Model::Dialogue { output_pins, .. }
            | Model::Condition { output_pins, .. } => Some(output_pins),

            Model::UserFolder { .. } | Model::Comment { .. } | Model::Custom(..) => None,
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
    asset: AssetId,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Rectangle {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

#[derive(Deserialize, Debug, Clone)]
pub enum PreviewImageMode {
    FromAsset,
}

#[derive(Deserialize, Debug, Clone)]
pub struct AssetId(String);

#[derive(Deserialize, Debug, Clone)]
pub struct Color {
    r: f32,
    g: f32,
    b: f32,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ExternalId(String);

#[derive(Deserialize, Debug, Clone)]
pub struct Point {
    x: f32,
    y: f32,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Size {
    w: f32,
    h: f32,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ShortId(u32);

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Pin {
    pub text: String,
    pub id: Id,
    pub owner: Id,

    // NOTE: Sometimes certain pins don't have connections, default to an empty Vec<Connection> then (vec![])
    #[serde(default)]
    pub connections: Vec<Connection>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Connection {
    pub label: String,
    pub target_pin: Id,
    pub target: Id,
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
    Assets,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Hierarchy {
    pub id: Id,
    pub technical_name: String,
    #[serde(rename(deserialize = "Type"))]
    pub kind: Type,
    pub children: Option<Vec<Hierarchy>>,
}
