use bevy_stat_query::{
    operations::StatOperation::{Add, Max, Mul},
    types::StatIntPercentAdditive,
    Qualifier, QualifierFlag, QualifierQuery, Stat, StatMap,
};

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
    struct Q: u32 {
        const Fire = 1;
        const Water = 2;
        const Earth = 4;
        const Air = 8;
        const Magic = 16;
        const Slash = 32;
        const Blast = 64;
    }
}

#[derive(Debug, Clone, Copy, Stat)]
#[stat(value = "StatIntPercentAdditive<i32>")]
pub struct S;

#[test]
pub fn qualifier_test() {
    let none = Qualifier::<Q>::none();

    assert!(none.qualifies_as(&QualifierQuery::none()));
    assert!(none.qualifies_as(&QualifierQuery::Aggregate(Q::Fire)));
    assert!(none.qualifies_as(&QualifierQuery::Aggregate(Q::Fire | Q::Water)));
    assert!(none.qualifies_as(&QualifierQuery::Aggregate(Q::Fire | Q::Magic)));
    assert!(none.qualifies_as(&QualifierQuery::Aggregate(Q::Water | Q::Magic)));
    assert!(none.qualifies_as(&QualifierQuery::Aggregate(Q::Water | Q::Magic)));

    let fire = Qualifier::all_of(Q::Fire);
    assert!(!fire.qualifies_as(&QualifierQuery::none()));
    assert!(fire.qualifies_as(&QualifierQuery::Aggregate(Q::Fire)));
    assert!(fire.qualifies_as(&QualifierQuery::Aggregate(Q::Fire | Q::Water)));
    assert!(fire.qualifies_as(&QualifierQuery::Aggregate(Q::Fire | Q::Magic)));
    assert!(!fire.qualifies_as(&QualifierQuery::Aggregate(Q::Water | Q::Magic)));

    let fire_magic = Qualifier::all_of(Q::Fire | Q::Magic);

    assert!(!fire_magic.qualifies_as(&QualifierQuery::none()));
    assert!(!fire_magic.qualifies_as(&QualifierQuery::Aggregate(Q::Fire)));
    assert!(!fire_magic.qualifies_as(&QualifierQuery::Aggregate(Q::Fire | Q::Water)));
    assert!(fire_magic.qualifies_as(&QualifierQuery::Aggregate(Q::Fire | Q::Magic)));
    assert!(fire_magic.qualifies_as(&QualifierQuery::Aggregate(Q::Fire | Q::Water | Q::Magic)));
    assert!(!fire_magic.qualifies_as(&QualifierQuery::Aggregate(Q::Water | Q::Magic)));

    let elemental = Qualifier::any_of(Q::Fire | Q::Water | Q::Earth | Q::Air);

    assert!(!elemental.qualifies_as(&QualifierQuery::none()));
    assert!(elemental.qualifies_as(&QualifierQuery::Aggregate(Q::Fire)));
    assert!(elemental.qualifies_as(&QualifierQuery::Aggregate(Q::Water)));
    assert!(elemental.qualifies_as(&QualifierQuery::Aggregate(Q::Earth)));
    assert!(elemental.qualifies_as(&QualifierQuery::Aggregate(Q::Air)));
    assert!(elemental.qualifies_as(&QualifierQuery::Aggregate(Q::Fire | Q::Water)));
    assert!(elemental.qualifies_as(&QualifierQuery::Aggregate(Q::Earth | Q::Air)));
    assert!(elemental.qualifies_as(&QualifierQuery::Aggregate(
        Q::Fire | Q::Water | Q::Earth | Q::Air
    )));
    assert!(!elemental.qualifies_as(&QualifierQuery::Aggregate(Q::Magic)));
    assert!(!elemental.qualifies_as(&QualifierQuery::Aggregate(Q::Magic | Q::Blast)));
    assert!(elemental.qualifies_as(&QualifierQuery::Aggregate(Q::Fire | Q::Magic)));
    assert!(elemental.qualifies_as(&QualifierQuery::Aggregate(Q::Fire | Q::Air | Q::Magic)));

    let elemental_magic = elemental.and_all_of(Q::Magic);

    assert!(!elemental_magic.qualifies_as(&QualifierQuery::none()));
    assert!(!elemental_magic.qualifies_as(&QualifierQuery::Aggregate(Q::Fire)));
    assert!(!elemental_magic.qualifies_as(&QualifierQuery::Aggregate(Q::Water)));
    assert!(!elemental_magic.qualifies_as(&QualifierQuery::Aggregate(Q::Earth)));
    assert!(!elemental_magic.qualifies_as(&QualifierQuery::Aggregate(Q::Air)));
    assert!(elemental_magic.qualifies_as(&QualifierQuery::Aggregate(Q::Fire | Q::Magic)));
    assert!(elemental_magic.qualifies_as(&QualifierQuery::Aggregate(Q::Water | Q::Magic)));
    assert!(elemental_magic.qualifies_as(&QualifierQuery::Aggregate(Q::Earth | Q::Magic)));
    assert!(elemental_magic.qualifies_as(&QualifierQuery::Aggregate(Q::Air | Q::Magic)));
    assert!(!elemental_magic.qualifies_as(&QualifierQuery::Aggregate(Q::Fire | Q::Water)));
    assert!(!elemental_magic.qualifies_as(&QualifierQuery::Aggregate(Q::Earth | Q::Air)));
    assert!(!elemental_magic.qualifies_as(&QualifierQuery::Aggregate(
        Q::Fire | Q::Water | Q::Earth | Q::Air
    )));
    assert!(!elemental_magic.qualifies_as(&QualifierQuery::Aggregate(Q::Magic)));
    assert!(!elemental.qualifies_as(&QualifierQuery::Aggregate(Q::Magic | Q::Blast)));
    assert!(elemental_magic.qualifies_as(&QualifierQuery::Aggregate(Q::Fire | Q::Magic)));
    assert!(elemental_magic.qualifies_as(&QualifierQuery::Aggregate(Q::Fire | Q::Air | Q::Magic)));

    assert!(!none.qualifies_as(&QualifierQuery::Exact {
        any_of: Q::none(),
        all_of: Q::Fire,
    }));

    assert!(!elemental.qualifies_as(&QualifierQuery::Exact {
        any_of: Q::none(),
        all_of: Q::Fire,
    }));

    assert!(fire.qualifies_as(&QualifierQuery::Exact {
        any_of: Q::none(),
        all_of: Q::Fire,
    }));

    let query_elemental = QualifierQuery::Exact {
        any_of: Q::Fire | Q::Water | Q::Earth | Q::Air,
        all_of: Q::none(),
    };
    let all_elements = Qualifier::all_of(Q::Fire | Q::Water | Q::Earth | Q::Air);

    assert!(elemental.qualifies_as(&query_elemental));
    assert!(!none.qualifies_as(&query_elemental));

    assert!(!all_elements.qualifies_as(&query_elemental));
    assert!(!fire.qualifies_as(&query_elemental));
    assert!(!fire_magic.qualifies_as(&query_elemental));

    let mut map = StatMap::<Q>::new();
    map.insert_base(none, S, 1);
    map.insert_base(fire, S, 2);
    map.insert_base(fire_magic, S, 4);
    assert_eq!(map.eval_stat(&QualifierQuery::none(), &S), 1);
    assert_eq!(map.eval_stat(&QualifierQuery::Aggregate(Q::Fire), &S), 3);
    assert_eq!(
        map.eval_stat(&QualifierQuery::Aggregate(Q::Fire | Q::Magic), &S),
        7
    );

    let mut map = StatMap::<Q>::new();
    map.modify(none, S, Add(2));
    // + 100%
    map.modify(fire, S, Mul(100));
    map.modify(fire_magic, S, Max(2));
    assert_eq!(map.eval_stat(&QualifierQuery::none(), &S), 2);
    assert_eq!(map.eval_stat(&QualifierQuery::Aggregate(Q::Fire), &S), 4);
    assert_eq!(
        map.eval_stat(&QualifierQuery::Aggregate(Q::Fire | Q::Magic), &S),
        2
    );
}
