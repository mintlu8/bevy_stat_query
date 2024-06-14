use bevy_stat_query::{
    types::StatIntPercentAdditive, BaseStatMap, Qualifier, QualifierFlag, QualifierQuery, Stat,
    StatOperation, StatOperationsMap, StatValue, StatValuePair, StatelessStream,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct S;

impl Stat for S {
    type Data = StatIntPercentAdditive<i32>;

    fn name(&self) -> &str {
        "s"
    }

    fn values() -> impl IntoIterator<Item = Self> {
        [S]
    }
}

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

    let mut map = BaseStatMap::<Q>::new();
    map.insert(none, S, 1);
    map.insert(fire, S, 2);
    map.insert(fire_magic, S, 4);

    let mut data = StatIntPercentAdditive::<i32>::default();
    map.stat_extend(
        &QualifierQuery::none(),
        &mut StatValuePair::new(&S, &mut data),
    );
    assert_eq!(data.eval(), 1);

    let mut data = StatIntPercentAdditive::<i32>::default();
    map.stat_extend(
        &QualifierQuery::Aggregate(Q::Fire),
        &mut StatValuePair::new(&S, &mut data),
    );
    assert_eq!(data.eval(), 3);

    let mut data = StatIntPercentAdditive::<i32>::default();
    map.stat_extend(
        &QualifierQuery::Aggregate(Q::Fire | Q::Magic),
        &mut StatValuePair::new(&S, &mut data),
    );
    assert_eq!(data.eval(), 7);

    let mut map = StatOperationsMap::<Q>::new();
    map.insert(none, S, StatOperation::Add(2));
    // + 100%
    map.insert(fire, S, StatOperation::Mul(100));
    map.insert(fire_magic, S, StatOperation::Max(2));

    let mut data = StatIntPercentAdditive::<i32>::default();
    map.stat_extend(
        &QualifierQuery::none(),
        &mut StatValuePair::new(&S, &mut data),
    );
    assert_eq!(data.eval(), 2);

    let mut data = StatIntPercentAdditive::<i32>::default();
    map.stat_extend(
        &QualifierQuery::Aggregate(Q::Fire),
        &mut StatValuePair::new(&S, &mut data),
    );
    assert_eq!(data.eval(), 4);

    let mut data = StatIntPercentAdditive::<i32>::default();
    map.stat_extend(
        &QualifierQuery::Aggregate(Q::Fire | Q::Magic),
        &mut StatValuePair::new(&S, &mut data),
    );
    assert_eq!(data.eval(), 2);
}
