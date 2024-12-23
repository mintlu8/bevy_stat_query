use bevy_ecs::{component::Component, entity::Entity, system::RunSystemOnce, world::World};
use bevy_hierarchy::{BuildChildren, ChildBuild};
use bevy_reflect::TypePath;
use bevy_stat_query::{
    types::{StatInt, StatOnce},
    QualifierQuery, Querier, Stat, StatCache, StatEntities, StatEntity, StatExtension, StatQuery,
    StatQueryMut, StatStream, StatVTable, StatValue, StatValuePair,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Stat)]
#[stat(value = "StatOnce<i32>")]
pub struct StatDistance;

#[derive(Debug, Clone, Copy, Stat)]
#[stat(value = "StatOnce<Relation>")]
pub struct StatAllegiance;

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Default,
    Component,
    TypePath,
    Serialize,
    Deserialize,
)]
pub enum Allegiance {
    #[default]
    Player,
    AI,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Default,
    Component,
    TypePath,
    Serialize,
    Deserialize,
)]
pub enum Relation {
    #[default]
    Ally,
    Enemy,
}

#[derive(Component)]
pub struct Position([i32; 2]);

#[derive(Component)]
pub struct A;

#[derive(Component)]
pub struct B;

impl StatStream for Position {
    type Qualifier = bool;

    fn stream_relation(
        &self,
        other: &Self,
        _: Entity,
        _: Entity,
        _: &QualifierQuery<Self::Qualifier>,
        stat_value: &mut StatValuePair,
        _: Querier<Self::Qualifier>,
    ) {
        if let Some(v) = stat_value.is_then_cast(&StatDistance) {
            v.set((self.0[0] - other.0[0]).abs() + (self.0[1] - other.0[1]).abs())
        }
    }
}

impl StatStream for Allegiance {
    type Qualifier = bool;

    fn stream_relation(
        &self,
        other: &Self,
        _: Entity,
        _: Entity,
        _: &QualifierQuery<Self::Qualifier>,
        stat_value: &mut StatValuePair,
        _: Querier<Self::Qualifier>,
    ) {
        if let Some(v) = stat_value.is_then_cast(&StatAllegiance) {
            if self == other {
                v.set(Relation::Ally)
            } else {
                v.set(Relation::Enemy)
            }
        }
    }
}

#[derive(Component)]
pub struct DistanceAura(Entity);

#[derive(Component)]
pub struct AllegianceAura(i32, Entity);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StatEffects {
    Distance,
    Allegiance,
}

impl Stat for StatEffects {
    type Value = StatInt<i32>;

    fn name(&self) -> &'static str {
        match self {
            StatEffects::Distance => "DistanceEffect",
            StatEffects::Allegiance => "AllegianceEffect",
        }
    }

    fn values() -> impl IntoIterator<Item = Self> {
        [Self::Distance, Self::Allegiance]
    }

    fn vtable() -> &'static bevy_stat_query::StatVTable<Self> {
        static VTABLE: StatVTable<StatEffects> = StatVTable::of::<StatEffects>();
        &VTABLE
    }

    fn as_index(&self) -> u64 {
        match self {
            StatEffects::Distance => 0,
            StatEffects::Allegiance => 1,
        }
    }

    fn from_index(index: u64) -> Self {
        match index {
            0 => StatEffects::Distance,
            _ => StatEffects::Allegiance,
        }
    }
}

impl StatStream for DistanceAura {
    type Qualifier = bool;

    fn stream_stat(
        &self,
        entity: Entity,
        qualifier: &QualifierQuery<Self::Qualifier>,
        stat_value: &mut StatValuePair,
        querier: Querier<Self::Qualifier>,
    ) {
        if let Some(v) = stat_value.is_then_cast(&StatEffects::Distance) {
            // could panic or return default or write to ctx etc.
            let distance = querier
                .query_relation(self.0, entity, qualifier, &StatDistance)
                .unwrap()
                .unwrap();
            v.add(distance);
        }
    }
}

impl StatStream for AllegianceAura {
    type Qualifier = bool;

    fn stream_stat(
        &self,
        entity: Entity,
        qualifier: &QualifierQuery<Self::Qualifier>,
        stat_value: &mut StatValuePair,
        querier: Querier<Self::Qualifier>,
    ) {
        if let Some(v) = stat_value.is_then_cast(&StatEffects::Allegiance) {
            let distance = querier
                .query_relation(self.1, entity, qualifier, &StatAllegiance)
                .unwrap()
                .unwrap();
            v.add(match distance {
                Relation::Ally => self.0,
                Relation::Enemy => 0,
            });
        }
    }
}

#[test]
pub fn main() {
    let mut world = World::new();
    world.init_resource::<StatCache<bool>>();
    world.register_stat::<StatAllegiance>();
    world.register_stat::<StatDistance>();
    let a = world
        .spawn((StatEntity, Position([-1, 7]), Allegiance::Player, A))
        .id();
    let b = world
        .spawn((StatEntity, Position([4, 5]), Allegiance::AI, B))
        .id();
    world.entity_mut(a).with_children(|f| {
        f.spawn((DistanceAura(b), AllegianceAura(5, b)));
    });
    world.entity_mut(b).with_children(|f| {
        f.spawn((DistanceAura(a), AllegianceAura(7, a)));
    });
    let _ = world.run_system_once({
        move |query: StatEntities<bool>,
              mut allegiance: StatQueryMut<Allegiance>,
              mut position: StatQueryMut<Position>,
              allegiance_aura: StatQuery<AllegianceAura>,
              distance_aura: StatQuery<DistanceAura>| {
            macro_rules! querier {
                () => {
                    query
                        .join(&allegiance)
                        .join(&position)
                        .join(&allegiance_aura)
                        .join(&distance_aura)
                };
            }
            assert_eq!(
                querier!().eval_stat(a, &QualifierQuery::Aggregate(false), &StatEffects::Distance),
                Some(7)
            );
            assert_eq!(
                querier!().eval_stat(b, &QualifierQuery::Aggregate(false), &StatEffects::Distance),
                Some(7)
            );
            position.query.get_mut(a).unwrap().0[1] = -7;
            assert_eq!(
                querier!().eval_stat(a, &QualifierQuery::Aggregate(false), &StatEffects::Distance),
                Some(7)
            );
            assert_eq!(
                querier!().eval_stat(b, &QualifierQuery::Aggregate(false), &StatEffects::Distance),
                Some(7)
            );
            query.clear_cache();
            assert_eq!(
                querier!().eval_stat(a, &QualifierQuery::Aggregate(false), &StatEffects::Distance),
                Some(17)
            );
            assert_eq!(
                querier!().eval_stat(b, &QualifierQuery::Aggregate(false), &StatEffects::Distance),
                Some(17)
            );
            assert_eq!(
                querier!().eval_stat(
                    a,
                    &QualifierQuery::Aggregate(false),
                    &StatEffects::Allegiance
                ),
                Some(0)
            );
            assert_eq!(
                querier!().eval_stat(
                    b,
                    &QualifierQuery::Aggregate(false),
                    &StatEffects::Allegiance
                ),
                Some(0)
            );
            *allegiance.query.get_mut(b).unwrap() = Allegiance::Player;

            assert_eq!(
                querier!().eval_stat(
                    a,
                    &QualifierQuery::Aggregate(false),
                    &StatEffects::Allegiance
                ),
                Some(0)
            );
            assert_eq!(
                querier!().eval_stat(
                    b,
                    &QualifierQuery::Aggregate(false),
                    &StatEffects::Allegiance
                ),
                Some(0)
            );
            query.clear_cache();

            assert_eq!(
                querier!().eval_stat(
                    a,
                    &QualifierQuery::Aggregate(false),
                    &StatEffects::Allegiance
                ),
                Some(5)
            );
            assert_eq!(
                querier!().eval_stat(
                    b,
                    &QualifierQuery::Aggregate(false),
                    &StatEffects::Allegiance
                ),
                Some(7)
            );
        }
    });
}
