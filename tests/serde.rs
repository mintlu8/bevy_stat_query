use bevy_ecs::{component::Component, world::World};
use bevy_reflect::TypePath;
use bevy_serde_lens::{bind_object, DefaultInit, WorldExtension};
use bevy_stat_query::StatVTable;
use bevy_stat_query::{types::*, Fraction, Qualifier, Stat, StatExtension, StatMap, operations::StatOperation};
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
            type Value = $ty;

            fn name(&self) -> &'static str {
                stringify!($name)
            }

            fn values() -> impl IntoIterator<Item = Self> {
                [Self]
            }

            fn vtable() -> &'static StatVTable<$name> {
                static VTABLE: StatVTable<$name> = StatVTable::of::<$name>();
                &VTABLE
            }

            fn as_index(&self) -> u64 {
                0
            }

            fn from_index(_: u64) -> Self {
                Self
            }
        })*
    };
}

impl_stat!(
    SInt: StatInt<i32>,
    SUInt: StatInt<u32>,
    SFloat32: StatFloat<f32>,
    SFlags: StatFlags<MyFlags>,
    SString: StatOnce<Box<str>>,
    SIntPct: StatIntPercent<i32>,
    SIntFrac: StatIntFraction<i8>,
    SMul: StatMult<f32>,
    SFracMul: StatMult<Fraction<i32>>
);

#[derive(Debug, Component, Serialize, Deserialize, Default, TypePath)]
pub struct BaseMarker;

bind_object!(
    #[serde(transparent)]
    pub struct Base as BaseMarker{
        #[serde(skip)]
        marker: DefaultInit<BaseMarker>,
        map: StatMap<bool>,
    }
);

#[derive(Debug, Component, Serialize, Deserialize, Default, TypePath)]
pub struct OpMarker;

bind_object!(
    #[serde(transparent)]
    pub struct Op as OpMarker{
        #[serde(skip)]
        marker: DefaultInit<OpMarker>,
        map: StatMap<bool>,
    }
);

#[derive(Debug, Component, Serialize, Deserialize, Default, TypePath)]
pub struct FullMarker;

bind_object!(
    #[serde(transparent)]
    pub struct Full as FullMarker{
        #[serde(skip)]
        marker: DefaultInit<FullMarker>,
        map: StatMap<bool>,
    }
);

#[test]
pub fn serde_test() {
    let mut world = World::new();
    world.register_stat::<SInt>();
    world.register_stat::<SUInt>();
    world.register_stat::<SFloat32>();
    world.register_stat::<SFlags>();
    world.register_stat::<SString>();
    world.register_stat::<SIntPct>();
    world.register_stat::<SIntFrac>();
    world.register_stat::<SMul>();
    world.register_stat::<SFracMul>();

    let q_false = Qualifier::all_of(false);
    world.spawn((BaseMarker, {
        let mut map = StatMap::new();
        map.insert_base(q_false, SInt, -4);
        map.insert_base(q_false, SUInt, 7);
        map.insert_base(q_false, SFloat32, 3.5);
        map.insert_base(q_false, SFlags, MyFlags::F);
        map.insert_base(
            q_false,
            SString,
            "Ferris the Rustacean".into(),
        );
        map.insert_base(q_false, SIntFrac, 69);
        map.insert_base(q_false, SIntPct, 420);
        map.insert_base(q_false, SMul, 1.5);
        map.insert_base(q_false, SFracMul, Fraction::new(44, 57));
        map
    }));
    let value = world
        .save::<BaseMarker, _>(serde_json::value::Serializer)
        .unwrap();
    world.despawn_bound_objects::<BaseMarker>();
    world.load::<BaseMarker, _>(&value).unwrap();
    let value2 = world
        .save::<BaseMarker, _>(serde_json::value::Serializer)
        .unwrap();
    assert_eq!(value, value2);

    world.spawn((OpMarker, {
        let mut map = StatMap::new();
        use StatOperation::*;
        map.modify(q_false, SInt, Add(-4));
        map.modify(q_false, SUInt, Max(7));
        map.modify(q_false, SFloat32, Min(3.5));
        map.modify(q_false, SFlags, Or(MyFlags::F));
        map.modify(q_false, SString, Not("Ferris the Rustacean".into()));
        map.modify(q_false, SIntFrac, Mul(Fraction::new(43, -47)));
        map.modify(q_false, SIntPct, Mul(32));
        map.modify(q_false, SMul, Mul(102.125));
        map.modify(q_false, SFracMul, Mul(Fraction::new(0, 1)));
        map
    }));
    let value = world.save::<Op, _>(serde_json::value::Serializer).unwrap();
    world.despawn_bound_objects::<Op>();
    world.load::<Op, _>(&value).unwrap();
    let value2 = world.save::<Op, _>(serde_json::value::Serializer).unwrap();
    assert_eq!(value, value2);

    world.spawn((FullMarker, {
        let mut map = StatMap::new();
        map.insert(q_false, SInt, Default::default());
        map.insert(q_false, SUInt, Default::default());
        map.insert(q_false, SString, Default::default());
        map.insert(q_false, SIntFrac, Default::default());
        map.insert(q_false, SIntPct, Default::default());
        map.insert(q_false, SFracMul, Default::default());
        map.insert(q_false, SFloat32, Default::default());
        map.insert(q_false, SFlags, Default::default());
        map.insert(q_false, SMul, Default::default());
        map
    }));
    let value = world
        .save::<Full, _>(serde_json::value::Serializer)
        .unwrap();
    world.despawn_bound_objects::<Full>();
    world.load::<Full, _>(&value).unwrap();
    let value2 = world
        .save::<Full, _>(serde_json::value::Serializer)
        .unwrap();
    assert_eq!(value, value2);
    world.despawn_bound_objects::<Full>();

    world.spawn((FullMarker, {
        let mut map = StatMap::new();
        map.insert(q_false, SInt, Default::default());
        map.insert(q_false, SUInt, Default::default());
        map.insert(q_false, SString, Default::default());
        map.insert(q_false, SIntFrac, Default::default());
        map.insert(q_false, SIntPct, Default::default());
        map.insert(q_false, SFracMul, Default::default());
        map.insert(q_false, SFloat32, Default::default());
        map.insert(q_false, SFlags, Default::default());
        map.insert(q_false, SMul, Default::default());
        map
    }));
    use postcard::ser_flavors::Flavor;
    let mut vec = postcard::Serializer {
        output: postcard::ser_flavors::AllocVec::new(),
    };
    world.save::<Full, _>(&mut vec).unwrap();
    let result = vec.output.finalize().unwrap();
    world.despawn_bound_objects::<Full>();
    world
        .load::<Full, _>(&mut postcard::Deserializer::from_bytes(&result))
        .unwrap();

    let mut vec2 = postcard::Serializer {
        output: postcard::ser_flavors::AllocVec::new(),
    };
    world.save::<Full, _>(&mut vec2).unwrap();
    let result2 = vec2.output.finalize().unwrap();
    assert_eq!(result, result2);
}
