use std::{collections::HashMap, path::PathBuf};

use clap::{Args, Parser, Subcommand, ValueEnum};
use pixeleagle_cli::Project;

#[derive(Parser, Debug)]
#[command(name = "pixeleagle")]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(long, env = "PIXEL_EAGLE_TOKEN")]
    token: String,

    #[cfg(feature = "self-hosted")]
    #[arg(long, env = "PIXEL_EAGLE_URL")]
    pixel_eagle_url: String,

    /// Output format
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Clone, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Create a new run
    ///
    /// The ID of the new run will be printed to stdout
    NewRun {
        /// Metadata about this run, used to identify it and trigger comparisons
        ///
        /// Example value: '{"commit": "b6bf94a"}'
        #[arg(long)]
        metadata: Option<String>,
    },
    /// Upload a screenshot to a run
    UploadScreenshot {
        /// ID of the run to upload the screenshot to
        #[arg(required = true)]
        run_id: u32,

        /// Path to the screenshot file to upload
        #[arg(required = true)]
        path: String,

        /// Name of the screenshot
        #[arg(long)]
        name: Option<String>,

        /// Use the file name without extension as the screenshot name
        #[arg(long)]
        clean_name: bool,
    },
    /// Upload a screenshot to a run
    UploadScreenshots {
        /// ID of the run to upload the screenshot to
        #[arg(required = true)]
        run_id: u32,

        /// Path to the screenshot file to upload
        #[arg(required = true)]
        path: Vec<String>,

        /// Use the file name without extension as the screenshot name
        #[arg(long)]
        clean_name: bool,
    },
    /// Trigger a run comparison with another
    CompareRun {
        /// ID of the first run
        #[arg(required = true)]
        run_id: u32,

        #[clap(flatten)]
        with: CompareWith,

        #[clap(flatten)]
        run_arguments: RunArguments,
    },
    /// Get a run comparison with another
    GetComparison {
        /// ID of the first run
        #[arg(required = true)]
        run_id: u32,

        #[clap(long)]
        with_run: u32,

        #[clap(flatten)]
        run_arguments: RunArguments,
    },
    /// Download a screenshot from a comparison
    ///
    /// Downloads the screenshot image for a given name from a comparison result.
    /// The project must be public.
    DownloadScreenshot {
        /// ID of the run containing the screenshot
        #[arg(required = true)]
        run_id: u32,

        /// ID of the run to compare with (needed to look up the screenshot)
        #[clap(long, required = true)]
        with_run: u32,

        /// Name of the screenshot to download
        #[arg(required = true)]
        name: String,

        /// Output file path
        #[arg(long, short)]
        output: PathBuf,
    },
    /// Download the diff image between two screenshots
    ///
    /// Downloads the visual diff for a given screenshot name from a comparison.
    /// The project must be public.
    DownloadDiff {
        /// ID of the first run
        #[arg(required = true)]
        run_id: u32,

        /// ID of the run to compare with
        #[clap(long, required = true)]
        with_run: u32,

        /// Name of the screenshot to download the diff for
        #[arg(required = true)]
        name: String,

        /// Output file path
        #[arg(long, short)]
        output: PathBuf,
    },
}

#[derive(Debug, Args)]
struct RunArguments {
    /// Print details of the comparison
    #[arg(long)]
    print_details: bool,

    /// Wait for the comparison to finish before returning
    #[arg(long)]
    wait: bool,

    /// Timeout when waiting
    #[arg(long, default_value = "300")]
    wait_timeout: u32,
}

#[derive(Debug, clap::Args)]
#[group(required = true, multiple = true)]
pub struct CompareWith {
    /// ID of the run to compare with
    #[clap(long, group = "a", group = "b")]
    with_run: Option<u32>,

    /// Metadata key:value pair to find a run matching this pair
    #[clap(long, group = "a", value_parser = key_value)]
    filter: Vec<(String, String)>,

    /// Metadata key to find a run with the same value for this metadata as the selected run
    #[clap(long, group = "b")]
    same: Vec<String>,
}

fn key_value(s: &str) -> Result<(String, String), String> {
    let parts = s.split(':').collect::<Vec<_>>();
    if parts.len() != 2 {
        Err(format!("Invalid key:value pair: '{s:?}'"))
    } else {
        Ok((parts[0].to_string(), parts[1].to_string()))
    }
}

impl CompareWith {
    pub fn as_hashmap(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        for (key, value) in &self.filter {
            map.insert(key.clone(), value.clone());
        }
        for key in &self.same {
            map.insert(key.clone(), "<equal>".to_string());
        }
        map
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    #[cfg(feature = "self-hosted")]
    let project = Project::new(&cli.pixel_eagle_url, cli.token);
    #[cfg(not(feature = "self-hosted"))]
    let project = Project::new(cli.token);

    let json = matches!(cli.format, OutputFormat::Json);

    match cli.command {
        Commands::NewRun { metadata } => {
            let id = project.create_run(metadata).await;
            if json {
                println!("{}", serde_json::json!({"id": id}));
            } else {
                println!("{id}");
            }
        }
        Commands::UploadScreenshots {
            run_id,
            path,
            clean_name,
        } => {
            project
                .upload_screenshots(
                    run_id,
                    path.into_iter().map(|path| (path, None)),
                    clean_name,
                )
                .await;
        }
        Commands::UploadScreenshot {
            run_id,
            path,
            name,
            clean_name,
        } => {
            project
                .upload_screenshot(run_id, &path, name, clean_name)
                .await;
        }
        Commands::CompareRun {
            run_arguments,
            run_id,
            with,
        } => {
            let mut comparison = if let Some(with_run) = with.with_run {
                project.compare_two_runs(run_id, with_run).await
            } else {
                project
                    .compare_two_runs_auto(run_id, with.as_hashmap())
                    .await
            };
            if run_arguments.wait {
                comparison = project
                    .wait_for_comparison(comparison, run_arguments.wait_timeout)
                    .await;
            }
            if json {
                project.print_comparison_json(comparison);
            } else {
                project.print_comparison(comparison, run_arguments.print_details);
            }
        }
        Commands::GetComparison {
            run_arguments,
            run_id,
            with_run,
        } => {
            let mut comparison = project.get_comparison(run_id, with_run).await;
            if run_arguments.wait {
                comparison = project
                    .wait_for_comparison(comparison, run_arguments.wait_timeout)
                    .await;
            }
            if json {
                project.print_comparison_json(comparison);
            } else {
                project.print_comparison(comparison, run_arguments.print_details);
            }
        }
        Commands::DownloadScreenshot {
            run_id,
            with_run,
            name,
            output,
        } => {
            let comparison = project.get_comparison(run_id, with_run).await;
            let hash = comparison
                .unchanged
                .iter()
                .chain(comparison.new.iter())
                .chain(comparison.missing.iter())
                .find(|s| s.name == name)
                .map(|s| s.hash.clone())
                .or_else(|| {
                    comparison
                        .diff
                        .iter()
                        .find(|s| s.name == name)
                        .map(|s| s.hash.clone())
                })
                .unwrap_or_else(|| panic!("Screenshot '{name}' not found in comparison"));

            project
                .download_screenshot(comparison.project_id, &hash, &output)
                .await;
            eprintln!("Downloaded to {}", output.display());
        }
        Commands::DownloadDiff {
            run_id,
            with_run,
            name,
            output,
        } => {
            let comparison = project.get_comparison(run_id, with_run).await;
            let changed = comparison
                .diff
                .iter()
                .find(|s| s.name == name)
                .unwrap_or_else(|| panic!("Screenshot '{name}' not found in changed screenshots"));

            project
                .download_diff(
                    comparison.project_id,
                    &changed.hash,
                    &changed.previous_hash,
                    &output,
                )
                .await;
            eprintln!("Downloaded to {}", output.display());
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    #[test]
    fn test_compare_parameters_run() {
        let cli = Cli::command().try_get_matches_from([
            "cmd",
            "--token",
            "a",
            "compare-run",
            "1",
            "--with-run",
            "2",
        ]);
        assert!(cli.is_ok());
    }

    #[test]
    fn test_compare_parameters_auto() {
        let cli = Cli::command().try_get_matches_from([
            "cmd",
            "--token",
            "a",
            "compare-run",
            "1",
            "--filter",
            "branch:main",
        ]);
        assert!(cli.is_ok());
        let cli = Cli::command().try_get_matches_from([
            "cmd",
            "--token",
            "a",
            "compare-run",
            "1",
            "--same",
            "os",
        ]);
        assert!(cli.is_ok());
        let command = [
            "cmd",
            "--token",
            "a",
            "compare-run",
            "1",
            "--filter",
            "branch:main",
            "--same",
            "os",
        ];
        let cli = Cli::command().try_get_matches_from(command);
        assert!(cli.is_ok());
        let cli = Cli::parse_from(command);
        let Commands::CompareRun {
            run_id,
            with,
            run_arguments: _,
        } = cli.command
        else {
            panic!("parsed the wrong command");
        };
        assert_eq!(run_id, 1);
        assert!(with.with_run.is_none());
        assert_eq!(with.same, vec!["os"]);
        assert_eq!(
            with.filter,
            vec![("branch".to_string(), "main".to_string())]
        );

        let command = [
            "cmd",
            "--token",
            "a",
            "compare-run",
            "1",
            "--filter",
            "branch:main",
            "--filter",
            "ref:gold",
            "--same",
            "os",
        ];
        let cli = Cli::command().try_get_matches_from(command);
        assert!(cli.is_ok());
        let cli = Cli::parse_from(command);
        let Commands::CompareRun {
            run_id,
            with,
            run_arguments: _,
        } = cli.command
        else {
            panic!("parsed the wrong command");
        };
        assert_eq!(run_id, 1);
        assert!(with.with_run.is_none());
        assert_eq!(with.same, vec!["os"]);
        assert_eq!(
            with.filter,
            vec![
                ("branch".to_string(), "main".to_string()),
                ("ref".to_string(), "gold".to_string())
            ]
        );
    }

    #[test]
    fn test_compare_parameters_excluded() {
        let cli = Cli::command().try_get_matches_from([
            "cmd",
            "--token",
            "a",
            "compare-run",
            "1",
            "--with-run",
            "2",
            "--filter",
            "branch:main",
            "--same",
            "os",
        ]);
        assert!(cli.is_err());
        let cli = Cli::command().try_get_matches_from([
            "cmd",
            "--token",
            "a",
            "compare-run",
            "1",
            "--with-run",
            "2",
            "--same",
            "os",
        ]);
        assert!(cli.is_err());
        let cli = Cli::command().try_get_matches_from([
            "cmd",
            "--token",
            "a",
            "compare-run",
            "1",
            "--with-run",
            "2",
            "--filter",
            "branch:main",
        ]);
        assert!(cli.is_err());
    }

    #[test]
    fn test_compare_parameters_at_least_one() {
        let cli = Cli::command().try_get_matches_from(["cmd", "--token", "a", "compare-run", "1"]);
        assert!(cli.is_err());
    }
}
