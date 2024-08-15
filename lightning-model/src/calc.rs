use crate::build::{Build, Stat};
use crate::gem::{Gem, Tag};
use crate::modifier::DamageType;
use rustc_hash::FxHashMap;

/*enum Val {
    int(i64),
    int100(i64),
}*/

pub fn calc_gem(build: &Build, support_gems: &Vec<Gem>, active_gem: &Gem) -> FxHashMap<&'static str, i64> {
    assert!(!active_gem.data().is_support);
    let mut ret = FxHashMap::default();
    let display_name = &active_gem.data().base_item.as_ref().unwrap().display_name;
    dbg!(display_name);

    let tags = &active_gem.data().tags;
    //let active_skill = active_gem.data.active_skill.as_ref().unwrap();
    let dts = vec![DamageType::Cold, DamageType::Fire];
    let mut damage: FxHashMap<DamageType, i64> = Default::default();

    let mut mods = build.calc_mods(true);
    mods.extend(active_gem.calc_mods());
    for support_gem in support_gems {
        mods.extend(support_gem.calc_mods());
    }

    let stats = build.calc_stats(&mods, tags);
    //dbg!(&stats);

    for dt in &dts {
        let dmg = build.calc_stat_dmg("damage", &mods, tags, *dt);
        let mut min = build.calc_stat_dmg("minimum damage", &mods, tags, *dt);
        let mut max = build.calc_stat_dmg("maximum damage", &mods, tags, *dt);

        assert!(min.val() <= max.val());
        if max.val() <= 0 {
            continue;
        }

        min.assimilate(&dmg);
        max.assimilate(&dmg);

        damage.insert(*dt, (min.val() + max.val()) / 2);
    }

    if let Some(mut time) = active_gem.data().cast_time {
        if tags.contains(&Tag::Spell) {
            if let Some(cast_speed) = stats.get("cast speed") {
                time = cast_speed.calc_inv(time);
            }
        } else if tags.contains(&Tag::Attack) {
            if let Some(attack_speed) = stats.get("attack speed") {
                time = attack_speed.calc_inv(time);
            }
        }

        let mut dps = 0;
        for d in damage.values() {
            dps += (*d * 1000) / time;
        }
        ret.insert("DPS", dps);
        ret.insert("Speed", time);
    }
    ret
}

pub fn calc_defence(build: &Build) -> Vec<(String, Stat)> {
    let mut ret = vec![];
    let mods = build.calc_mods(true);
    let stats = build.calc_stats(&mods, &hset![]);

    ret.push(("Maximum Life".to_string(), stats["maximum life"].clone()));
    ret.push((
        "Fire Resistance".to_string(),
        build.calc_stat_dmg("resistance", &mods, &hset![], DamageType::Fire),
    ));
    ret.push((
        "Cold Resistance".to_string(),
        build.calc_stat_dmg("resistance", &mods, &hset![], DamageType::Cold),
    ));
    ret.push((
        "Lightning Resistance".to_string(),
        build.calc_stat_dmg("resistance", &mods, &hset![], DamageType::Lightning),
    ));

    ret
}
