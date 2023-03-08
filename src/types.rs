#![allow(dead_code)]

use serde::{Deserialize, Deserializer};
use serde_json::Value;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct ArticyFile {
    pub settings: Settings,
    pub project: Project,
    pub global_variables: Vec<GlobalVariable>,
    pub object_definitions: Vec<ObjectDefinition>,
    pub packages: Vec<Package>,
    pub script_methods: Vec<ScriptMethod>,
    pub hierarchy: Hierarchy
}

#[derive(Deserialize, Debug)]
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

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Project {
    name: String,
    detail_name: String,
    guid: String, // TODO: Maybe use guid struct?
    technical_name: String
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct GlobalVariable {
    namespace: String,
    description: String,
    variables: Vec<Variable>
}

#[derive(Deserialize, Debug)]
#[serde(try_from = "Value")]
// TODO: Implement From<Value> to make custom Variable type
pub struct Variable {
    variable: String,
    // FIXME: rename this in serde
    // _type: VariableType,
    // value: VariableValue,
    value: VariableValue,
    description: String
}

#[derive(Debug)]
pub enum DeserializationError {
    KeyNotFound,
    UnexpectedType,
    ParseFailure
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
                DeserializationError::ParseFailure => "ParseFailure"
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
                variable: value.get("Variable")
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
                            // FIXME: Return error when state is invalid instead of defaulting to false
                            _ => VariableValue::Boolean(false)
                        },
                        "Integer" => match variable_value.parse::<i32>() {
                            Ok(integer) => VariableValue::Integer(integer),
                            Err(_) => return Err(DeserializationError::ParseFailure)
                        },
                        "String" => VariableValue::String(variable_value.to_string()),
                    _ => VariableValue::Unknown
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
#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub enum VariableType {
    Boolean
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub enum VariableValue {
    // TODO: Remove Unknown and add deserialization error to be exhaustive
    Unknown,

    Boolean(bool),
    Integer(i32),
    String(String)
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct ObjectDefinition {
    // TODO: rename in serde
    #[serde(rename(deserialize = "Type"))]
    property_type: String,//PropertyType,
    class: NodeType,//ObjectClass,
    properties: Option<Vec<ObjectProperty>>,
    // TODO: Implement other properties of ObjectDefinition or let it fail until all are met
    // display_names
    // values
    // display_name
}


#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub enum ObjectClass {
    Primitive,
    ArticyObject,
    Enum,
    FlowFragment,
    Dialogue,
    DialogueFragment,
    Hub,
    Comment,
    Jump,
    Entity,
    Location,
    Spot,
    Zone,
    Path,
    Link,
    Asset,
    Condition,
    Instruction,
    LocationText,
    LocationImage,
    Document,
    TextObject,
    UserFolder
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct ObjectProperty {
    property: String,
    // Rename in Serde
    #[serde(rename(deserialize = "Type"))]
    property_type: String,//PropertyType,
    item_type: Option<String>//PropertyType>
}

// FIXME: Perhaps combine with VariableType?
// FIXME: ObjectTypes dont need to be known at compile time i think
// #[derive(Deserialize, Debug)]
// #[serde(rename_all = "PascalCase")]
// enum PropertyType {
//     #[serde(alias = "float")]
//     Float,
//     Rect,
//     PreviewImageViewBoxModes,
//     #[serde(alias = "id")]
//     Id,
//     Point,
//     Array,
//     #[serde(alias = "string")]
//     String,
//     Color,
//     InputPin,
//     OutputPin,
//     Size,
//     PreviewImage,
//     Transformation,
//     OutgoingConnection,
//     IncomingConnection,
//     #[serde(rename(deserialize = "Script_Instruction"))]
//     ScriptInstruction,
//     #[serde(rename(deserialize = "Script_Condition"))]
//     ScriptCondition,
//     LocationAnchor,
//     LocationAnchorSize,
//
// }

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Package {
    pub name: String,
    pub description: String,
    pub is_default_package: bool,
    pub models: Vec<Model>
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Model {
    // FIXME: Rename in serde
    #[serde(rename(deserialize = "Type"))]
    pub model_type: ModelType,
    pub properties: ModelProperties
}


#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase", from = "&str")]
// // FIXME: Implement "Other" type and bring back in favor of String
pub enum ModelType {
    Float,
    FlowFragment,
    DialogueFragment,
    Dialogue,
    Jump,
    Hub,
    Condition,
    Entity,
    Comment,
    UserFolder,

    Custom(String)
}

impl From<&str> for ModelType {
    fn from(from: &str) -> ModelType {
        // TODO: Implement all keyword covered by Articy's spec
        match from {
            "Float" => ModelType::Float,
            "FlowFragment" => ModelType::FlowFragment,
            "DialogueFragment" => ModelType::DialogueFragment,
            "Dialogue" => ModelType::Dialogue,
            "Jump" => ModelType::Jump,
            "Hub" => ModelType::Hub,
            "Condition" => ModelType::Condition,
            "Entity" => ModelType::Entity,
            "Comment" => ModelType::Comment,
            "UserFolder" => ModelType::UserFolder,

            other => {
                // NOTE: This line is nice for catching additional keywords
                // NOTE: Maybe implement an error when finding a term that is expressed in PascalCase
                // println!("Writing {other} to Custom");
                ModelType::Custom(other.to_string())
            }
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct ModelProperties {
    pub technical_name: String,
    pub id: Id, // FIXME: Find a way to represent 0x0100-like id's
    pub parent: Id,
    // attachments: Vec<Attachment>,
    // pub display_name: String,
    // preview_image: PreviewImage,
    // color: Color,
    pub text: Option<String>,
    // external_id: ExternalId,
    // position: Position,
    // z_index: f32,
    // size: Size,
    // short_id: ShortId,
    pub display_name: Option<String>,
    pub expression: Option<String>,
    pub input_pins: Option<Vec<Pin>>,
    pub output_pins: Option<Vec<Pin>>
}

#[derive(Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Id(pub String);

impl Id {
    pub fn to_inner(&self) -> String {
        self.0.to_owned()
    }
}


#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Attachment;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct PreviewImage {
    view_box: Rectangle,
    mode: PreviewImageMode,
    asset: AssetId
}

#[derive(Deserialize, Debug)]
pub struct Rectangle {
    x: f32,
    y: f32,
    w: f32,
    h: f32
}

#[derive(Deserialize, Debug)]
pub enum PreviewImageMode {
    FromAsset
}

#[derive(Deserialize, Debug)]
pub struct AssetId(String);

#[derive(Deserialize, Debug)]
pub struct Color {
    r: f32, 
    g: f32,
    b: f32
}

#[derive(Deserialize, Debug)]
pub struct ExternalId(String);

#[derive(Deserialize, Debug)]
pub struct Position {
    x: f32,
    y: f32
}

#[derive(Deserialize, Debug)]
pub struct Size {
    w: f32,
    h: f32
}

#[derive(Deserialize, Debug)]
pub struct ShortId(i64);

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Pin {
   pub text: String,
   pub id: Id,
   pub owner: Id,
   pub connections: Option<Vec<Connection>>
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Connection {
    pub label: String,
    pub target_pin: Id,
    pub target: Id
}

#[derive(Deserialize, Debug)]
pub struct ScriptMethod;

#[derive(Deserialize, Debug, PartialEq)]
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

    // NOTE: Hypothethically the options below could be a seperate type
    ProjectSettingsFolder,
    ProjectSettingsFlow,
    ProjectSettingsGeneral,
    ProjectSettingsJourneys,
    ProjectSettingsLocation,
    VariableSet,
    Flow,
    Entities,
    EntitiesUserFolder,
    Locations,
    Documents,
    Journeys,
    TemplateDesign,
    Features,
    Feature,
    TraitTemplatesFolder,
    TraitTemplatesTypedFolder,
    EnumTraitTemplate,
    NumberTraitTemplate,
    ReferenceStripTraitTemplate,
    Templates,
    TemplateTypeFolder,
    Template,
    RuleSets,
    RuleSet,
    RuleSetPackage,
    AssetsUserFolder,

    // Technically part of ObjectClass
    Primitive,
    ArticyObject,
    Enum,
    FlowFragment,
    Dialogue,
    DialogueFragment,
    Hub,
    Comment,
    Jump,
    Entity,
    Location,
    Spot,
    Zone,
    Path,
    Link,
    Asset,
    Condition,
    Instruction,
    LocationText,
    LocationImage,
    Document,
    TextObject,
    UserFolder

}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Hierarchy {
    pub id: Id,
    pub technical_name: String,
    #[serde(rename(deserialize = "Type"))]
    pub node_type: NodeType,
    pub children: Option<Vec<Hierarchy>>
}

#[derive(Deserialize, Debug)]
pub enum HierarchyType {
}

// FIXME: Perhaps this can be the same as the NodeType
#[derive(Deserialize, Debug)]
pub enum HierarchyChildType {
    ProjectSettingsFolder
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_death_and_taxes_export() {
        let json = std::fs::read_to_string("./example_project.json")
            .expect("to be able to read the file");

        let _articy_file: ArticyFile = serde_json::from_str(&json)
            .expect("to be able to parse articy data");
    }
}
