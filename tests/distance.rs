use bevy_ecs::{component::Component, entity::Entity, system::{Query, RunSystemOnce}, world::World};
use bevy_hierarchy::BuildWorldChildren;
use bevy_reflect::TypePath;
use bevy_stat_query::{
   types::{StatInt, StatOnce}, ComponentStream, QualifierQuery, Querier, RelationStream, Stat, StatCache, StatEntity, StatExt, StatExtension, StatQuery, StatVTable, StatValue
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StatDistance;

impl Stat for StatDistance {
    type Value = StatOnce<i32>;

    fn name(&self) -> &'static str {
        "Distance"
    }

    fn values() -> impl IntoIterator<Item = Self> {
        [StatDistance]
    }
    
    fn vtable() -> &'static StatVTable<Self> {
        static VTABLE: StatVTable<StatDistance> = StatVTable::of::<StatDistance>();
        &VTABLE
    }
    
    fn as_index(&self) -> u64 {
        0
    }
    
    fn from_index(index: u64) -> Self {
        Self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StatAllegiance;

impl Stat for StatAllegiance {
    type Value = StatOnce<Relation>;

    fn name(&self) -> &'static str {
        "Allegiance"
    }

    fn values() -> impl IntoIterator<Item = Self> {
        [StatAllegiance]
    }
    
    fn vtable() -> &'static StatVTable<Self> {
        static VTABLE: StatVTable<StatAllegiance> = StatVTable::of::<StatAllegiance>();
        &VTABLE
    }
    
    fn as_index(&self) -> u64 {
        0
    }
    
    fn from_index(index: u64) -> Self {
        Self
    }
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

impl ComponentStream<bool> for &Position {
    type Cx = ();
    
    fn stream<S: Stat>(
        this: Entity,
        cx: &<Self::Cx as bevy_ecs::system::SystemParam>::Item<'_, '_>,
        component: <Self::ReadOnly as bevy_ecs::query::WorldQuery>::Item<'_>,
        qualifier: &QualifierQuery<bool>,
        stat: &S,
        value: &mut S::Value,
        querier: &impl bevy_stat_query::Querier<bool>,
    ) {}
}

impl RelationStream<bool> for &Position {

    fn relation<S: Stat>(
        this: <Self::ReadOnly as bevy_ecs::query::WorldQuery>::Item<'_>,
        other: <Self::ReadOnly as bevy_ecs::query::WorldQuery>::Item<'_>,
        cx: &<Self::Cx as bevy_ecs::system::SystemParam>::Item<'_, '_>,
        qualifier: &QualifierQuery<bool>,
        stat: &S,
        value: &mut S::Value,
        querier: &impl bevy_stat_query::Querier<bool>,
    ) {
        if let Some(v) = stat.is_then_cast(&StatDistance, value) {
            v.set((this.0[0] - other.0[0]).abs() + (this.0[1] - other.0[1]).abs())
        }
    }
}


impl ComponentStream<bool> for &mut Position {
    type Cx = ();
    
    fn stream<S: Stat>(
        this: Entity,
        cx: &<Self::Cx as bevy_ecs::system::SystemParam>::Item<'_, '_>,
        component: <Self::ReadOnly as bevy_ecs::query::WorldQuery>::Item<'_>,
        qualifier: &QualifierQuery<bool>,
        stat: &S,
        value: &mut S::Value,
        querier: &impl bevy_stat_query::Querier<bool>,
    ) {}
}

impl RelationStream<bool> for &mut Position {

    fn relation<S: Stat>(
        this: <Self::ReadOnly as bevy_ecs::query::WorldQuery>::Item<'_>,
        other: <Self::ReadOnly as bevy_ecs::query::WorldQuery>::Item<'_>,
        cx: &<Self::Cx as bevy_ecs::system::SystemParam>::Item<'_, '_>,
        qualifier: &QualifierQuery<bool>,
        stat: &S,
        value: &mut S::Value,
        querier: &impl bevy_stat_query::Querier<bool>,
    ) {
        if let Some(v) = stat.is_then_cast(&StatDistance, value) {
            v.set((this.0[0] - other.0[0]).abs() + (this.0[1] - other.0[1]).abs())
        }
    }
}


impl ComponentStream<bool> for &Allegiance {
    type Cx = ();
    
    fn stream<S: Stat>(
        this: Entity,
        cx: &<Self::Cx as bevy_ecs::system::SystemParam>::Item<'_, '_>,
        component: <Self::ReadOnly as bevy_ecs::query::WorldQuery>::Item<'_>,
        qualifier: &QualifierQuery<bool>,
        stat: &S,
        value: &mut S::Value,
        querier: &impl bevy_stat_query::Querier<bool>,
    ) {}
}

impl RelationStream<bool> for &Allegiance {

    fn relation<S: Stat>(
        this: <Self::ReadOnly as bevy_ecs::query::WorldQuery>::Item<'_>,
        other: <Self::ReadOnly as bevy_ecs::query::WorldQuery>::Item<'_>,
        cx: &<Self::Cx as bevy_ecs::system::SystemParam>::Item<'_, '_>,
        qualifier: &QualifierQuery<bool>,
        stat: &S,
        value: &mut S::Value,
        querier: &impl bevy_stat_query::Querier<bool>,
    ) {
        if let Some(v) = stat.is_then_cast(&StatAllegiance, value) {
            if this == other {
                v.set(Relation::Ally)
            } else {
                v.set(Relation::Enemy)
            }
        }
    }
}

impl ComponentStream<bool> for &mut Allegiance {
    type Cx = ();
    
    fn stream<S: Stat>(
        this: Entity,
        cx: &<Self::Cx as bevy_ecs::system::SystemParam>::Item<'_, '_>,
        component: <Self::ReadOnly as bevy_ecs::query::WorldQuery>::Item<'_>,
        qualifier: &QualifierQuery<bool>,
        stat: &S,
        value: &mut S::Value,
        querier: &impl bevy_stat_query::Querier<bool>,
    ) {}
}

impl RelationStream<bool> for &mut Allegiance {

    fn relation<S: Stat>(
        this: <Self::ReadOnly as bevy_ecs::query::WorldQuery>::Item<'_>,
        other: <Self::ReadOnly as bevy_ecs::query::WorldQuery>::Item<'_>,
        cx: &<Self::Cx as bevy_ecs::system::SystemParam>::Item<'_, '_>,
        qualifier: &QualifierQuery<bool>,
        stat: &S,
        value: &mut S::Value,
        querier: &impl bevy_stat_query::Querier<bool>,
    ) {
        if let Some(v) = stat.is_then_cast(&StatAllegiance, value) {
            if this == other {
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

impl ComponentStream<bool> for &DistanceAura {
    type Cx = ();

    fn stream<S: Stat>(
        this: Entity,
        cx: &<Self::Cx as bevy_ecs::system::SystemParam>::Item<'_, '_>,
        component: <Self::ReadOnly as bevy_ecs::query::WorldQuery>::Item<'_>,
        qualifier: &QualifierQuery<bool>,
        stat: &S,
        value: &mut S::Value,
        querier: &impl bevy_stat_query::Querier<bool>,
    ) {
        if let Some(v) = stat.is_then_cast(&StatEffects::Distance, value) {
            // could panic or return default or write to ctx etc.
            let distance = querier
                .query_relation(this, component.0, qualifier, &StatDistance)
                .unwrap()
                .unwrap();
            v.add(distance);
        }
    }
}

impl ComponentStream<bool> for &AllegianceAura {
    type Cx = ();

    fn stream<S: Stat>(
        this: Entity,
        cx: &<Self::Cx as bevy_ecs::system::SystemParam>::Item<'_, '_>,
        component: <Self::ReadOnly as bevy_ecs::query::WorldQuery>::Item<'_>,
        qualifier: &QualifierQuery<bool>,
        stat: &S,
        value: &mut S::Value,
        querier: &impl bevy_stat_query::Querier<bool>,
    ) {
        if let Some(v) = stat.is_then_cast(&StatEffects::Allegiance, value) {
            let distance = querier
                .query_relation(this, component.1, qualifier, &StatAllegiance)
                .unwrap()
                .unwrap();
            v.add(match distance {
                Relation::Ally => component.0,
                Relation::Enemy => 0,
            });
        }
    }
}

#[test]
pub fn main() {
    let mut world = World::new();
    world.init_resource::<StatCache<u32>>();
    world.register_stat::<StatAllegiance>();
    world.register_stat::<StatDistance>();
    let a = world
        .spawn((
            StatEntity,
            Position([-1, 7]),
            Allegiance::Player,
            A,
        ))
        .id();
    let b = world
        .spawn((
            StatEntity,
            Position([4, 5]),
            Allegiance::AI,
            B,
        ))
        .id();
    world.entity_mut(a).with_children(|f| {
        f.spawn((DistanceAura(b), AllegianceAura(5, b)));
    });
    world.entity_mut(b).with_children(|f| {
        f.spawn((DistanceAura(a), AllegianceAura(7, a)));
    });
    world.run_system_once({
        move |
            query: StatQuery<bool>, 
            mut allegiance: Query<&mut Allegiance>,
            mut position: Query<&mut Position>,
            allegiance_aura: Query<&AllegianceAura>,
            distance_aura: Query<&DistanceAura>,
        | {
            macro_rules! querier {
                () => {
                    query.with_relation(&allegiance)
                        .with_relation(&position)
                        .with_children(&allegiance_aura)
                        .with_children(&distance_aura)
                };
            }
            assert_eq!(querier!().query_eval(a, &QualifierQuery::Aggregate(false), &StatEffects::Distance), Some(7));
            assert_eq!(querier!().query_eval(b, &QualifierQuery::Aggregate(false), &StatEffects::Distance), Some(7));
            position.get_mut(a).unwrap().0[1] = -7;
            assert_eq!(querier!().query_eval(a, &QualifierQuery::Aggregate(false), &StatEffects::Distance), Some(7));
            assert_eq!(querier!().query_eval(b, &QualifierQuery::Aggregate(false), &StatEffects::Distance), Some(7));
            query.clear_cache();
            assert_eq!(querier!().query_eval(a, &QualifierQuery::Aggregate(false), &StatEffects::Distance), Some(17));
            assert_eq!(querier!().query_eval(b, &QualifierQuery::Aggregate(false), &StatEffects::Distance), Some(17));
        }
    })
    // assert_eq!(
    //     world.eval_stat::<MyQuerier, _>(
    //         a,
    //         &QualifierQuery::Aggregate(false),
    //         &StatEffects::Distance
    //     ),
    //     Some(7)
    // );
    // assert_eq!(
    //     world.eval_stat::<MyQuerier, _>(
    //         b,
    //         &QualifierQuery::Aggregate(false),
    //         &StatEffects::Distance
    //     ),
    //     Some(7)
    // );
    // world.entity_mut(a).get_mut::<Position>().unwrap().0[1] = -7;
    // assert_eq!(
    //     world.eval_stat::<MyQuerier, _>(
    //         a,
    //         &QualifierQuery::Aggregate(false),
    //         &StatEffects::Distance
    //     ),
    //     Some(7)
    // );
    // assert_eq!(
    //     world.eval_stat::<MyQuerier, _>(
    //         b,
    //         &QualifierQuery::Aggregate(false),
    //         &StatEffects::Distance
    //     ),
    //     Some(7)
    // );
    // world.clear_stat_cache::<bool>();
    // assert_eq!(
    //     world.eval_stat::<MyQuerier, _>(
    //         a,
    //         &QualifierQuery::Aggregate(false),
    //         &StatEffects::Distance
    //     ),
    //     Some(17)
    // );
    // assert_eq!(
    //     world.eval_stat::<MyQuerier, _>(
    //         b,
    //         &QualifierQuery::Aggregate(false),
    //         &StatEffects::Distance
    //     ),
    //     Some(17)
    // );

    // assert_eq!(
    //     world.eval_stat::<MyQuerier, _>(
    //         a,
    //         &QualifierQuery::Aggregate(false),
    //         &StatEffects::Allegiance
    //     ),
    //     Some(0)
    // );
    // assert_eq!(
    //     world.eval_stat::<MyQuerier, _>(
    //         b,
    //         &QualifierQuery::Aggregate(false),
    //         &StatEffects::Allegiance
    //     ),
    //     Some(0)
    // );
    // *world.entity_mut(b).get_mut::<Allegiance>().unwrap() = Allegiance::Player;
    // assert_eq!(
    //     world.eval_stat::<MyQuerier, _>(
    //         a,
    //         &QualifierQuery::Aggregate(false),
    //         &StatEffects::Allegiance
    //     ),
    //     Some(0)
    // );
    // assert_eq!(
    //     world.eval_stat::<MyQuerier, _>(
    //         b,
    //         &QualifierQuery::Aggregate(false),
    //         &StatEffects::Allegiance
    //     ),
    //     Some(0)
    // );
    // world.clear_stat_cache::<bool>();
    // assert_eq!(
    //     world.eval_stat::<MyQuerier, _>(
    //         a,
    //         &QualifierQuery::Aggregate(false),
    //         &StatEffects::Allegiance
    //     ),
    //     Some(5)
    // );
    // assert_eq!(
    //     world.eval_stat::<MyQuerier, _>(
    //         b,
    //         &QualifierQuery::Aggregate(false),
    //         &StatEffects::Allegiance
    //     ),
    //     Some(7)
    // );
}
