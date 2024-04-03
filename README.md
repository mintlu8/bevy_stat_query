# bevy-stat-query

An over-engineered RPG stat system for the bevy engine.

## Qualified Stats

We describe each stat as a `Qualifier` and a `Stat`.
`Stat` is a concrete stat noun like `Strength`, `Magic`, etc.
`Qualifier` is a flags based adjective that describes
what this `Stat` can be applied to.

For example in `FireMagicDamage`, `Fire|Magic` is the qualifier,
`Damage` is the `Stat`.

What this means if an effect boosts `Fire|Damage`, `Magic|Damage`,
or simply just `Damage`, the effect will be applied to the stat,
but an effect on `Sword|Damage` or `Fire|Range` won't be applied to the stat.

### Qualifier

`Qualifier` is tied to effects, and provides the aforementioned `all_of`.
In addition `any_of` is provided for modelling conditional effects like
`Elemental|Damage`, which means `Fire or Water Damage` instead of `Fire and Water Damage`.

Each `Qualifier` can only have one group of `any_of` which is a limitation currently.

#### Examples

```rust
let fire = Qualifier::all_of(Flag::Fire);
let fire_magic = Qualifier::all_of(Flag::Fire|Flag::Magic);
let elemental = Qualifier::any_of(Fire|Water|Air|Earth);
let elemental_magic = Qualifier::any_of(Fire|Water|Air|Earth)
    .and_all_of(Magic);
```

### QualifierQuery

`QualifierQuery` matches all `Qualifiers` on our character that
qualifies as the query we are looking for.

`QualifierQuery::Aggregate` collects all qualifiers that matches the query.

For example, suppose we are looking for `(Fire|Burn|Magic, Damage)`:

* `((), Damage)` qualifies.
* `(Fire, Damage)` qualifies.
* `(Fire|Magic, Damage)` qualifies.
* `(Fire|Burn|Magic, Damage)` qualifies.
* `(Elemental, Damage)` qualifies.
* `(Fire|Sword, Damage)` does not qualify.
* `(Fire|Burn|Magic, Defense)` does not qualify.

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

* What do you mean? My `DarkFire` and `Fire` and totally different things and should be independent.

Create a new qualifier `DarkFire` instead of `Dark`|`Fire`.

## Getting Started

Add marker component `StatEntity` to an `Entity`.
If you need caching, add a `StatCache` as well.
You need to manually clear the cache when the state is changed, however.

* Implement `IntrinsicStream` to make components on the entity queryable.
* Implement `ExternalStream` to make components on child entities queryable.

For example we can add `BaseStatMap` to the `Entity` as base stats, if we include
it in the `intrinsic` section of the `querier!` macro.

## Querier

`StatQuerier` is the `SystemParam` to query stats, it is quite difficult to
define one manually so the recommended way is to define a `type` with the
`querier!` macro. Additionally we can also use the `StatExtension` with `World` access
for similar functionalities.

### Example

```rust
querier!(pub UnitStatQuerier {
    qualifier: MyQualifier,
    intrinsic: {
        Allegiance,
        Position
    },
    external: {
        Weapon,
        Ability,
        Effect,
        Potion,
    }
});
```

## Unordered StatStream

`bevy_stat_query` uses unordered operations to build up stats. This includes
`add`, `multiply`, `min`, `max` and `or`. This ensures no explicit ordering is
ever needed when querying for stats.

Each stat has its components form `StatValue`, e.g. `(12 * 4).min(99).max(0)`,
and its evaluated form, e.g. `48`. You can implement your own `StatValue`
to achieve custom behaviors. `StatOperation` stores a single operation
that can be written to a `StatValue`.

### Stat Relation

We can create relations between different
stats using either their components form or their evaluated form.
`StatStream`s are allowed to query other stats or other entities.
Since stat operations are unordered, dependency cycles cannot be resolved.
If a cycle is detected, an error will be thrown.

### Entity Relation

`IntrinsicStream` can be used to provide bi-entity relationship
like `distance` or `allegiance`. This can be used to model range based effects.

You may find `StatOnce` useful in implementing these.

## Note

* `StatQuerier` requires read access to all components in stat system so we cannot mutate
anything while having it as a parameter.
Using system piping or some kind of deferred command queue for mutations
might be advisable in this case.

* The crate heavily utilizes dynamic dispatch under the hood, and is therefore
not fully reflect compatible. The supported serialization method is
through the `bevy-serde-project` crate, Check out that crate for more information.

## Versions

| bevy | bevy-stat-query |
|------|-----------------|
| 0.13 | latest          |

## License

Licensed under either of

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
