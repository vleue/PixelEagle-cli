use std::{
    collections::HashMap,
    fs::File,
    io::Read,
    path::Path,
    time::{Duration, Instant},
};

use reqwest::{Response, Url};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::types::{ComparisonResult, Run, Screenshot};

const MAX_RETRIES: u32 = 3;
const INITIAL_BACKOFF_MS: u64 = 1000;

#[cfg(not(feature = "self-hosted"))]
const DEFAULT_URL: &str = "https://pixel-eagle.com";

pub struct Project {
    pub(crate) url: Url,
    pub token: String,
}

impl Project {
    #[cfg(not(feature = "self-hosted"))]
    pub fn new(token: String) -> Self {
        Self {
            url: Url::parse(DEFAULT_URL).expect("Failed to parse Pixel Eagle URL"),
            token,
        }
    }

    #[cfg(feature = "self-hosted")]
    pub fn new(url: &str, token: String) -> Self {
        Self {
            url: Url::parse(url).expect("Failed to parse Pixel Eagle URL"),
            token,
        }
    }

    fn set_comparison_url(&self, comparison: &mut ComparisonResult) {
        comparison.project_url = Some(self.url.clone());
    }
}

fn is_retryable(status: reqwest::StatusCode) -> bool {
    matches!(
        status,
        reqwest::StatusCode::BAD_GATEWAY
            | reqwest::StatusCode::SERVICE_UNAVAILABLE
            | reqwest::StatusCode::GATEWAY_TIMEOUT
            | reqwest::StatusCode::REQUEST_TIMEOUT
    )
}

async fn send_with_retry<F>(build_request: F) -> Response
where
    F: Fn() -> reqwest::RequestBuilder,
{
    let mut last_error = None;
    for attempt in 0..=MAX_RETRIES {
        if attempt > 0 {
            let backoff = Duration::from_millis(INITIAL_BACKOFF_MS * 2u64.pow(attempt - 1));
            eprintln!(
                "Retrying in {}s (attempt {}/{})...",
                backoff.as_secs(),
                attempt + 1,
                MAX_RETRIES + 1
            );
            tokio::time::sleep(backoff).await;
        }

        match build_request().send().await {
            Ok(response) if is_retryable(response.status()) => {
                eprintln!(
                    "Server returned {} {}",
                    response.status().as_u16(),
                    response.status().canonical_reason().unwrap_or("")
                );
                last_error = Some(format!("HTTP {}", response.status().as_u16()));
                continue;
            }
            Ok(response) => return response,
            Err(err) if err.is_connect() || err.is_timeout() => {
                eprintln!("Connection error: {err}");
                last_error = Some(err.to_string());
                continue;
            }
            Err(err) => panic!("Failed to contact Pixel Eagle: {err}"),
        }
    }

    panic!(
        "Failed to contact Pixel Eagle after {} attempts: {}",
        MAX_RETRIES + 1,
        last_error.unwrap_or_default()
    );
}

impl Project {
    pub async fn create_run(&self, metadata: Option<String>) -> u32 {
        let metadata = if let Some(metadata) = metadata {
            serde_json::from_str::<HashMap<String, String>>(&metadata)
                .expect("Failed to parse metadata, expected a valid JSON string")
        } else {
            Default::default()
        };
        let url = self.url.join("runs").unwrap();
        let token = self.token.clone();
        let response = send_with_retry(|| {
            reqwest::Client::new()
                .post(url.clone())
                .header(
                    "User-Agent",
                    format!("pixeleagle-{}", env!("CARGO_PKG_VERSION")),
                )
                .bearer_auth(token.clone())
                .json(&metadata)
        })
        .await;

        if !response.status().is_success() {
            panic!("Failed to create run");
        }
        let run = response.json::<Run>().await.unwrap();
        run.id
    }

    pub async fn upload_screenshot(
        &self,
        run_id: u32,
        path: &str,
        name: Option<String>,
        clean_name: bool,
    ) {
        self.upload_screenshots(
            run_id,
            std::iter::once((path.to_string(), name)),
            clean_name,
        )
        .await;
    }

    pub async fn upload_screenshots(
        &self,
        run_id: u32,
        paths: impl Iterator<Item = (String, Option<String>)>,
        clean_name: bool,
    ) {
        for (path, name) in self
            .screenshots_need_upload(
                run_id,
                paths.map(|(path, name)| {
                    let name = name.unwrap_or_else(|| {
                        if clean_name {
                            std::path::Path::new(&path)
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or(&path)
                                .to_string()
                        } else {
                            path.to_string()
                        }
                    });
                    (path, name)
                }),
            )
            .await
        {
            let url = self.url.join(&format!("runs/{run_id}")).unwrap();
            let token = self.token.clone();
            let response = send_with_retry(|| {
                reqwest::Client::new()
                    .post(url.clone())
                    .header(
                        "User-Agent",
                        format!("pixeleagle-{}", env!("CARGO_PKG_VERSION")),
                    )
                    .bearer_auth(token.clone())
                    .multipart(
                        reqwest::multipart::Form::new()
                            .text("screenshot", name.clone())
                            .part(
                                "data",
                                reqwest::multipart::Part::bytes(std::fs::read(&path).unwrap())
                                    .file_name(path.clone()),
                            ),
                    )
            })
            .await;

            if !response.status().is_success() {
                panic!("Failed to upload screenshot");
            }
        }
    }

    pub async fn screenshot_need_upload(&self, run_id: u32, path: &str, name: String) -> bool {
        !self
            .screenshots_need_upload(run_id, std::iter::once((path.to_string(), name)))
            .await
            .is_empty()
    }

    pub async fn screenshots_need_upload(
        &self,
        run_id: u32,
        paths: impl Iterator<Item = (String, String)>,
    ) -> Vec<(String, String)> {
        let mut name_to_path = HashMap::new();
        let mut name_hash = vec![];
        for (path, name) in paths {
            name_to_path.insert(name.clone(), path.clone());
            let Ok(mut file) = File::open(path) else {
                continue;
            };
            let mut data = vec![];
            if file.read_to_end(&mut data).is_err() {
                continue;
            };
            let hash = Sha256::digest(data);
            name_hash.push((name, hex::encode(hash)));
        }

        let url = self.url.join(&format!("runs/{run_id}/hashes")).unwrap();
        let token = self.token.clone();
        let Ok(response) = reqwest::Client::new()
            .post(url)
            .header(
                "User-Agent",
                format!("pixeleagle-{}", env!("CARGO_PKG_VERSION")),
            )
            .bearer_auth(token)
            .json(&name_hash)
            .send()
            .await
        else {
            return name_to_path
                .iter()
                .map(|(name, path)| (path.clone(), name.clone()))
                .collect();
        };

        let Ok(list) = response.json::<Vec<Screenshot>>().await else {
            return name_to_path
                .iter()
                .map(|(name, path)| (path.clone(), name.clone()))
                .collect();
        };

        list.into_iter()
            .map(|screenshot| {
                (
                    name_to_path.remove(&screenshot.name).unwrap(),
                    screenshot.name.clone(),
                )
            })
            .collect()
    }

    pub async fn compare_two_runs(&self, run_id_a: u32, run_id_b: u32) -> ComparisonResult {
        let url = self
            .url
            .join(&format!("runs/{run_id_a}/compare/{run_id_b}"))
            .unwrap();
        let token = self.token.clone();
        let response = send_with_retry(|| {
            reqwest::Client::new()
                .post(url.clone())
                .header(
                    "User-Agent",
                    format!("pixeleagle-{}", env!("CARGO_PKG_VERSION")),
                )
                .bearer_auth(token.clone())
        })
        .await;

        if !response.status().is_success() {
            panic!("Failed to trigger comparison");
        }

        let mut comparison = response
            .json::<ComparisonResult>()
            .await
            .expect("Error parsing response");
        self.set_comparison_url(&mut comparison);
        comparison
    }

    pub async fn compare_two_runs_auto(
        &self,
        run_id_a: u32,
        metadata: HashMap<String, String>,
    ) -> ComparisonResult {
        let url = self
            .url
            .join(&format!("runs/{run_id_a}/compare/auto"))
            .unwrap();
        let token = self.token.clone();
        let response = send_with_retry(|| {
            reqwest::Client::new()
                .post(url.clone())
                .json(&metadata)
                .header(
                    "User-Agent",
                    format!("pixeleagle-{}", env!("CARGO_PKG_VERSION")),
                )
                .bearer_auth(token.clone())
        })
        .await;

        if !response.status().is_success() {
            panic!("Failed to trigger comparison");
        }

        let mut comparison = response
            .json::<ComparisonResult>()
            .await
            .expect("Error parsing response");
        self.set_comparison_url(&mut comparison);
        comparison
    }

    pub async fn get_comparison(&self, run_id_a: u32, run_id_b: u32) -> ComparisonResult {
        let url = self
            .url
            .join(&format!("runs/{run_id_a}/compare/{run_id_b}"))
            .unwrap();
        let token = self.token.clone();
        let response = send_with_retry(|| {
            reqwest::Client::new()
                .get(url.clone())
                .header(
                    "User-Agent",
                    format!("pixeleagle-{}", env!("CARGO_PKG_VERSION")),
                )
                .bearer_auth(token.clone())
        })
        .await;

        if !response.status().is_success() {
            panic!("Failed to get comparison");
        }

        let mut comparison = response
            .json::<ComparisonResult>()
            .await
            .expect("Error parsing response");
        self.set_comparison_url(&mut comparison);
        comparison
    }

    pub async fn wait_for_comparison(
        &self,
        mut comparison: ComparisonResult,
        wait_timeout: u32,
    ) -> ComparisonResult {
        let timeout = Duration::from_secs(wait_timeout as u64);
        let start = Instant::now();
        while !comparison.is_finished() {
            if start.elapsed() > timeout {
                println!("Timed out while waiting for comparison to finish");
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
            comparison = self.get_comparison(comparison.from, comparison.to).await;
        }
        comparison
    }

    pub fn print_comparison_json(&self, comparison: &ComparisonResult) {
        println!(
            "{}",
            serde_json::to_string(comparison).expect("Failed to serialize comparison")
        );
    }

    pub async fn download_screenshot(&self, project_id: Uuid, hash: &str, output: &Path) {
        let url = self
            .url
            .join(&format!("files/{project_id}/screenshot/{hash}"))
            .unwrap();
        let response = send_with_retry(|| reqwest::Client::new().get(url.clone())).await;

        if !response.status().is_success() {
            panic!(
                "Failed to download screenshot (HTTP {})",
                response.status().as_u16()
            );
        }

        let bytes = response.bytes().await.expect("Failed to read response");
        std::fs::write(output, &bytes).expect("Failed to write file");
    }

    pub async fn download_diff(&self, project_id: Uuid, hash_a: &str, hash_b: &str, output: &Path) {
        let url = self
            .url
            .join(&format!("files/{project_id}/diff/{hash_a}/{hash_b}"))
            .unwrap();
        let response = send_with_retry(|| reqwest::Client::new().get(url.clone())).await;

        if !response.status().is_success() {
            panic!(
                "Failed to download diff (HTTP {})",
                response.status().as_u16()
            );
        }

        let bytes = response.bytes().await.expect("Failed to read response");
        std::fs::write(output, &bytes).expect("Failed to write file");
    }

    pub fn print_comparison(&self, comparison: &ComparisonResult, with_details: bool) {
        println!("{}", comparison.get_url());

        if with_details {
            println!("{}", comparison.get_detail());
        }
    }
}
