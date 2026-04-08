#[cfg(not(feature = "self-hosted"))]
use std::env;

#[cfg(not(feature = "self-hosted"))]
use pixeleagle_cli::Project;

#[cfg(not(feature = "self-hosted"))]
#[tokio::main]
async fn main() {
    let token = env::var("PIXEL_EAGLE_TOKEN").expect("PIXEL_EAGLE_TOKEN must be set");

    let project = Project::new(token);

    // Step 1: Create a new run
    let run_id = project.create_run(None).await;
    println!("Created run {run_id}");

    // Step 2: Find all PNG files in the current directory and upload them
    let pngs: Vec<_> = std::fs::read_dir(".")
        .expect("Failed to read current directory")
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("png") {
                Some(path.to_string_lossy().to_string())
            } else {
                None
            }
        })
        .collect();

    if pngs.is_empty() {
        println!("No PNG files found in current directory");
        return;
    }

    println!("Uploading {} screenshots...", pngs.len());
    project
        .upload_screenshots(run_id, pngs.iter().map(|path| (path.clone(), None)), false)
        .await;
    println!("Upload complete");

    // Step 3: Compare with the previous run
    let previous_run_id = run_id
        .checked_sub(1)
        .expect("No previous run to compare with");
    println!("Comparing run {run_id} with run {previous_run_id}...");
    let comparison = project.compare_two_runs(run_id, previous_run_id).await;

    // Step 4: Wait for the comparison to finish
    let comparison = project.wait_for_comparison(comparison, 300).await;

    // Print results
    project.print_comparison(&comparison, true);
}

#[cfg(feature = "self-hosted")]
fn main() {
    panic!("This example is not compatible with the `self-hosted` feature");
}
