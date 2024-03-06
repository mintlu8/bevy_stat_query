use bevy_ecs::{component::Component, world::World};
use bevy_reflect::TypePath;
use bevy_serde_project::{bind_object, WorldExtension};
use bevy_stat_query::{types::*, BaseStatMap, Fraction, FullStatMap, Qualifier, Stat, StatExtension, StatOperation, StatOperationsMap};
use bevy_utils::hashbrown::HashSet;
use serde::{Deserialize, Serialize};


bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize, TypePath)]
    pub struct MyFlags: u32 {
        const A = 1;
        const B = 2;
        const C = 4;
        const D = 8;
        const E = 16;
        const F = 32;
        const G = 64;
    }
}

macro_rules! impl_stat {
    ($($name: ident: $ty: ty),* $(,)?) => {
        $(#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name;

        impl Stat for $name {
            type Data = $ty;

            fn name(&self) -> &str {
                stringify!($name)
            }

            fn values() -> impl IntoIterator<Item = Self> {
                [Self]
            }
        })*
    };
}

impl_stat!(
    SInt: StatInt<i32>,
    SUInt: StatInt<u32>,
    SFloat32: StatFloat<f32>,
    SFloat64: StatFloat<f64>,
    SFlags: StatFlags<MyFlags>,
    SString: StatOnce<String>,
    SSet: StatSet<String>,
    SIntPct: StatIntPercent<i64>,
    SIntFrac: StatIntFraction<i8>,
    SMul: StatMult<f64>,
    SFracMul: StatMult<Fraction<isize>>
);

#[derive(Debug, Component, Serialize, Deserialize, Default)]
pub struct BaseMarker;

bind_object!(
    #[serde(transparent)]
    BaseMarker as "BaseStatMap"{
        #[serde(skip)]
        marker => BaseMarker,
        map => BaseStatMap<bool>,
    }
);


#[derive(Debug, Component, Serialize, Deserialize, Default)]
pub struct OpMarker;

bind_object!(
    #[serde(transparent)]
    OpMarker as "StatOperationsMap"{
        #[serde(skip)]
        marker => OpMarker,
        map => StatOperationsMap<bool>,
    }
);


#[derive(Debug, Component, Serialize, Deserialize, Default)]
pub struct FullMarker;

bind_object!(
    #[serde(transparent)]
    FullMarker as "StatOperationsMap"{
        #[serde(skip)]
        marker => FullMarker,
        map => FullStatMap<bool>,
    }
);

#[test]
pub fn operations() {
    let mut world = World::new();
    world.register_stat::<SInt>();
    world.register_stat::<SUInt>();
    world.register_stat::<SFloat32>();
    world.register_stat::<SFloat64>();
    world.register_stat::<SFlags>();
    world.register_stat::<SString>();
    world.register_stat::<SSet>();
    world.register_stat::<SIntPct>();
    world.register_stat::<SIntFrac>();
    world.register_stat::<SMul>();
    world.register_stat::<SFracMul>();

    let q_false = Qualifier::all_of(false);
    world.spawn((BaseMarker, {
        let mut map = BaseStatMap::new();
        map.insert(q_false, SInt, -4);
        map.insert(q_false, SUInt, 7);
        map.insert(q_false, SFloat32, 3.5);
        map.insert(q_false, SFloat64, -6.25);
        map.insert(q_false, SFlags, MyFlags::F);
        map.insert(q_false, SString, StatOnce::Found("Ferris the Rustacean".to_owned()));
        // TODO: this is actually non-deterministic, maybe fix later?
        map.insert(q_false, SSet, HashSet::from(["foo".to_owned()]));
        map.insert(q_false, SIntFrac, 69);
        map.insert(q_false, SIntPct, 420);
        map.insert(q_false, SMul, 1.5);
        map.insert(q_false, SFracMul, Fraction::new(44, 57));
        map
    }));
    let value = world.save::<BaseMarker, _>(serde_json::value::Serializer).unwrap();
    world.despawn_bound_objects::<BaseMarker>();
    world.load::<BaseMarker, _>(&value).unwrap();
    let value2 = world.save::<BaseMarker, _>(serde_json::value::Serializer).unwrap();
    assert_eq!(value, value2);

    world.spawn((OpMarker, {
        let mut map = StatOperationsMap::new();
        use StatOperation::*;
        map.insert(q_false, SInt, Add(-4));
        map.insert(q_false, SUInt, Max(7));
        map.insert(q_false, SFloat32, Min(3.5));
        map.insert(q_false, SFloat64, Max(-6.25));
        map.insert(q_false, SFlags, Or(MyFlags::F));
        map.insert(q_false, SString, Not("Ferris the Rustacean".to_owned()));
        map.insert(q_false, SSet, Or(HashSet::from(["foo".to_owned()])));
        map.insert(q_false, SIntFrac, Mul(Fraction::new(43, -47)));
        map.insert(q_false, SIntPct, Mul(32));
        map.insert(q_false, SMul, Mul(102.125));
        map.insert(q_false, SFracMul, Mul(Fraction::new(0, 1)));
        map
    }));
    let value = world.save::<OpMarker, _>(serde_json::value::Serializer).unwrap();
    world.despawn_bound_objects::<OpMarker>();
    world.load::<OpMarker, _>(&value).unwrap();
    let value2 = world.save::<OpMarker, _>(serde_json::value::Serializer).unwrap();
    assert_eq!(value, value2);


    world.spawn((FullMarker, {
        let mut map = FullStatMap::new();
        map.insert(q_false, SInt, Default::default());
        map.insert(q_false, SUInt, Default::default());
        map.insert(q_false, SString, Default::default());
        map.insert(q_false, SSet, Default::default());
        map.insert(q_false, SIntFrac, Default::default());
        map.insert(q_false, SIntPct, Default::default());
        map.insert(q_false, SFracMul, Default::default());
        map.insert(q_false, SFloat32, Default::default());
        map.insert(q_false, SFloat64, Default::default());
        map.insert(q_false, SFlags, Default::default());
        map.insert(q_false, SMul, Default::default());
        map
    }));
    let value = world.save::<FullMarker, _>(serde_json::value::Serializer).unwrap();
    world.despawn_bound_objects::<FullMarker>();
    world.load::<FullMarker, _>(&value).unwrap();
    let value2 = world.save::<FullMarker, _>(serde_json::value::Serializer).unwrap();
    assert_eq!(value, value2);
}