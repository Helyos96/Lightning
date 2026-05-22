use enumflags2::{bitflags, BitFlags};
use crate::{build::{Defence, Slot, buff::Buff, property, stat::StatId}, data::{base_item::ItemClass, gem::GemTag, tree::NodeType}, item::JewelRadius, stackvec::StackVec};
use crate::tree::NodeMutation;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub enum Type {
    #[default]
    Base,
    Flat, // Unlike Base, Flat is not scaled by multipliers
    Inc,
    More,
    Override,
}

#[derive(Debug, Clone, Copy)]
pub enum Mutation {
    MultiplierStat((i64, StatId)),
    MultiplierStatLowest((i64, &'static [StatId])),
    MultiplierProperty((i64, property::Int)),
    MultiplierQuality(i64),
    StatPct((i64, StatId)),
    MultiplierSlotDefence((i64, Slot, Defence)),
    UpTo(i64),
    IncreasedEffect(i64),
    StatIncPct(i64, StatId),
    MultiplierOvercap(i64, StatId, StatId),
    StatMultExtra(StatId, i64),
}

impl Mutation {
    pub fn set_amount(&mut self, amount: i64) {
        match self {
            Mutation::MultiplierStat(mutation) => mutation.0 = amount,
            Mutation::MultiplierProperty(mutation) => mutation.0 = amount,
            Mutation::MultiplierStatLowest(mutation) => mutation.0 = amount,
            Mutation::StatPct(mutation) => mutation.0 = amount,
            Mutation::MultiplierSlotDefence(mutation) => mutation.0 = amount,
            Mutation::UpTo(mutation) => *mutation = amount,
            Mutation::IncreasedEffect(mutation) => *mutation = amount,
            Mutation::MultiplierQuality(mutation) => *mutation = amount,
            Mutation::StatIncPct(pct, _) => *pct = amount,
            Mutation::MultiplierOvercap(amnt, _, _) => *amnt = amount,
            Mutation::StatMultExtra(_, extra) => *extra = amount,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Condition {
    GreaterEqualProperty((i64, property::Int)),
    GreaterEqualStat((i64, StatId)),
    LesserEqualProperty((i64, property::Int)),
    LesserEqualStat((i64, StatId)),
    PropertyBool((bool, property::Bool)),
    WhileWielding(BitFlags<ItemClass>),
    WhileDualWielding,
    WhileDualWieldingItems(BitFlags<ItemClass>),
    SlotsHaveDefence((Defence, &'static [Slot])),
    SlotLesserEqualStats((Slot, i64, &'static [StatId])),
    GreaterEqualMasteryAllocated((&'static str, u32)),
    WithThisWeapon,
    NoFlaskActive,
}

#[derive(Default, Debug, Clone, Copy)]
pub enum Source {
    #[default]
    Innate,
    Node(u32),
    Mastery((u32, u32)),
    Item(Slot),
    Gem(&'static str),
    Custom(&'static str),
}

#[bitflags]
#[repr(u16)]
#[derive(Debug, Clone, Copy)]
pub enum ModFlag {
    Hit,
    Ailment,
    Bleed,
    Ignite,
    Poison,
    Aura,
    Buff,
    Curse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuildFlag {
    ItemsGrantLifeInsteadES,
    QualityNoDefences,
    EleMaxResHighest,
}

pub const MUTATIONS_COUNT: usize = 2;
pub const CONDITIONS_COUNT: usize = 2;

#[derive(Debug, Clone, Copy)]
pub struct ModStat {
    pub stat: StatId,
    pub typ: Type,
    pub amount: i64,
    pub revised_amount: Option<i64>,
    pub mutations: StackVec<Mutation, MUTATIONS_COUNT>,
}

impl ModStat {
    pub fn final_amount(&self) -> i64 {
        self.revised_amount.unwrap_or(self.amount)
    }
}

impl Default for ModStat {
    fn default() -> Self {
        ModStat { stat: StatId::Strength, typ: Type::Base, amount: 0, revised_amount: None, mutations: StackVec::default() }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ModEffect {
    Stat(ModStat),
    Allocate(u32),
    ForceBool(property::Bool, bool),
    MutateNode(NodeMutation, BitFlags<NodeType>),
    RingSize(JewelRadius),
    BuildFlag(BuildFlag),
    LevelOfGems(u32),
    QualityOfGems(i32),
    Buff(Buff),
}

impl Default for ModEffect {
    fn default() -> Self {
        ModEffect::Stat(ModStat::default())
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct Mod {
    pub effect: ModEffect,
    pub conditions: StackVec<Condition, CONDITIONS_COUNT>,
    pub tags: BitFlags<GemTag>,
    pub source: Source,
    pub weapons: BitFlags<ItemClass>,
    pub flags: BitFlags<ModFlag>,
}

impl Mod {
    pub fn stat(stat: StatId, typ: Type, amount: i64) -> Self {
        Self {
            effect: ModEffect::Stat(ModStat {
                stat, typ, amount, revised_amount: None, mutations: StackVec::default()
            }),
            ..Default::default()
        }
    }

    pub fn allocate(node: u32) -> Self {
        Self { effect: ModEffect::Allocate(node), ..Default::default() }
    }

    pub fn build_flag(flag: BuildFlag) -> Self {
        Self { effect: ModEffect::BuildFlag(flag), ..Default::default() }
    }

    pub fn gem_level(level: u32) -> Self {
        Self { effect: ModEffect::LevelOfGems(level), ..Default::default() }
    }

    pub fn buff(buff: Buff) -> Self {
        Self { effect: ModEffect::Buff(buff), ..Default::default() }
    }

    pub fn gem_quality(quality: i32) -> Self {
        Self { effect: ModEffect::QualityOfGems(quality), ..Default::default() }
    }

    pub fn ring_size(size: JewelRadius) -> Self {
        Self { effect: ModEffect::RingSize(size), ..Default::default() }
    }

    pub fn mutate_node(node_mutation: NodeMutation, allowed_types: BitFlags<NodeType>) -> Self {
        Self { effect: ModEffect::MutateNode(node_mutation, allowed_types), ..Default::default() }
    }

    pub fn force_bool(prop: property::Bool, val: bool) -> Self {
        Self { effect: ModEffect::ForceBool(prop, val), ..Default::default() }
    }

    pub fn with_mutations(mut self, mutations: StackVec<Mutation, MUTATIONS_COUNT>) -> Self {
        if let Some(stat) = self.as_stat_mut() {
            stat.mutations.extend(mutations);
        } else {
            eprintln!("Trying to add mutations to non-stat Mod");
        }
        self
    }

    pub fn with_conditions(mut self, conditions: StackVec<Condition, CONDITIONS_COUNT>) -> Self {
        self.conditions.extend(conditions);
        self
    }

    pub fn with_tags(mut self, tags: impl Into<BitFlags<GemTag>>) -> Self {
        self.tags.insert(tags);
        self
    }

    pub fn with_flags(mut self, flags: impl Into<BitFlags<ModFlag>>) -> Self {
        self.flags.insert(flags);
        self
    }

    pub fn with_weapons(mut self, weapons: impl Into<BitFlags<ItemClass>>) -> Self {
        self.weapons.insert(weapons);
        self
    }

    pub fn with_source(mut self, source: Source) -> Self {
        self.source = source;
        self
    }

    pub fn as_stat(&self) -> Option<&ModStat> {
        if let ModEffect::Stat(stat) = &self.effect {
            Some(stat)
        } else {
            None
        }
    }

    pub fn as_build_flag(&self) -> Option<&BuildFlag> {
        if let ModEffect::BuildFlag(flag) = &self.effect {
            Some(flag)
        } else {
            None
        }
    }

    pub fn as_gem_level(&self) -> Option<u32> {
        if let ModEffect::LevelOfGems(level) = &self.effect {
            Some(*level)
        } else {
            None
        }
    }

    pub fn as_buff(&self) -> Option<Buff> {
        if let ModEffect::Buff(buff) = &self.effect {
            Some(*buff)
        } else {
            None
        }
    }

    pub fn as_gem_quality(&self) -> Option<i32> {
        if let ModEffect::QualityOfGems(quality) = &self.effect {
            Some(*quality)
        } else {
            None
        }
    }

    pub fn as_ring_size(&self) -> Option<&JewelRadius> {
        if let ModEffect::RingSize(size) = &self.effect {
            Some(size)
        } else {
            None
        }
    }

    pub fn as_stat_mut(&mut self) -> Option<&mut ModStat> {
        if let ModEffect::Stat(stat) = &mut self.effect {
            Some(stat)
        } else {
            None
        }
    }

    pub fn as_allocate(&self) -> Option<u32> {
        if let ModEffect::Allocate(id) = &self.effect {
            Some(*id)
        } else {
            None
        }
    }

    pub fn as_node_mutation(&self) -> Option<(NodeMutation, BitFlags<NodeType>)> {
        if let ModEffect::MutateNode(mutation, allowed_types) = &self.effect {
            Some((*mutation, *allowed_types))
        } else {
            None
        }
    }
}
