#[macro_use]
extern crate bencher;

use bencher::Bencher;
use enumflags2::BitFlags;
use lightning_model::{build::Build, modifier::CACHE};
use std::fs;

use lightning_model::import;

fn fetch() -> Result<Build, Box<dyn std::error::Error>> {
    const BUILD_PATH: &str = "build.json";

    if let Ok(data) = fs::read_to_string(BUILD_PATH) {
        if let Ok(player) = serde_json::from_str(&data) {
            return Ok(player);
        }
    }

    let player = import::character("Ben_#4007", "BenQT")?;
    serde_json::to_writer(&fs::File::create(BUILD_PATH)?, &player)?;
    Ok(player)
}

fn calc_mods_cached(bench: &mut Bencher) {
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

fn calc_mods_uncached(bench: &mut Bencher) {
    let player = match fetch() {
        Ok(b) => b,
        Err(err) => {
            println!("{err}");
            return;
        }
    };

    bench.iter(|| {
        player.calc_mods(true);
        CACHE.lock().unwrap().clear();
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
        player.calc_stats(&mods, BitFlags::empty());
    })
}

benchmark_group!(benches, calc_mods_cached, calc_mods_uncached, calc_stats);
benchmark_main!(benches);
