use std::collections::BTreeMap;

use bevy_stat_query::{
    operations::StatOperation::Add, types::StatIntPercentAdditive, Qualifier, QualifierQuery, Stat,
    StatMap, StatStreamExt,
};
use criterion::{criterion_group, criterion_main, Criterion};

#[derive(Debug, Clone, Copy, Stat)]
#[stat(value = "StatIntPercentAdditive<i32>")]
pub struct S;

pub fn query_many(c: &mut Criterion) {
    let mut m = StatMap::<u32>::new();
    let mut bt = BTreeMap::new();

    for i in 0..1024 {
        m.insert_op(Qualifier::all_of(i), S, Add(1));
        bt.insert(Qualifier::all_of(i), 1);
    }

    c.bench_function("btree_aggregate_many", |b| {
        b.iter(|| {
            bt.iter()
                .filter(|(q, _)| q.qualifies_as(&QualifierQuery::Aggregate(255)))
                .map(|(_, v)| v)
                .sum::<i32>()
        })
    });

    c.bench_function("aggregate_many", |b| {
        b.iter(|| m.eval_stat(&QualifierQuery::Aggregate(255), &S))
    });
}

criterion_group!(benches, query_many);
criterion_main!(benches);
