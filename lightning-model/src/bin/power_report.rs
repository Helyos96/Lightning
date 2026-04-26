use lightning_model::build::Build;
use lightning_model::calc::PowerReport;
use lightning_model::import;
use rayon::ThreadPoolBuilder;
use std::fs;

fn fetch() -> Result<Build, Box<dyn std::error::Error>> {
    const BUILD_PATH: &str = "build.json";

    if let Ok(data) = fs::read_to_string(BUILD_PATH) {
        if let Ok(player) = serde_json::from_str(&data) {
            return Ok(player);
        }
    }

    let player = import::character("Ben_#4007", "ben_im_jungroan")?;
    serde_json::to_writer(&fs::File::create(BUILD_PATH)?, &player)?;
    Ok(player)
}

fn main() {
    let build = fetch().expect("Failed to fetch build");
    let available_threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    let rayon_threads = (available_threads / 2).max(1);
    ThreadPoolBuilder::new().num_threads(rayon_threads).build_global().expect("Failed to initialize Rayon thread pool");

    for _i in 0..1000 {
        let _power_report = PowerReport::new_defence(&build, "Maximum Life");
    }
}
