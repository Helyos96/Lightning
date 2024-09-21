use crate::build::{Build, Stat, StatId};
use crate::gem::{Gem, GemTag};
use crate::modifier::{Mod, Type};
use rustc_hash::FxHashMap;

/*enum Val {
    int(i64),
    int100(i64),
}*/

pub fn calc_gem(build: &Build, support_gems: &Vec<Gem>, active_gem: &Gem) -> FxHashMap<&'static str, i64> {
    assert!(!active_gem.data().is_support);
    let mut ret = FxHashMap::default();
    //let display_name = &active_gem.data().base_item.as_ref().unwrap().display_name;

    let tags = &active_gem.data().tags;
    //let active_skill = active_gem.data.active_skill.as_ref().unwrap();
    let mut damage = vec![];

    let mut mods = build.calc_mods(true);
    mods.extend(active_gem.calc_mods());
    for support_gem in support_gems {
        mods.extend(support_gem.calc_mods());
    }

    let stats = build.calc_stats(&mods, tags);
    //dbg!(&stats);

    let damage_constants = [
        (StatId::FireDamage, StatId::MinimumFireDamage, StatId::MaximumFireDamage),
    ];

    for dt in &damage_constants {
        let dmg = build.calc_stat(dt.0, &mods, tags);
        let mut min = build.calc_stat(dt.1, &mods, tags);
        let mut max = build.calc_stat(dt.2, &mods, tags);

        if max.val() < min.val() {
            eprintln!("ERR: max ({}) < min ({})", min.val(), max.val());
        }

        if max.val() <= 0 {
            continue;
        }

        min.assimilate(&dmg);
        max.assimilate(&dmg);

        damage.push((min.val() + max.val()) / 2);
    }

    if let Some(mut time) = active_gem.data().cast_time {
        if tags.contains(&GemTag::Spell) {
            time = stats.stat(StatId::CastSpeed).calc_inv(time);
        } else if tags.contains(&GemTag::Attack) {
            time = stats.stat(StatId::AttackSpeed).calc_inv(time);
        }

        let mut dps = 0;
        for d in damage {
            dps += (d * 1000) / time;
        }
        ret.insert("DPS", dps);
        ret.insert("Speed", time);
    }
    ret
}

pub fn calc_defence(build: &Build) -> Vec<(String, i64)> {
    let mut ret = vec![];
    let mods = build.calc_mods(true);
    let stats = build.calc_stats(&mods, &hset![]);

    ret.push(("Maximum Life".to_string(), stats.stat(StatId::MaximumLife).val_rounded_up()));
    ret.push((
        "Fire Resistance".to_string(),
        stats.stat(StatId::FireResistance).val(),
    ));
    ret.push((
        "Cold Resistance".to_string(),
        stats.stat(StatId::ColdResistance).val(),
    ));
    ret.push((
        "Lightning Resistance".to_string(),
        stats.stat(StatId::LightningResistance).val(),
    ));
    ret.push((
        "Chaos Resistance".to_string(),
        stats.stat(StatId::ChaosResistance).val(),
    ));
    let mut life_regen = stats.stat(StatId::LifeRegeneration);
    let life_regen_pct = stats.stat(StatId::LifeRegenerationPct);
    let adjust_life_regen = Mod {
        stat: StatId::LifeRegeneration,
        typ: Type::Base,
        amount: (life_regen_pct.val() * stats.stat(StatId::MaximumLife).val_rounded_up()) / 10000,
        ..Default::default()
    };
    life_regen.adjust(Type::Base, adjust_life_regen.amount(), &adjust_life_regen);
    life_regen.assimilate(&stats.stat(StatId::LifeRegenerationRate));
    ret.push((
        "Life Regeneration".to_string(),
        life_regen.val(),
    ));
    ret.push(("Strength".to_string(), stats.stat(StatId::Strength).val()));
    ret.push(("Dexterity".to_string(), stats.stat(StatId::Dexterity).val()));
    ret.push(("Intelligence".to_string(), stats.stat(StatId::Intelligence).val()));

    ret
}
