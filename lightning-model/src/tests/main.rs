use lightning_model::build::Build;
use lightning_model::calc::*;
use lightning_model::import;
use std::fs;

fn fetch() -> Result<Build, Box<dyn std::error::Error>> {
    const BUILD_PATH: &str = "build.json";

    if let Ok(data) = fs::read_to_string(BUILD_PATH) {
        if let Ok(player) = serde_json::from_str(&data) {
            return Ok(player);
        }
    }

    let player = import::character("Steelmage", "SteelMyTink")?;
    serde_json::to_writer(&fs::File::create(BUILD_PATH)?, &player)?;
    Ok(player)
}

#[test]
fn main() {
    let player = match fetch() {
        Ok(b) => b,
        Err(err) => {
            println!("{err}");
            return;
        }
    };

    //dbg!(&ITEMS["Blue Pearl Amulet"]);

    /*let fireball = Gem {
        id: "Fireball".to_string(),
        level: 20,
        qual: 0,
        alt_qual: 0,
    };
    let supports = vec![
        Gem {
            id: "SupportConcentratedEffect".to_string(),
            level: 20,
            qual: 0,
            alt_qual: 0,
        },
        Gem {
            id: "SupportFasterCast".to_string(),
            level: 20,
            qual: 0,
            alt_qual: 0,
        },
    ];*/

    //dbg!(player.tree.calc_mods());

    /*for gemlink in &player.gem_links {
        for gem in &gemlink.active_gems {
            calc_offense(&player, &gemlink.support_gems, gem);
        }
    }*/

    calc_defence(&player);

    //dbg!(&data::GEMS["Fireball"]);
}
