use std::{any::Any, collections::BTreeMap};

use bevy_stat_query::{
    operations::StatOperation::Add, types::StatIntPercentAdditive, Qualifier, QualifierQuery, Stat,
    StatMap, StatValue,
};
use criterion::{criterion_group, criterion_main, Criterion};

#[derive(Debug, Clone, Copy, Stat)]
#[stat(value = "StatIntPercentAdditive<i32>")]
pub struct S;

pub fn query_many(c: &mut Criterion) {
    let mut m = StatMap::<u32>::new();
    let mut bt = BTreeMap::new();
    let mut bt_dyn = BTreeMap::new();

    for i in 0..1024 {
        m.insert_op(Qualifier::all_of(i), S, Add(1));
        bt.insert(Qualifier::all_of(i), 1);
        bt_dyn.insert(Qualifier::all_of(i), Box::new(1) as Box<dyn Any>);
    }

    c.bench_function("btree_aggregate_many", |b| {
        b.iter(|| {
            let mut result = StatIntPercentAdditive::<i32>::default();
            bt.iter()
                .filter(|(q, _)| q.qualifies_as(&QualifierQuery::Aggregate(255)))
                .for_each(|(_, v)| result.join(Add(*v).into_stat()));
            result
        })
    });

    c.bench_function("btree_dyn_aggregate_many", |b| {
        b.iter(|| {
            let mut result = StatIntPercentAdditive::<i32>::default();
            bt_dyn
                .iter()
                .filter(|(q, _)| q.qualifies_as(&QualifierQuery::Aggregate(255)))
                .map(|(_, v)| v.downcast_ref::<i32>().copied().unwrap())
                .for_each(|v| result.join(Add(v).into_stat()));
            result
        })
    });

    c.bench_function("stat_map_aggregate_many", |b| {
        b.iter(|| m.eval_stat(&QualifierQuery::Aggregate(255), &S))
    });
}

criterion_group!(benches, query_many);
criterion_main!(benches);
