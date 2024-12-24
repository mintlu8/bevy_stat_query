# bevy-stat-query

[![Crates.io](https://img.shields.io/crates/v/bevy_stat_query.svg)](https://crates.io/crates/bevy_stat_query)
[![Docs](https://docs.rs/bevy_stat_query/badge.svg)](https://docs.rs/bevy_stat_query/latest/bevy_stat_query/)
[![Bevy tracking](https://img.shields.io/badge/Bevy%20tracking-released%20version-lightblue)](https://bevyengine.org/learn/book/plugin-development/)

Versatile RPG stat system for the bevy engine.

## Overview

In order to represent stats, stat buffs and stat queries in an ECS,
`bevy_stat_query` exclusively uses unordered operations to represent
stats, this includes `add`, `multiply`, `min`, `max` and `or`.

For instance if we want to evaluate a character's strength,
taken into account buffs and debuffs this can look something like this:

```rust
clamp((42 + 4 + 7 + (-4)) * 2 * 0.75, 1, 99)
```

Note how the order of evaluation doesn't matter, which fits perfectly into
the "insert component and have effect" usage pattern of the ECS.

## Qualified Stats

We describe each stat as a `Qualifier` and a `Stat`.
`Stat` is a noun like *strength* or *damage* and
`Qualifier` are adjectives that describes
what this `Stat` can be applied to.

For example in *fire magic damage*, *(fire, magic)* is the `Qualifier`,
*damage* is the `Stat`.

## Modifier and Query

There are actually two types of stats, `modifier` and `query`.

A `modifier` is something alone the lines of

```text
Increase (fire, magic) damage by 5.
```

While a query is

```text
This attack does (fire, magic) damage.
```

When querying for *(fire, magic) damage*, all modifiers that boosts
*damage*, *fire damage*, *magic damage* or *(fire, magic) damage*
can apply to this query.
While modifier that boosts a different qualifier *ice damage* or a
different stat *fire defense* does not apply to this query.

In `bevy_stat_query`,
a modifier is represented as `(Qualifier, Stat, Value)` while a
query is represented as `(QualifierQuery, Stat)`.

* Conditional Modifiers

A common trope in fantasy games is the modifier `elemental damage`, which applies to
any of fire, ice, etc. In `Qualifier` this is the `any_of` field.

* Exact Query

Imagine we have an effect like this:

```text
Add 50% of the character's magic damage to physical damage.
```

In order to avoid duplication, since effects boosting `damage` applies to
both, we can use `QualifierQuery::exact`.

## Traits

Qualifier is usually a bitflags implementing `QualifierFlag`, Stat is usually an enum deriving `Stat`.

An app usually has a single `QualifierFlag` but multiple `Stat` implementors,
since each `Stat` can associate to a different type.
For example `strength` and `magic` can be a `i32`,
`hp` can be a `f32`, `is_dragon` can be a `bool` etc.

Different types of stats can still query each other via `Querier`
to model effects like

```text
If user is a dragon, increase damage by 50%.
```

## `StatStream` and `QueryStream`

In order for components to contribute to stats, you must implement `QueryStream`. `StatStream` can be used
if no additional querying is needed. A `Component` that implements `StatStream` is automatically a `QueryStream`.

In order to use `QueryStream`, mark queryable entities as `StatEntity`.
Then add `StatEntities` to you system and join it with various `QueryStream`s.

```rust
fn stat_query(
    entities: StatEntities,
    stat_maps: StatQuery<StatMap>,
    weapons: StatQuery<Weapon>,
    buffs: ChildQuery<Buff>,
) {
    let querier = entities.join(&stat_maps).join(&weapons).join(&buffs);
    let damage = querier.eval_stat(&MyQualifier::Magic, &MyStat::Damage).unwrap();
}
```

Using `bevy_stat_query` is significantly easier if you have access to `&mut World`.
One-shot systems are recommended to perform queries.

## Relations

`StatStream` and `QueryStream` provides the `stream_relation` function that makes it easier to implement
relation based effects like

```text
Increase damage of all allies within 3 yards by 5.
```

Checkout one of our examples on how to implement this.

## Versions

| bevy | bevy-stat-query |
|------|-----------------|
| 0.15 | 0.1 - latest    |

## License

Licensed under either of

* Apache License, Version 2.0 (LICENSE-APACHE(LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
* MIT license (LICENSE-MIT(LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
