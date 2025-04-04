use bevy::{
    asset::AssetPlugin,
    prelude::{Single, With},
};
use bevy_app::App;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    system::{Commands, RunSystemOnce},
};
use bevy_stat_query::{
    match_stat, types::StatFloat, ChildQuery, Qualifier, QualifierFlag, QualifierQuery, Querier,
    Stat, StatEntities, StatEntity, StatExtension, StatMap, StatQuery, StatStream, StatValue,
    StatValuePair,
};

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
    pub struct Adjective: u32 {
        const Fire = 1;
        const Water = 2;
        const Earth = 4;
        const Air = 8;
    }
}

#[derive(Debug, Clone, Copy, Stat)]
#[stat(value = "StatFloat<f32>")]
pub enum Stats {
    WeaponDamage,
    Damage,
    Defense,
    Strength,
    WeaponProficiency,
}

#[derive(Component)]
pub struct Main;

#[derive(Component)]
pub struct Weapon {
    pub damage: f32,
}

impl StatStream for Weapon {
    type Qualifier = Adjective;

    fn stream_stat(
        &self,
        _: Entity,
        _: &QualifierQuery<Self::Qualifier>,
        stat_value: &mut StatValuePair,
        _: Querier<Self::Qualifier>,
    ) {
        if let Some(value) = stat_value.is_then_cast(&Stats::WeaponDamage) {
            value.add(self.damage);
        }
    }
}

#[derive(Component)]
pub struct StrengthBuff {
    qualifier: Qualifier<Adjective>,
    multiplier: f32,
}

impl StatStream for StrengthBuff {
    type Qualifier = Adjective;

    fn stream_stat(
        &self,
        _: Entity,
        qualifier: &QualifierQuery<Self::Qualifier>,
        stat_value: &mut StatValuePair,
        _: Querier<Self::Qualifier>,
    ) {
        if let Some(value) = stat_value.is_then_cast(&Stats::Strength) {
            if qualifier.qualify(&self.qualifier) {
                value.mul(self.multiplier);
            }
        }
    }
}

#[derive(Component)]
pub struct DamageBuff {
    qualifier: Qualifier<Adjective>,
    multiplier: f32,
}

impl StatStream for DamageBuff {
    type Qualifier = Adjective;

    fn stream_stat(
        &self,
        _: Entity,
        qualifier: &QualifierQuery<Self::Qualifier>,
        stat_value: &mut StatValuePair,
        _: Querier<Self::Qualifier>,
    ) {
        if let Some(value) = stat_value.is_then_cast(&Stats::Damage) {
            if qualifier.qualify(&self.qualifier) {
                value.mul(self.multiplier);
            }
        }
    }
}

#[test]
pub fn test() {
    let mut app = App::new();
    app.add_plugins(AssetPlugin::default())
        .register_stat_relation::<Adjective>(|entity, qualifier, stat, querier| {
            match_stat!(stat => {
                (Stats::Damage, value) => {
                    value.add(
                        querier.eval_stat(entity, qualifier, &Stats::Strength).unwrap()
                    );
                    value.add(
                        querier.eval_stat(entity, qualifier, &Stats::WeaponDamage).unwrap() *
                        querier.eval_stat(entity, qualifier, &Stats::WeaponProficiency).unwrap()
                    );
                }
            })
        });
    app.world_mut().run_system_once(init).unwrap();
    app.world_mut().flush();
    app.world_mut().run_system_once(query).unwrap();
}

fn init(mut commands: Commands) {
    commands
        .spawn((
            StatEntity,
            {
                let mut map = StatMap::<Adjective>::new();
                map.insert_base(Default::default(), Stats::Strength, 4.0);
                map.insert_base(Default::default(), Stats::WeaponProficiency, 0.5);
                map
            },
            Weapon { damage: 12.0 },
            Main,
        ))
        .with_children(|c| {
            c.spawn(StrengthBuff {
                qualifier: Adjective::none().into(),
                multiplier: 1.5,
            });
            c.spawn(StrengthBuff {
                qualifier: Adjective::Fire.into(),
                multiplier: 2.0,
            });
            c.spawn(StrengthBuff {
                qualifier: Adjective::Water.into(),
                multiplier: 0.5,
            });
            // elemental damage
            c.spawn(DamageBuff {
                qualifier: Qualifier {
                    all_of: Adjective::none(),
                    any_of: Adjective::all(),
                },
                multiplier: 2.0,
            });
            c.spawn(DamageBuff {
                qualifier: Adjective::Fire.into(),
                multiplier: 1.5,
            });
        });
}

fn query(
    entities: Single<Entity, With<Main>>,
    querier: StatEntities<Adjective>,
    base_stat_query: StatQuery<StatMap<Adjective>>,
    weapon_query: StatQuery<Weapon>,
    strength_buffs: ChildQuery<StrengthBuff>,
    damage_buffs: ChildQuery<DamageBuff>,
) {
    let entity = *entities;
    let querier = querier
        .join(&base_stat_query)
        .join(&weapon_query)
        .join(&strength_buffs)
        .join(&damage_buffs);

    // 4 * 1.5 + 12 * 0.5
    assert_eq!(
        querier.eval_stat(entity, &Default::default(), &Stats::Damage),
        Some(12.0)
    );
    // (4 * 1.5 * 2 + 12 * 0.5) * 2 * 1.5
    assert_eq!(
        querier.eval_stat(entity, &Adjective::Fire.into(), &Stats::Damage),
        Some(54.0)
    );
    // (4 * 1.5 * 0.5 + 12 * 0.5) * 2
    assert_eq!(
        querier.eval_stat(entity, &Adjective::Water.into(), &Stats::Damage),
        Some(18.0)
    );
    // (4 * 1.5 * 2 * 0.5 + 12 * 0.5) * 2 * 1.5
    assert_eq!(
        querier.eval_stat(
            entity,
            &(Adjective::Fire | Adjective::Water).into(),
            &Stats::Damage
        ),
        Some(36.0)
    );
}
