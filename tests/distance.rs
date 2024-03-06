use bevy_ecs::{component::Component, entity::Entity, world::World};
use bevy_hierarchy::BuildWorldChildren;
use bevy_reflect::TypePath;
use bevy_stat_query::{querier, types::{StatInt, StatOnce}, ExternalStream, IntrinsicStream, QualifierQuery, QuerierRef, Stat, StatCache, StatEntity, StatExtension, StatValue};
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StatDistance;

impl Stat for StatDistance {
    type Data = StatOnce<i32>;

    fn name(&self) -> &str {
        "Distance"
    }

    fn values() -> impl IntoIterator<Item = Self> {
        [StatDistance]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StatAllegiance;

impl Stat for StatAllegiance {
    type Data = StatOnce<Relation>;

    fn name(&self) -> &str {
        "Allegiance"
    }

    fn values() -> impl IntoIterator<Item = Self> {
        [StatAllegiance]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Component, TypePath, Serialize, Deserialize)]
pub enum Allegiance {
    #[default]
    Player, AI,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Component, TypePath, Serialize, Deserialize)]
pub enum Relation {
    #[default]
    Ally, Enemy,
}

#[derive(Component)]
pub struct Position([i32; 2]);

#[derive(Component)]
pub struct A;

#[derive(Component)]
pub struct B;


impl ExternalStream<bool> for Position {
    type Ctx = ();
    type QueryData = &'static Position;
    fn stream (
        _: &<Self::Ctx as bevy_ecs::system::SystemParam>::Item<'_, '_>,
        _: <Self::QueryData as bevy_ecs::query::WorldQuery>::Item<'_>,
        _: &QualifierQuery<bool>,
        _: &mut bevy_stat_query::StatValuePair,
        _: &mut QuerierRef<'_, bool>
    ) {}
}

impl IntrinsicStream<bool> for Position {
    fn distance (
        _: &<Self::Ctx as bevy_ecs::system::SystemParam>::Item<'_, '_>,
        this: <Self::QueryData as bevy_ecs::query::WorldQuery>::Item<'_>,
        other: <Self::QueryData as bevy_ecs::query::WorldQuery>::Item<'_>,
        _: &QualifierQuery<bool>,
        stat: &mut bevy_stat_query::StatValuePair,
        _: &mut QuerierRef<bool>
    ) {
        stat.is_then(&StatDistance, |v| 
            v.set((this.0[0] - other.0[0]).abs() + (this.0[1] - other.0[1]).abs())
        );
    }
}

impl ExternalStream<bool> for Allegiance {
    type Ctx = ();
    type QueryData = &'static Allegiance;
    fn stream (
        _: &<Self::Ctx as bevy_ecs::system::SystemParam>::Item<'_, '_>,
        _: <Self::QueryData as bevy_ecs::query::WorldQuery>::Item<'_>,
        _: &QualifierQuery<bool>,
        _: &mut bevy_stat_query::StatValuePair,
        _: &mut QuerierRef<'_, bool>
    ) {}
}

impl IntrinsicStream<bool> for Allegiance {
    fn distance (
        _: &<Self::Ctx as bevy_ecs::system::SystemParam>::Item<'_, '_>,
        this: <Self::QueryData as bevy_ecs::query::WorldQuery>::Item<'_>,
        other: <Self::QueryData as bevy_ecs::query::WorldQuery>::Item<'_>,
        _: &QualifierQuery<bool>,
        stat: &mut bevy_stat_query::StatValuePair,
        _: &mut QuerierRef<bool>
    ) {
        stat.is_then(&StatAllegiance, |v| 
            if this == other {
                v.set(Relation::Ally)
            } else {
                v.set(Relation::Enemy)
            }
        );
    }
}

#[derive(Component)]
pub struct DistanceAura(Entity);

#[derive(Component)]
pub struct AllegianceAura(i32, Entity);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StatEffects{
    Distance,
    Allegiance
}

impl Stat for StatEffects {
    type Data = StatInt<i32>;

    fn name(&self) -> &str {
        match self {
            StatEffects::Distance => "DistanceEffect",
            StatEffects::Allegiance => "AllegianceEffect",
        }
    }

    fn values() -> impl IntoIterator<Item = Self> {
        [Self::Distance, Self::Allegiance]
    }
}

impl ExternalStream<bool> for DistanceAura {
    type Ctx = ();

    type QueryData = &'static DistanceAura;

    fn stream (
        _: &<Self::Ctx as bevy_ecs::system::SystemParam>::Item<'_, '_>,
        component: <Self::QueryData as bevy_ecs::query::WorldQuery>::Item<'_>,
        qualifier: &QualifierQuery<bool>,
        stat: &mut bevy_stat_query::StatValuePair,
        querier: &mut QuerierRef<'_, bool>,
    ) {
        dbg!(&stat);
        stat.is_then(&StatEffects::Distance, |s| {
            // could panic or return default or write to ctx etc.
            let distance = querier.query_distance(component.0, qualifier, &StatDistance)
                .unwrap().unwrap();
            s.add(distance);
        });
    }
}

impl ExternalStream<bool> for AllegianceAura {
    type Ctx = ();

    type QueryData = &'static AllegianceAura;

    fn stream (
        _: &<Self::Ctx as bevy_ecs::system::SystemParam>::Item<'_, '_>,
        component: <Self::QueryData as bevy_ecs::query::WorldQuery>::Item<'_>,
        qualifier: &QualifierQuery<bool>,
        stat: &mut bevy_stat_query::StatValuePair,
        querier: &mut QuerierRef<'_, bool>,
    ) {
        stat.is_then(&StatEffects::Allegiance, |s| {
            // could panic or return default or write to ctx etc.
            let distance = querier.query_distance(component.1, qualifier, &StatAllegiance)
                .unwrap().unwrap();
            s.add(match distance {
                Relation::Ally => component.0,
                Relation::Enemy => 0,
            });
        });
    }
}


querier!(pub MyQuerier {
    qualifier: bool,
    intrinsic: {
        Position,
        Allegiance
    },
    external: {
        DistanceAura,
        AllegianceAura,
    }
});

#[test]
pub fn main() {
    let mut world = World::new();
    world.register_stat::<StatAllegiance>();
    world.register_stat::<StatDistance>();
    let a = world.spawn((
        StatEntity,
        StatCache::<bool>::default(),
        Position([-1, 7]),
        Allegiance::Player,
        A,
    )).id();
    let b = world.spawn((
        StatEntity,
        StatCache::<bool>::default(),
        Position([4, 5]),
        Allegiance::AI,
        B,
    )).id();
    world.entity_mut(a).with_children(|f| {
        f.spawn((
            DistanceAura(b),
            AllegianceAura(5, b),
        ));
    });
    world.entity_mut(b).with_children(|f| {
        f.spawn((
            DistanceAura(a),
            AllegianceAura(7, a),
        ));
    });
    assert_eq!(world.query_eval_stat::<MyQuerier, _>(a, &QualifierQuery::Aggregate(false), &StatEffects::Distance), Some(7));
    assert_eq!(world.query_eval_stat::<MyQuerier, _>(b, &QualifierQuery::Aggregate(false), &StatEffects::Distance), Some(7));
    world.entity_mut(a).get_mut::<Position>().unwrap().0[1] = -7;
    assert_eq!(world.query_eval_stat::<MyQuerier, _>(a, &QualifierQuery::Aggregate(false), &StatEffects::Distance), Some(7));
    assert_eq!(world.query_eval_stat::<MyQuerier, _>(b, &QualifierQuery::Aggregate(false), &StatEffects::Distance), Some(7));
    world.clear_stat_cache::<bool>();
    assert_eq!(world.query_eval_stat::<MyQuerier, _>(a, &QualifierQuery::Aggregate(false), &StatEffects::Distance), Some(17));
    assert_eq!(world.query_eval_stat::<MyQuerier, _>(b, &QualifierQuery::Aggregate(false), &StatEffects::Distance), Some(17));

    assert_eq!(world.query_eval_stat::<MyQuerier, _>(a, &QualifierQuery::Aggregate(false), &StatEffects::Allegiance), Some(0));
    assert_eq!(world.query_eval_stat::<MyQuerier, _>(b, &QualifierQuery::Aggregate(false), &StatEffects::Allegiance), Some(0));
    *world.entity_mut(b).get_mut::<Allegiance>().unwrap() = Allegiance::Player;
    assert_eq!(world.query_eval_stat::<MyQuerier, _>(a, &QualifierQuery::Aggregate(false), &StatEffects::Allegiance), Some(0));
    assert_eq!(world.query_eval_stat::<MyQuerier, _>(b, &QualifierQuery::Aggregate(false), &StatEffects::Allegiance), Some(0));
    world.clear_stat_cache::<bool>();
    assert_eq!(world.query_eval_stat::<MyQuerier, _>(a, &QualifierQuery::Aggregate(false), &StatEffects::Allegiance), Some(5));
    assert_eq!(world.query_eval_stat::<MyQuerier, _>(b, &QualifierQuery::Aggregate(false), &StatEffects::Allegiance), Some(7));
}
