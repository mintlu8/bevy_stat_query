use bevy::asset::{Asset, AssetApp, AssetPlugin, AssetServer, Assets, Handle};
use bevy_app::{App, Startup, Update};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{QueryData, With},
    system::{Commands, Query, Res},
};
use bevy_hierarchy::BuildChildren;
use bevy_reflect::TypePath;
use bevy_stat_query::{
    types::StatFloat, ComponentStream, QualifierQuery, Querier, Stat, StatEntity, StatExt,
    StatExtension, StatQuery, StatVTable, StatValue,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Damage;

impl Stat for Damage {
    type Value = StatFloat<f32>;

    fn name(&self) -> &'static str {
        "Damage"
    }

    fn values() -> impl IntoIterator<Item = Self> {
        [Damage]
    }

    fn vtable() -> &'static StatVTable<Damage> {
        static VTABLE: StatVTable<Damage> = StatVTable::of::<Damage>();
        &VTABLE
    }

    fn as_index(&self) -> u64 {
        0
    }

    fn from_index(_: u64) -> Self {
        Damage
    }
}

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

#[derive(Component)]
pub struct WeaponState {
    pub durability: f32,
}

#[derive(Component)]
pub struct A;

#[derive(Component)]
pub struct B;

#[derive(QueryData)]
pub struct WeaponHandle {
    weapon: &'static Handle<Weapon>,
    state: &'static WeaponState,
}

impl ComponentStream<u32> for WeaponHandle {
    type Cx = Res<'static, Assets<Weapon>>;

    fn stream<S: Stat>(
        cx: &<Self::Cx as bevy_ecs::system::SystemParam>::Item<'_, '_>,
        component: <Self::ReadOnly as bevy_ecs::query::WorldQuery>::Item<'_>,
        _: &QualifierQuery<u32>,
        stat: &S,
        value: &mut S::Value,
        _: &impl bevy_stat_query::Querier<u32>,
    ) {
        if let Some(value) = stat.is_then_cast(&Damage, value) {
            let Some(weapon) = cx.get(component.weapon) else {
                return;
            };
            value.add(weapon.damage * component.state.durability)
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
            assets.add(Weapon { damage: 4.0 }),
            WeaponState { durability: 0.5 },
        ));
    });
    commands.spawn((StatEntity, B)).with_children(|x| {
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
    querier: StatQuery<u32>,
    weapon_query: Query<WeaponHandle>,
    cx: Res<Assets<Weapon>>,
    a: Query<Entity, (With<StatEntity>, With<A>)>,
    b: Query<Entity, (With<StatEntity>, With<B>)>,
) {
    let querier = querier.with_children_cx(&weapon_query, &cx);
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
