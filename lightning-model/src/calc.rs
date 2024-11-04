use crate::build::{calc_stat, Build, Slot, Stat, StatId};
use crate::gem::{Gem, GemTag};
use crate::modifier::{Mod, Source, Type};
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
        (StatId::PhysicalDamage, StatId::MinPhysicalDamage, StatId::MaxPhysicalDamage, "physical"),
        (StatId::FireDamage, StatId::MinFireDamage, StatId::MaxFireDamage, "fire"),
    ];

    let stat_dmg = stats.stat(StatId::Damage);
    for dt in &damage_constants {
        let dmg = stats.stat(dt.0);
        let mut min = stats.stat(dt.1);
        let mut max = stats.stat(dt.2);

        if max.val() < min.val() {
            eprintln!("ERR: max ({}) < min ({})", min.val(), max.val());
        }

        if tags.contains(&GemTag::Attack) {
            // TODO: with physical damage, each weapon should be compared independently regarding
            // enemy armour, not added together.
            for (min_item, max_item) in [Slot::Weapon, Slot::Offhand].iter().map(|s| build.equipment.get(s)).flatten().map(|weapon| weapon.calc_dmg(dt.3)).flatten() {
                min.adjust(Type::Base, min_item, &Mod { amount: min_item, typ: Type::Base, source: Source::Item, ..Default::default() });
                max.adjust(Type::Base, max_item, &Mod { amount: max_item, typ: Type::Base, source: Source::Item, ..Default::default() });
            }
        }

        if max.val() <= 0 {
            continue;
        }

        min.assimilate(&dmg);
        min.assimilate(&stat_dmg);
        max.assimilate(&dmg);
        max.assimilate(&stat_dmg);

        if let Some(damage_effectiveness) = active_gem.damage_effectiveness() {
            damage.push((((min.val() + max.val()) / 2) * (100 + damage_effectiveness)) / 100);
        } else {
            damage.push((min.val() + max.val()) / 2);
        }
    }

    let time = {
        if tags.contains(&GemTag::Spell) {
            if let Some(time) = active_gem.data().cast_time {
                stats.stat(StatId::CastSpeed).val_custom_inv(time)
            } else {
                0
            }
        } else if tags.contains(&GemTag::Attack) {
            let mut div = 0;
            let mut time = 0;
            for slot in [Slot::Weapon, Slot::Offhand] {
                if let Some(weapon) = build.equipment.get(&slot) {
                    let weapon_restrictions = &active_gem.data().active_skill.as_ref().unwrap().weapon_restrictions;
                    if !weapon_restrictions.is_empty() && weapon_restrictions.contains(&weapon.data().item_class) {
                        if let Some(item_speed) = weapon.attack_speed() {
                            time += item_speed;
                            div += 1;
                        }
                    }
                }
            }
            if div > 0 {
                time /= div;
                stats.stat(StatId::AttackSpeed).val_custom_inv(time)
            } else {
                0
            }
        } else {
            0
        }
    };

    if time != 0 {
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
