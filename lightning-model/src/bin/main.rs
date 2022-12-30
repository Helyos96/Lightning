use lightning_model::calc::*;
use lightning_model::util;

fn main() {
    let player = match util::load_or_fetch() {
        Ok(b) => b,
        Err(err) => {
            println!("{}", err);
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
