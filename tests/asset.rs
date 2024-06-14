use bevy::asset::{Asset, AssetApp, AssetPlugin, AssetServer, Assets, Handle};
use bevy_app::{App, Startup, Update};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::With,
    system::{Commands, Query, Res},
};
use bevy_hierarchy::BuildChildren;
use bevy_reflect::TypePath;
use bevy_stat_query::{
    querier, types::StatFloat, ExternalStream, QualifierQuery, QuerierRef, Stat, StatCache,
    StatEntity, StatExtension, StatQueryPlugin, StatValue,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Damage;

impl Stat for Damage {
    type Data = StatFloat<f32>;

    fn name(&self) -> &str {
        "Damage"
    }

    fn values() -> impl IntoIterator<Item = Self> {
        [Damage]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Defense;

impl Stat for Defense {
    type Data = StatFloat<f32>;

    fn name(&self) -> &str {
        "Defense"
    }

    fn values() -> impl IntoIterator<Item = Self> {
        [Defense]
    }
}

#[derive(Asset, TypePath)]
pub struct Weapon {
    pub damage: f32,
}

#[derive(Component)]
pub struct WeaponState {
    pub durability: f32,
}

#[derive(Component)]
pub struct A;

#[derive(Component)]
pub struct B;

type MyQualifier = u32;

impl ExternalStream<MyQualifier> for Weapon {
    type Ctx = Res<'static, Assets<Weapon>>;
    type QueryData = (&'static Handle<Weapon>, &'static WeaponState);
    fn stream(
        assets: &<Self::Ctx as bevy_ecs::system::SystemParam>::Item<'_, '_>,
        (handle, state): <Self::QueryData as bevy_ecs::query::WorldQuery>::Item<'_>,
        _: &QualifierQuery<MyQualifier>,
        stat: &mut bevy_stat_query::StatValuePair,
        _: &mut QuerierRef<'_, MyQualifier>,
    ) {
        if let Some(value) = stat.is(&Damage) {
            let Some(weapon) = assets.get(handle) else {
                return;
            };
            value.add(weapon.damage * state.durability)
        }
    }
}

querier!(pub MyQuerier {
    qualifier: MyQualifier,
    intrinsic: {},
    external: {
        Weapon
    }
});

#[test]
pub fn main() {
    App::new()
        .add_plugins(AssetPlugin::default())
        .add_plugins(StatQueryPlugin)
        .init_asset::<Weapon>()
        .register_stat::<Damage>()
        .register_stat::<Defense>()
        .add_systems(Startup, init)
        .add_systems(Update, query)
        .update();
}

fn init(mut commands: Commands, assets: Res<AssetServer>) {
    commands
        .spawn((StatEntity, StatCache::<MyQualifier>::default(), A))
        .with_children(|x| {
            x.spawn((
                assets.add(Weapon { damage: 4.0 }),
                WeaponState { durability: 0.5 },
            ));
        });
    commands
        .spawn((StatEntity, StatCache::<MyQualifier>::default(), B))
        .with_children(|x| {
            x.spawn((
                assets.add(Weapon { damage: 8.0 }),
                WeaponState { durability: 1.0 },
            ));
            x.spawn((
                assets.add(Weapon { damage: 6.0 }),
                WeaponState { durability: 2.0 },
            ));
        });
}

fn query(
    querier: MyQuerier,
    a: Query<Entity, (With<StatEntity>, With<A>)>,
    b: Query<Entity, (With<StatEntity>, With<B>)>,
) {
    let a = a.single();
    assert_eq!(
        querier.query_eval(a, &QualifierQuery::Aggregate(0u32), &Damage),
        Some(2.0)
    );
    assert_eq!(
        querier.query_eval(a, &QualifierQuery::Aggregate(0u32), &Defense),
        Some(0.0)
    );

    let b = b.single();
    assert_eq!(
        querier.query_eval(b, &QualifierQuery::Aggregate(0u32), &Damage),
        Some(20.0)
    );
    assert_eq!(
        querier.query_eval(b, &QualifierQuery::Aggregate(0u32), &Defense),
        Some(0.0)
    );
}
