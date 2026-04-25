use lightning_model::build::Build;
use lightning_model::calc::PowerReport;
use lightning_model::import;
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
    for _i in 0..100 {
        let _power_report = PowerReport::new_defence(&build, "Maximum Life");
    }
}
