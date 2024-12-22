# bevy-stat-query

Versatile RPG stat system for the bevy engine.

## Overview

In order to represent stats, stat buffs and stat queries in an ECS,
`bevy_stat_query` exclusively uses unordered operations to represent
stats, this includes `add`, `multiply`, `min`, `max` and `or`.

For instance if we want to evaluate a character's strength,
taken into account buffs and debuffs this can look something like this:

```rust
clamp((42 + 4 + 7) * 2 * 0.75, 1, 99)
```

Note how the order of evaluation doesn't matter, which fits perfectly into
the "insert component and have effect" usage pattern of the ECS.

## Qualified Stats

We describe each stat as a `Qualifier` and a `Stat`.
`Stat` is a noun like "Strength" or "Magic" and
`Qualifier` are adjectives that describes
what this `Stat` can be applied to.

For example in "FireMagicDamage", "Fire|Magic" is the qualifier,
"Damage" is the `Stat`.

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

When querying for `(fire, magic) damage`, all modifiers that boosts
`damage`, `fire damage` or `magic damage` can apply to this query,
while modifier that boosts a different qualifier `ice damage` or a
different stat `fire defense` does not apply to this query.

In `bevy_stat_query`,
modifier is represented as `(Qualifier, Stat, Value)` and a
query is represented as `(QualifierQuery, Stat)`.

* Conditional Modifiers

A common trope in fantasy games is `elemental damage`, which applies to
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

An app usually has a single `QualifierFlag` but multiple `Stat` implementors. Since each `Stat` can associate to a different type.
For example `strength` and `magic` can be a `i32`,
`hp` can be a `f32`, `is_dragon` can be a `bool` etc.

## Unordered Operations

`bevy_stat_query` uses unordered operations to build up stats. This includes
`add`, `multiply`, `min`, `max` and `or`. This ensures no explicit ordering is
ever needed when querying for stats.

Each stat has its components form `StatValue`, e.g. `(12 * 4).min(99).max(0)`,
and its evaluated form, e.g. `48`.

## Queries

`StatQuery` is the `SystemParam` to query stats. `StatQuery` only collects `StatEntity`s, which are
marker components for queryable entities. To actually query for stats, you need to join it with
`ComponentStream`s and `RelationStream`s. They can query stats from components and children of
the `Entity`.

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
