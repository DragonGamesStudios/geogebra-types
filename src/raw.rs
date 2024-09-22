//! Raw GeoGebra structures.

use std::marker::PhantomData;

use serde::{de::Visitor, ser::SerializeMap, Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

/// Top-level element representing a Geogebra workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename = "geogebra")]
pub struct Geogebra {
    /// Format version. Schema states this attribute is deprecated, but Geogebra complains
    /// if it's not here. Library was tested with 5.0
    #[serde(rename = "@format")]
    pub format: String,
    /// Application to load this file in.
    #[serde(rename = "@app")]
    pub app: String,
    /// Subapplication to load this file in.
    #[serde(rename = "@subApp")]
    pub sub_app: String,
    /// The contained construction
    pub construction: Construction,
}

/// The construction contained in the workspace
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Construction {
    /// Construction's items
    #[serde(rename = "$value")]
    pub items: Vec<ConstructionItem>,
}

/// An item of the construction element.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConstructionItem {
    /// An element of the construction.
    Element(Element),
    /// A construction command.
    Command(Command),
    /// An expression
    Expression(Expression),
}

/// A construction element.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Element {
    /// Type of this element
    #[serde(rename = "@type")]
    pub type_: ElementType,
    /// The element's label
    #[serde(rename = "@label")]
    pub label: String,
    /// The element's caption
    pub caption: Option<Val<String>>,
    /// What should be displayed in place of the label
    pub label_mode: Val<LabelMode>,
    /// Which parts of the element should be shown
    pub show: Show,
    /// The element's coordinates
    pub coords: Option<Coords>,
    /// How to draw the line, if this is a line
    pub line_style: Option<LineStyle>,
    /// Color of this object
    pub obj_color: Option<ObjColorType>,
}

/// Type of an element
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ElementType {
    Point,
    Segment,
    Line,
    Numeric,
    Conic,
    Ray,
}

/// Style of a line
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
pub struct LineStyle {
    /// Thickness. 5 by default
    #[serde(rename = "@thickness")]
    pub thickness: Option<u16>,
    /// Stroke
    #[serde(rename = "@type")]
    pub type_: Option<LineType>,
    /// Opacity of this object
    #[serde(rename = "@opacity")]
    pub opacity: Option<f64>,
}

/// Stroke of a line
#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr)]
#[repr(u16)]
pub enum LineType {
    /// Solid line
    Solid = 0,
    /// Short dashes
    DashedShort = 10,
    /// Long dashes
    DashedLong = 15,
    /// Dots
    Dotted = 20,
    /// Dots and dashes
    DashedDotted = 30,
}

/// A value in an attribute
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Val<T> {
    #[serde(rename = "@val")]
    pub val: T,
}

impl<T> From<T> for Val<T> {
    fn from(value: T) -> Self {
        Self { val: value }
    }
}

/// What to display in place of an element's label
#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum LabelMode {
    /// Label
    Label,
    /// Label = Value
    LabelAndValue,
    /// Value
    Value,
    /// Caption
    Caption,
    /// Caption = Value
    CaptionAndValue,
}

/// What parts of an element should be shown.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Show {
    /// Show the object itself.
    #[serde(rename = "@object")]
    pub object: bool,
    /// Show the object's label
    #[serde(rename = "@label")]
    pub label: bool,
}

impl Show {
    /// Show only the object
    #[must_use]
    pub fn object() -> Self {
        Self {
            object: true,
            label: false,
        }
    }

    /// Show only the label
    #[must_use]
    pub fn label() -> Self {
        Self {
            object: false,
            label: true,
        }
    }

    /// Show both the object and its label
    #[must_use]
    pub fn object_and_label() -> Self {
        Self {
            object: true,
            label: true,
        }
    }

    /// Show neither the object nor its label
    #[must_use]
    pub fn none() -> Self {
        Self {
            object: false,
            label: false,
        }
    }
}

/// Cartesian coordinates of an element
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Coords {
    /// X coordinate
    #[serde(rename = "@x")]
    x: f64,
    /// Y coordinate
    #[serde(rename = "@y")]
    y: f64,
    /// Z coordinate
    #[serde(rename = "@z")]
    z: f64,
}

impl Coords {
    /// Create new coords from X and Y coordinates. Z is automatically set to 1.
    #[must_use]
    pub fn xy(x: f64, y: f64) -> Self {
        Self { x, y, z: 1.0 }
    }
}

/// A construction command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Command {
    /// The name of the command
    #[serde(rename = "@name")]
    pub name: String,
    /// Command inputs
    pub input: IndexedAttrs<String>,
    /// Command outputs
    pub output: IndexedAttrs<String>,
}

/// Helper for Geogebra's `a1`, `a2`, `a3` attributes in io.
#[derive(Debug, Clone)]
pub struct IndexedAttrs<T> {
    /// Attributes
    pub attrs: Vec<T>,
}

impl<T> From<Vec<T>> for IndexedAttrs<T> {
    fn from(value: Vec<T>) -> Self {
        Self { attrs: value }
    }
}

impl<T: Serialize> Serialize for IndexedAttrs<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = serializer.serialize_map(Some(self.attrs.len()))?;

        for (i, attr) in self.attrs.iter().enumerate() {
            s.serialize_entry(&format!("@a{i}"), attr)?;
        }

        s.end()
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for IndexedAttrs<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(IndexedAttrsVisitor(PhantomData))
    }
}

struct IndexedAttrsVisitor<T>(PhantomData<T>);

impl<'de, T: Deserialize<'de>> Visitor<'de> for IndexedAttrsVisitor<T> {
    type Value = IndexedAttrs<T>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a map")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut attrs = Vec::new();

        while let Some(v) = map.next_value()? {
            attrs.push(v);
        }

        Ok(IndexedAttrs { attrs })
    }
}

/// A Geogebra expression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Expression {
    /// Type of this expression
    #[serde(rename = "@type")]
    pub type_: ElementType,
    /// Label of this expression
    #[serde(rename = "@label")]
    pub label: String,
    /// The expression itself
    #[serde(rename = "@exp")]
    pub exp: String,
}

/// Color in Geogebra
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ObjColorType {
    /// The red channel
    #[serde(rename = "@r")]
    pub r: u8,
    /// The green channel
    #[serde(rename = "@g")]
    pub g: u8,
    /// The blue channel
    #[serde(rename = "@b")]
    pub b: u8,
}
