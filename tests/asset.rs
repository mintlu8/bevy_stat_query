use bevy_app::{App, Startup, Update};
use bevy_asset::{Asset, AssetApp, AssetPlugin, AssetServer, Assets, Handle};
use bevy_ecs::{component::Component, entity::Entity, query::With, system::{Commands, Query, Res}};
use bevy_hierarchy::BuildChildren;
use bevy_reflect::TypePath;
use bevy_stat_engine::{querier, stats, types::StatFloat, QualifierQuery, StatCache, StatValue, StatEnginePlugin, StatEntity, StatStream};

stats!(
    MyStatsPlugin {
        StatFloat<f32> {
            Damage,
            Defense,
        },
        StatFloat<f64> {
            X,
            Y {
                Hello,
                Hi,
            }
        },
    }
);

#[derive(Asset, TypePath)]
pub struct Weapon {
    pub damage: f32,
}

#[derive(Component)]
pub struct WeaponState {
    pub durability: f32
}


#[derive(Component)]
pub struct A;

#[derive(Component)]
pub struct B;

type MyQualifier = u32;

impl StatStream<MyQualifier> for Weapon {
    type Ctx = Res<'static, Assets<Weapon>>;
    type QueryData = (&'static Handle<Weapon>, &'static WeaponState);
    fn stream (
        assets: &<Self::Ctx as bevy_ecs::system::SystemParam>::Item<'_, '_>,
        (handle, state): <Self::QueryData as bevy_ecs::query::WorldQuery>::Item<'_>,
        _: &QualifierQuery<MyQualifier>,
        stat: &mut bevy_stat_engine::StatValuePair,
        _: &mut impl bevy_stat_engine::StatQuerier<MyQualifier>
    ) {
        stat.is_then(&Damage, |w| {
            let Some(weapon) = assets.get(handle) else {return};
            w.add(weapon.damage * state.durability)
        });
    }
}

querier!(pub MyQuerier {
    qualifier: MyQualifier,
    intrinsic: (),
    components: {
        Weapon
    }
});

#[test]
pub fn main() {
    App::new()
        .add_plugins(AssetPlugin::default())
        .add_plugins(StatEnginePlugin)
        .init_asset::<Weapon>()
        .add_systems(Startup, init)
        .add_systems(Update, query)
        .update();
}

fn init(mut commands: Commands, assets: Res<AssetServer>){
    commands.spawn((
        StatEntity,
        StatCache::<MyQualifier>::default(),
        A,
    )).with_children(|x| {
        x.spawn((
            assets.add(Weapon {
                damage: 4.0,
            }),
            WeaponState {
                durability: 0.5,
            },
        ));
    });
    commands.spawn((
        StatEntity,
        StatCache::<MyQualifier>::default(),
        B,
    )).with_children(|x| {
        x.spawn((
            assets.add(Weapon {
                damage: 8.0,
            }),
            WeaponState {
                durability: 1.0,
            },
        ));
        x.spawn((
            assets.add(Weapon {
                damage: 6.0,
            }),
            WeaponState {
                durability: 2.0,
            },
        ));
    });
}

fn query(
    mut querier: MyQuerier,
    a: Query<Entity, (With<StatEntity>, With<A>)>,
    b: Query<Entity, (With<StatEntity>, With<B>)>,
){
    let a = a.single();
    assert_eq!(querier.query_eval(a, &QualifierQuery::Aggregate(0u32), &Damage), Some(2.0));
    assert_eq!(querier.query_eval(a, &QualifierQuery::Aggregate(0u32), &Defense), Some(0.0));

    let b = b.single();
    assert_eq!(querier.query_eval(b, &QualifierQuery::Aggregate(0u32), &Damage), Some(20.0));
    assert_eq!(querier.query_eval(b, &QualifierQuery::Aggregate(0u32), &Defense), Some(0.0));
}
