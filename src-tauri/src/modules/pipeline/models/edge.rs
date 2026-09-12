use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub enum EdgeType {
    #[default]
    #[serde(rename = "next")]
    Next,
    #[serde(rename = "error")]
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub edge_type: EdgeType,
}

#[cfg(test)]
mod tests;
