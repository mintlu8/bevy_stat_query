use bevy_stat_query::types::StatIntRounded;
use bevy_stat_query::Attribute;
use bevy_stat_query::Stat;

#[derive(Debug, Clone, Copy, Stat, PartialEq, Eq)]
#[stat(value = "StatIntRounded<i32, f32>")]
pub enum Stats {
    A,
    B,
    C,
    D,
}

#[derive(Debug, Clone, Copy, Stat, PartialEq, Eq)]
#[stat(value = "StatIntRounded<i32, f32>")]
pub enum NumStats {
    E = 2,
    F = 0,
    G,
    H = 3,
}

#[derive(Debug, Clone, Copy, Stat, PartialEq, Eq)]
#[stat(value = "StatIntRounded<i32, f32>")]
pub struct X;

use NumStats::*;
use Stats::*;

#[derive(Debug, Attribute)]
pub struct IsDragon;

#[derive(Debug, Attribute)]
#[repr(u64)]
pub enum CreatureType {
    Beast,
    Dragon,
}
#[derive(Attribute)]
pub struct CreatureAbility(u64);

bitflags::bitflags! {
    impl CreatureAbility: u64 {
        const Fire = 1;
        const Ice = 2;
    }
}

#[test]
pub fn test_derive() {
    assert_eq!(Stats::from_index(Stat::as_index(&A)), A);
    assert_eq!(Stats::from_index(Stat::as_index(&B)), B);
    assert_eq!(Stats::from_index(Stat::as_index(&C)), C);
    assert_eq!(Stats::from_index(Stat::as_index(&D)), D);
    assert_eq!(NumStats::from_index(Stat::as_index(&E)), E);
    assert_eq!(NumStats::from_index(Stat::as_index(&F)), F);
    assert_eq!(NumStats::from_index(Stat::as_index(&G)), G);
    assert_eq!(NumStats::from_index(Stat::as_index(&H)), H);
    assert_eq!(Stats::values().into_iter().count(), 4);
    assert_eq!(NumStats::values().into_iter().count(), 4);
    assert_eq!(A.name(), "A");
    assert_eq!(B.name(), "B");
    assert_eq!(C.name(), "C");
    assert_eq!(D.name(), "D");
    assert_eq!(E.name(), "E");
    assert_eq!(F.name(), "F");
    assert_eq!(G.name(), "G");
    assert_eq!(H.name(), "H");
    assert_eq!(X::from_index(Stat::as_index(&X)), X);
    assert_eq!(X::values().into_iter().count(), 1);
    assert_eq!(X.name(), "X");
}
