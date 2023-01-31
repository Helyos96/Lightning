#[macro_use]
extern crate bencher;

use bencher::Bencher;
use lightning_model::build::Build;
use std::fs;

use lightning_model::import;

fn fetch() -> Result<Build, Box<dyn std::error::Error>> {
    const BUILD_PATH: &str = "build.json";

    if let Ok(data) = fs::read_to_string(BUILD_PATH) {
        if let Ok(player) = serde_json::from_str(&data) {
            return Ok(player);
        }
    }

    let player = import::character("Darkee", "BenQT")?;
    serde_json::to_writer(&fs::File::create(BUILD_PATH)?, &player)?;
    Ok(player)
}

fn calc_mods(bench: &mut Bencher) {
    let player = match fetch() {
        Ok(b) => b,
        Err(err) => {
            println!("{err}");
            return;
        }
    };
    player.calc_mods(true);

    bench.iter(|| {
        player.calc_mods(true);
    })
}

fn calc_stats(bench: &mut Bencher) {
    let player = match fetch() {
        Ok(b) => b,
        Err(err) => {
            println!("{err}");
            return;
        }
    };
    let mods = player.calc_mods(true);

    bench.iter(|| {
        player.calc_stats(&mods, &Default::default());
    })
}

benchmark_group!(benches, calc_mods, calc_stats);
benchmark_main!(benches);
