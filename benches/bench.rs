use std::collections::BTreeMap;

use bevy_stat_query::{
    QualifierItem, QualifierQuery, Stat, StatMapBase, StatValue, types::StatIntPercentAdditive,
};
use criterion::{Criterion, criterion_group, criterion_main};

#[derive(Debug, Clone, Copy, Stat)]
#[stat(value = "StatIntPercentAdditive<i32>")]
pub struct S;

pub fn query_many(c: &mut Criterion) {
    let mut m = StatMapBase::<QualifierItem<u32>, S>::new();
    let mut bt_dyn = BTreeMap::new();

    for i in 0..1024 {
        m.insert_base(QualifierItem::all_of(i), S, 1);
        bt_dyn.insert(
            QualifierItem::all_of(i),
            StatIntPercentAdditive::default().with_add(1),
        );
    }

    c.bench_function("btree_aggregate_many", |b| {
        b.iter(|| {
            let mut result = StatIntPercentAdditive::<i32>::default();
            bt_dyn
                .iter()
                .filter(|(q, _)| q.qualifies_as(&QualifierQuery::Aggregate(255)))
                .for_each(|(_, v)| result.join(v));
            result
        })
    });

    c.bench_function("stat_map_aggregate_many", |b| {
        b.iter(|| m.eval_stat(&QualifierQuery::Aggregate(255), &S))
    });
}

criterion_group!(benches, query_many);
criterion_main!(benches);
