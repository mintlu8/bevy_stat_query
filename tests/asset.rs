use bevy::{
    asset::{Asset, AssetApp, AssetPlugin, AssetServer, Assets, Handle},
    prelude::Single,
};
use bevy_app::{App, Startup, Update};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{QueryData, With},
    system::{Commands, Res},
};
use bevy_hierarchy::{BuildChildren, ChildBuild};
use bevy_reflect::TypePath;
use bevy_stat_query::{
    types::StatFloat, QualifierQuery, Querier, QueryStream, Stat, StatEntities, StatEntity,
    StatExtension, StatQuery, StatVTable, StatValue, StatValuePair,
};

#[derive(Debug, Clone, Copy, Stat)]
#[stat(value = "StatFloat<f32>")]
pub struct Damage;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Defense;

impl Stat for Defense {
    type Value = StatFloat<f32>;

    fn name(&self) -> &'static str {
        "Defense"
    }

    fn values() -> impl IntoIterator<Item = Self> {
        [Defense]
    }

    fn vtable() -> &'static StatVTable<Defense> {
        static VTABLE: StatVTable<Defense> = StatVTable::of::<Defense>();
        &VTABLE
    }

    fn as_index(&self) -> u64 {
        0
    }

    fn from_index(_: u64) -> Self {
        Defense
    }
}

#[derive(Asset, TypePath)]
pub struct Weapon {
    pub damage: f32,
}

#[derive(Debug, Component)]
pub struct WeaponHandle(Handle<Weapon>);

#[derive(Component)]
pub struct WeaponState {
    pub durability: f32,
}

#[derive(Component)]
pub struct A;

#[derive(Component)]
pub struct B;

#[derive(QueryData)]
pub struct WeaponQuery {
    weapon: &'static WeaponHandle,
    state: &'static WeaponState,
}

impl QueryStream for WeaponQuery {
    type Qualifier = u32;
    type Context = Res<'static, Assets<Weapon>>;
    type Query = WeaponQuery;

    fn stream_stat(
        query: <<Self::Query as QueryData>::ReadOnly as bevy_ecs::query::WorldQuery>::Item<'_>,
        context: &<Self::Context as bevy_ecs::system::SystemParam>::Item<'_, '_>,
        _: Entity,
        _: &QualifierQuery<Self::Qualifier>,
        stat_value: &mut StatValuePair,
        _: Querier<Self::Qualifier>,
    ) {
        if let Some(value) = stat_value.is_then_cast(&Damage) {
            let Some(weapon) = context.get(query.weapon.0.id()) else {
                return;
            };
            value.add(weapon.damage * query.state.durability)
        }
    }
}

#[test]
pub fn asset_test() {
    App::new()
        .add_plugins(AssetPlugin::default())
        .init_asset::<Weapon>()
        .register_stat::<Damage>()
        .register_stat::<Defense>()
        .add_systems(Startup, init)
        .add_systems(Update, query)
        .update();
}

fn init(mut commands: Commands, assets: Res<AssetServer>) {
    commands.spawn((StatEntity, A)).with_children(|x| {
        x.spawn((
            WeaponHandle(assets.add(Weapon { damage: 4.0 })),
            WeaponState { durability: 0.5 },
        ));
    });
    commands.spawn((StatEntity, B)).with_children(|x| {
        x.spawn((
            WeaponHandle(assets.add(Weapon { damage: 8.0 })),
            WeaponState { durability: 1.0 },
        ));
        x.spawn((
            WeaponHandle(assets.add(Weapon { damage: 6.0 })),
            WeaponState { durability: 2.0 },
        ));
    });
}

fn query(
    querier: StatEntities<u32>,
    weapon_query: StatQuery<WeaponQuery>,
    a: Single<Entity, (With<StatEntity>, With<A>)>,
    b: Single<Entity, (With<StatEntity>, With<B>)>,
) {
    let querier = querier.join(&weapon_query);
    assert_eq!(
        querier.eval_stat(*a, &QualifierQuery::Aggregate(0u32), &Damage),
        Some(2.0)
    );
    assert_eq!(
        querier.eval_stat(*a, &QualifierQuery::Aggregate(0u32), &Defense),
        Some(0.0)
    );

    assert_eq!(
        querier.eval_stat(*b, &QualifierQuery::Aggregate(0u32), &Damage),
        Some(20.0)
    );
    assert_eq!(
        querier.eval_stat(*b, &QualifierQuery::Aggregate(0u32), &Defense),
        Some(0.0)
    );
}
