use bevy::{
    asset::{Asset, AssetApp, AssetPlugin, Assets, Handle},
    prelude::{ResMut, Single},
};
use bevy_app::App;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{QueryData, With},
    system::{Commands, Res, RunSystemOnce},
};
use bevy_hierarchy::{BuildChildren, ChildBuild};
use bevy_reflect::TypePath;
use bevy_stat_query::{
    types::StatFloat, ChildQuery, QualifierQuery, Querier, QueryStream, Stat, StatEntities,
    StatEntity, StatValue, StatValuePair,
};

#[derive(Debug, Clone, Copy, Stat)]
#[stat(value = "StatFloat<f32>")]
pub enum Stats {
    Damage,
    Defense,
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
        query: WeaponQueryItem,
        context: &Res<Assets<Weapon>>,
        _: Entity,
        _: &QualifierQuery<Self::Qualifier>,
        stat_value: &mut StatValuePair,
        _: Querier<Self::Qualifier>,
    ) {
        if let Some(value) = stat_value.is_then_cast(&Stats::Damage) {
            let Some(weapon) = context.get(query.weapon.0.id()) else {
                return;
            };
            value.add(weapon.damage * query.state.durability)
        }
    }
}

#[test]
pub fn asset_test() {
    let mut app = App::new();
    app.add_plugins(AssetPlugin::default())
        .init_asset::<Weapon>();
    app.world_mut().run_system_once(init).unwrap();
    app.world_mut().flush();
    app.world_mut().run_system_once(query).unwrap();
}

fn init(mut commands: Commands, mut assets: ResMut<Assets<Weapon>>) {
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
    weapon_query: ChildQuery<WeaponQuery>,
    a: Single<Entity, (With<StatEntity>, With<A>)>,
    b: Single<Entity, (With<StatEntity>, With<B>)>,
) {
    let querier = querier.join(&weapon_query);
    assert_eq!(
        querier.eval_stat(*a, &QualifierQuery::Aggregate(0u32), &Stats::Damage),
        Some(2.0)
    );
    assert_eq!(
        querier.eval_stat(*a, &QualifierQuery::Aggregate(0u32), &Stats::Defense),
        Some(0.0)
    );

    assert_eq!(
        querier.eval_stat(*b, &QualifierQuery::Aggregate(0u32), &Stats::Damage),
        Some(20.0)
    );
    assert_eq!(
        querier.eval_stat(*b, &QualifierQuery::Aggregate(0u32), &Stats::Defense),
        Some(0.0)
    );
}
