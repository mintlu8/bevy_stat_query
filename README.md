# bevy-stat-query

Blazing fast and versatile RPG stat system for the bevy engine.

## Qualified Stats

We describe each stat as a `Qualifier` and a `Stat`.
`Stat` is a concrete stat noun like "Strength", "Magic", etc.
`Qualifier` is a flags based adjective that describes
what this `Stat` can be applied to.

For example in "FireMagicDamage", "Fire|Magic" is the qualifier,
"Damage" is the `Stat`.

What this means if an effect boosts "Fire|Damage", "Magic|Damage",
or simply just "Damage", the effect will be applied to the stat,
but an effect on "Sword|Damage" or "Fire|Range" won't be affecting the stat.

### Qualifier

`Qualifier` additionally provides `any_of` for modelling conditional effects like
"Elemental|Damage", which matches "Fire or Water or Wind Damage"
instead of "Fire and Water and Wind Damage".
Each `Qualifier` can only have one group of `any_of`.

#### Examples

```rust
let fire = Qualifier::all_of(Flag::Fire);
let fire_magic = Qualifier::all_of(Flag::Fire|Flag::Magic);
let elemental = Qualifier::any_of(Fire|Water|Air|Earth);
let elemental_magic = Qualifier::any_of(Fire|Water|Air|Earth)
    .and_all_of(Magic);
```

### QualifierQuery

`QualifierQuery` matches all `Qualifiers` on our entity that
qualifies as the query we are looking for.

`QualifierQuery::Aggregate` collects all qualifiers that matches the query.

For example, suppose we are looking for `(Frost|Piercing|Magic, Damage)`:

* `((), Damage)` qualifies.
* `(Frost, Damage)` qualifies.
* `(Frost|Magic, Damage)` qualifies.
* `(Frost|Piercing|Magic, Damage)` qualifies.
* `(Elemental, Damage)` qualifies.
* `(Frost|Sword, Damage)` does not qualify.
* `(Fire|Piercing|Magic, Defense)` does not qualify.

`QualifierQuery::Exact` allows us to deny
more generalized qualifiers.

For example, in order to model a statement like so:

```text
Add 50% of the character's magic damage to physical damage.
```

Querying `(Magic, Damage)`, which contains `((), Damage)`,
and adding to `(Physical, Damage)` would cause a duplication.

Therefore the query should be:

```rust
QualifierQuery::Exact {
    any_of: None,
    all_of: Magic,
}
```

### Stat

An app usually has a single `QualifierFlag` but multiple `Stat` implementors. This is because
each `Stat` can associate to a different type. For example `strength` and `magic` can be a `i32`,
`hp` can be a `f32`, `is_dragon` can be a `bool` etc. `Stat`s are usually enums and you might find
the `strum` crate useful in implementing them.

## Unordered StatStream

`bevy_stat_query` uses unordered operations to build up stats. This includes
`add`, `multiply`, `min`, `max` and `or`. This ensures no explicit ordering is
ever needed when querying for stats.

Each stat has its components form `StatValue`, e.g. `(12 * 4).min(99).max(0)`,
and its evaluated form, e.g. `48`. You can implement your own `StatValue`
to achieve custom behaviors.

## Queries

`StatQuery` is the `SystemParam` to query stats. `StatQuery` only collects `StatEntity`s, which are
marker components for queryable entities. To actually query for stats, you need to join it with
`ComponentStream`s and `RelationStream`s. They can query stats from components and children of
the `Entity`.

### Relations

All `StatStream`s have access to a `Querier`, which can query for other stats
from any entity in the world. In addition, `RelationStream` allows the stat system to
query for relationship between entities, for example to model an aura effect base on distance.

## StatMap

`StatMap` is a optimized map like storage for all stats that implements `StatStream`.

## Serialization

Due to the type of dynamic dispatch used by `StatMap`, we only have native serialization support
via `bevy_serde_lens`.
Call `StatExtension::register_stat` on the world for each `Stat` used in deserialization.

To use `Reflect` deserialization you must wrap your deserialization inside
a `bevy_serde_lens_core::private::de_scope` scope.
tatCache

A resource that must be manually added.
If added, will cache all query results.
If state has changed, must be manually cleared
either via `StatQuery` or directly on the resource.

## GlobalStatDefaults

A resource that contains default values of stats. If
you want to constrain `HP` to `0..=99` it should be done here.

Extension methods exists on the `App` like `StatExtension::register_stat_max` to
set default values of stats.

## GlobalStatRelations

A resource that contains `StatStream`s that runs on all queries.

Extension method `StatExtension::register_stat_relation` on `App` can be used to
register these.

## Versions

| bevy | bevy-stat-query |
|------|-----------------|
| 0.13 | 0.0.x           |
| 0.14 | 0.1 - latest    |

## License

Licensed under either of

* Apache License, Version 2.0 (LICENSE-APACHE(LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
* MIT license (LICENSE-MIT(LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
