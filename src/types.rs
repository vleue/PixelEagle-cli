use std::fmt::Write;

use reqwest::Url;
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
    #[serde(skip)]
    pub(crate) project_url: Option<Url>,
}

impl ComparisonResult {
    pub fn is_finished(&self) -> bool {
        self.diff
            .iter()
            .all(|d| matches!(d.diff, Difference::Done(_)))
    }

    pub fn get_detail(&self) -> String {
        let mut detail = String::new();
        writeln!(
            &mut detail,
            "{} screenshots, {} unchanged",
            self.new.len() + self.diff.len() + self.unchanged.len(),
            self.unchanged.len()
        )
        .unwrap();

        if !self.missing.is_empty() {
            writeln!(&mut detail, "{} missing", self.missing.len()).unwrap();
            for screenshot in &self.missing {
                writeln!(&mut detail, " - {}", screenshot.name).unwrap();
            }
        }
        if !self.new.is_empty() {
            writeln!(&mut detail, "{} new", self.new.len()).unwrap();
            for screenshot in &self.new {
                writeln!(&mut detail, " - {}", screenshot.name).unwrap();
            }
        }
        if !self.diff.is_empty() {
            let diff: Vec<_> = self
                .diff
                .iter()
                .filter(|p| matches!(p.diff, Difference::Done(_)))
                .collect();
            if !diff.is_empty() {
                writeln!(&mut detail, "{} changed", diff.len()).unwrap();
                for screenshot in &diff {
                    let Difference::Done(x) = screenshot.diff else {
                        continue;
                    };
                    writeln!(&mut detail, " - {} ({:.2}%)", screenshot.name, x * 100.0).unwrap();
                }
            }
            let waiting: Vec<_> = self
                .diff
                .iter()
                .filter(|p| !matches!(p.diff, Difference::Done(_)))
                .collect();
            if !waiting.is_empty() {
                writeln!(&mut detail, "{} processing", waiting.len()).unwrap();
                for screenshot in &waiting {
                    writeln!(&mut detail, " - {}", screenshot.name).unwrap();
                }
            }
        }

        detail
    }

    pub fn get_url(&self) -> String {
        self.project_url
            .as_ref()
            .expect("Project URL not set on ComparisonResult")
            .join(&format!(
                "project/{}/run/{}/compare/{}",
                self.project_id, self.from, self.to,
            ))
            .unwrap()
            .to_string()
    }
}
