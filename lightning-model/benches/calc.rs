use enumflags2::BitFlags;
use lightning_model::{build::Build, gem::Gem, modifier::CACHE};
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
    let player = fetch().expect("Failed to get a build");

    player.calc_mods(true);

    bencher.bench_local(|| {
        player.calc_mods(true);
    });
}

#[divan::bench]
fn calc_mods_uncached(bencher: divan::Bencher) {
    let player = fetch().expect("Failed to get a build");

    bencher.bench_local(|| {
        CACHE.lock().unwrap().clear();
        player.tree.force_regen_modcache();
        player.calc_mods(true);
    });
}

#[divan::bench]
fn calc_stats(bencher: divan::Bencher) {
    let player = fetch().expect("Failed to get a build");
    let mods = player.calc_mods(true);

    bencher.bench_local(|| {
        player.calc_stats(&mods, BitFlags::EMPTY, BitFlags::EMPTY);
    });
}

#[divan::bench]
fn calc_clone_build(bencher: divan::Bencher) {
    let player = fetch().expect("Failed to get a build");

    bencher.bench_local(|| {
        let _ = player.clone();
    });
}

#[divan::bench]
fn calc_gem(bencher: divan::Bencher) {
    let player = fetch().expect("Failed to get a build");
    let active_gem = player.gem_links[1].active_gems().nth(0).unwrap();
    let support_gems: Vec<&Gem> = player.gem_links[1].support_gems().collect();

    lightning_model::calc::calc_gem(&player, &support_gems, active_gem);

    bencher.bench_local(|| {
        lightning_model::calc::calc_gem(&player, &support_gems, active_gem);
    });
}

fn main() {
    divan::main();
}
