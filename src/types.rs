use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct Run {
    pub id: u32,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]
pub struct Screenshot {
    pub name: String,
    pub hash: String,
}

#[derive(Serialize, Deserialize)]
pub struct ChangedScreenshot {
    pub name: String,
    pub hash: String,
    pub previous_hash: String,
    pub diff: Difference,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub enum Difference {
    Unknown,
    Processing,
    Done(f32),
}

#[derive(Serialize, Deserialize)]
pub struct ComparisonResult {
    pub project_id: Uuid,
    pub from: u32,
    pub to: u32,
    pub missing: Vec<Screenshot>,
    pub new: Vec<Screenshot>,
    pub diff: Vec<ChangedScreenshot>,
    pub unchanged: Vec<Screenshot>,
}

impl ComparisonResult {
    pub fn is_finished(&self) -> bool {
        self.diff
            .iter()
            .all(|d| matches!(d.diff, Difference::Done(_)))
    }
}
