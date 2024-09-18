//! Raw GeoGebra structures.

use serde::{Deserialize, Serialize};

/// Top-level element representing a Geogebra workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename = "geogebra")]
pub struct Geogebra {
    /// Workspace settings
    pub kernel: Kernel,
}

/// Settings for the workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Kernel {
    /// TODO: What is this?
    pub digits: Val<i32>,
    /// Unit to use for displaying angles.
    pub angle_unit: Val<AngleUnit>,
    /// Coordinates display style. TODO: What is this?
    pub coord_style: Val<i32>,
}

/// A value in an attribute
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Val<T> {
    #[serde(rename = "@val")]
    pub val: T,
}

/// Unit of angle measurement.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AngleUnit {
    Degree,
    Radiant,
}
