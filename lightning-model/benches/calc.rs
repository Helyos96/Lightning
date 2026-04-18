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

    let player = import::character("Ben_#4007", "ben_im_jungroan")?;
    serde_json::to_writer(&fs::File::create(BUILD_PATH)?, &player)?;
    Ok(player)
}

#[divan::bench]
fn calc_mods_cached(bencher: divan::Bencher) {
    let player = match fetch() {
        Ok(b) => b,
        Err(err) => {
            println!("{err}");
            return;
        }
    };

    player.calc_mods(true);

    bencher.bench_local(|| {
        player.calc_mods(true);
    });
}

#[divan::bench]
fn calc_mods_uncached(bencher: divan::Bencher) {
    let player = match fetch() {
        Ok(b) => b,
        Err(err) => {
            println!("{err}");
            return;
        }
    };

    bencher.bench_local(|| {
        player.calc_mods(true);
        CACHE.lock().unwrap().clear();
    });
}

#[divan::bench]
fn calc_stats(bencher: divan::Bencher) {
    let player = match fetch() {
        Ok(b) => b,
        Err(err) => {
            println!("{err}");
            return;
        }
    };
    let mods = player.calc_mods(true);

    bencher.bench_local(|| {
        player.calc_stats(&mods, BitFlags::EMPTY, BitFlags::EMPTY);
    });
}

#[divan::bench]
fn calc_clone_build(bencher: divan::Bencher) {
    let player = match fetch() {
        Ok(b) => b,
        Err(err) => {
            println!("{err}");
            return;
        }
    };

    bencher.bench_local(|| {
        let _ = player.clone();
    });
}

fn main() {
    divan::main();
}
