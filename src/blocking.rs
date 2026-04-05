use std::{collections::HashMap, path::Path};

use uuid::Uuid;

use crate::types::ComparisonResult;

pub struct Project {
    inner: crate::Project,
    runtime: tokio::runtime::Runtime,
}

impl Project {
    pub fn new(url: &str, token: String) -> Self {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime");
        Self {
            inner: crate::Project::new(url, token),
            runtime,
        }
    }

    pub fn create_run(&self, metadata: Option<String>) -> u32 {
        self.runtime.block_on(self.inner.create_run(metadata))
    }

    pub fn upload_screenshot(&self, run_id: u32, path: &str, name: Option<String>) {
        self.runtime
            .block_on(self.inner.upload_screenshot(run_id, path, name));
    }

    pub fn upload_screenshots(
        &self,
        run_id: u32,
        paths: impl Iterator<Item = (String, Option<String>)>,
    ) {
        self.runtime
            .block_on(self.inner.upload_screenshots(run_id, paths));
    }

    pub fn screenshot_need_upload(&self, run_id: u32, path: &str, name: String) -> bool {
        self.runtime
            .block_on(self.inner.screenshot_need_upload(run_id, path, name))
    }

    pub fn screenshots_need_upload(
        &self,
        run_id: u32,
        paths: impl Iterator<Item = (String, String)>,
    ) -> Vec<(String, String)> {
        self.runtime
            .block_on(self.inner.screenshots_need_upload(run_id, paths))
    }

    pub fn compare_two_runs(&self, run_id_a: u32, run_id_b: u32) -> ComparisonResult {
        self.runtime
            .block_on(self.inner.compare_two_runs(run_id_a, run_id_b))
    }

    pub fn compare_two_runs_auto(
        &self,
        run_id_a: u32,
        metadata: HashMap<String, String>,
    ) -> ComparisonResult {
        self.runtime
            .block_on(self.inner.compare_two_runs_auto(run_id_a, metadata))
    }

    pub fn get_comparison(&self, run_id_a: u32, run_id_b: u32) -> ComparisonResult {
        self.runtime
            .block_on(self.inner.get_comparison(run_id_a, run_id_b))
    }

    pub fn wait_for_comparison(
        &self,
        comparison: ComparisonResult,
        wait_timeout: u32,
    ) -> ComparisonResult {
        self.runtime
            .block_on(self.inner.wait_for_comparison(comparison, wait_timeout))
    }

    pub fn download_screenshot(&self, project_id: Uuid, hash: &str, output: &Path) {
        self.runtime
            .block_on(self.inner.download_screenshot(project_id, hash, output));
    }

    pub fn download_diff(&self, project_id: Uuid, hash_a: &str, hash_b: &str, output: &Path) {
        self.runtime
            .block_on(self.inner.download_diff(project_id, hash_a, hash_b, output));
    }

    pub fn print_comparison_json(&self, comparison: ComparisonResult) {
        self.inner.print_comparison_json(comparison);
    }

    pub fn print_comparison(&self, comparison: ComparisonResult, with_details: bool) {
        self.inner.print_comparison(comparison, with_details);
    }
}
