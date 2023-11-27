use serde_derive::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Deserialize, Serialize, Clone)]
pub struct HOSConnectionList {
    pub connections: Vec<(String, String)>,
}
