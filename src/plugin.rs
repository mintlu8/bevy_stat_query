use std::{marker::PhantomData, str::FromStr};
use bevy_app::App;
use bevy_ecs::{entity::Entity, system::{Resource, SystemId}, world::World};
use crate::{Data, StatInstances};
use crate::{calc::StatDefaults, querier::GenericQuerier, types::DynStatValue, QualifierFlag, QualifierQuery, Stat, StatOperation, StatValue};

#[derive(Debug, Resource)]
pub struct QuerySysId<Q: QualifierFlag, S: Stat>(SystemId<(Entity, QualifierQuery<Q>, S), Option<S::Data>>, PhantomData<(Q, S)>);

type Bounds<T> = <<T as Stat>::Data as StatValue>::Bounds;

/// Extension on [`World`] and [`App`]
pub trait StatExtension {
    /// Register associated serialization routine for a stat.
    fn register_stat<T: Stat>(&mut self) -> &mut Self;
    /// Register associated serialization routine for a stat that uses [`FromStr`].
    fn register_stat_parser<T: Stat + FromStr>(&mut self) -> &mut Self;
    /// Register a default stat value.
    ///
    /// This is the standard way
    /// to add default bounds to a stat, e.g, in `1..15`.
    fn register_stat_default<S: Stat>(&mut self, stat: S, value: S::Data) -> &mut Self;

    /// Register the minimum value of a stat.
    fn register_stat_min<S: Stat>(&mut self, stat: &S, value: Bounds<S>) -> &mut Self;

    /// Register the maximum value of a stat.
    fn register_stat_max<S: Stat>(&mut self, stat: &S, value: Bounds<S>) -> &mut Self;

    /// Query for a stat on an [`Entity`] with [`World`] access.
    fn query_stat<E: GenericQuerier, S: Stat>(
        &mut self,
        entity: Entity,
        qualifier: &QualifierQuery<E::Qualifier>,
        stat: &S,
    ) -> Option<S::Data>;

    /// Query for a stat on an [`Entity`] with [`World`] access.
    fn query_eval_stat<E: GenericQuerier, S: Stat>(
        &mut self,
        entity: Entity,
        qualifier: &QualifierQuery<E::Qualifier>,
        stat: &S,
    ) -> Option<<S::Data as StatValue>::Out> {
        self.query_stat::<E, S>(entity, qualifier, stat)
            .map(|x| x.eval())
    }
}

impl StatExtension for World {
    fn register_stat<T: Stat>(&mut self) -> &mut Self {
        use bevy_serde_project::WorldExtension;
        self.register_typetag::<Box<dyn DynStatValue>, T::Data>();
        self.register_typetag::<Box<dyn Data>, <T::Data as StatValue>::Out>();
        self.register_typetag::<Box<dyn Data>, StatOperation<T::Data>>();
        self.get_resource_or_insert_with::<StatInstances>(Default::default)
            .register::<T>();
        self
    }

    fn register_stat_parser<T: Stat + FromStr>(&mut self) -> &mut Self {
        use bevy_serde_project::WorldExtension;
        self.register_typetag::<Box<dyn DynStatValue>, T::Data>();
        self.register_typetag::<Box<dyn Data>, <T::Data as StatValue>::Out>();
        self.register_typetag::<Box<dyn Data>, StatOperation<T::Data>>();
        self.get_resource_or_insert_with::<StatInstances>(Default::default)
            .register_parser::<T>();
        self
    }

    fn register_stat_default<S: Stat>(&mut self, stat: S, value: S::Data) -> &mut Self {
        self.get_resource_or_insert_with::<StatDefaults>(Default::default)
            .insert(stat, value);
        self
    }

    fn register_stat_min<S: Stat>(&mut self, stat: &S, value: Bounds<S>) -> &mut Self {
        self.get_resource_or_insert_with::<StatDefaults>(Default::default)
            .patch(stat, StatOperation::Min(value));
        self
    }

    fn register_stat_max<S: Stat>(&mut self, stat: &S, value: Bounds<S>) -> &mut Self {
        self.get_resource_or_insert_with::<StatDefaults>(Default::default)
            .patch(stat, StatOperation::Max(value));
        self
    }

    fn query_stat<E: GenericQuerier, S: Stat>(
        &mut self,
        entity: Entity,
        qualifier: &QualifierQuery<E::Qualifier>,
        stat: &S,
    ) -> Option<S::Data> {
        let id = if let Some(res) = self.get_resource::<QuerySysId<E::Qualifier, S>>() {
            res.0
        } else {
            let id = self.register_system(E::system);
            self.insert_resource(QuerySysId::<E::Qualifier, S>(id, PhantomData));
            id
        };
        self.run_system_with_input(id, (entity, qualifier.clone(), stat.clone())).unwrap()
    }
}

impl StatExtension for App {
    fn register_stat<T: Stat>(&mut self) -> &mut Self {
        self.world.register_stat::<T>();
        self
    }

    fn register_stat_parser<T: Stat + FromStr>(&mut self) -> &mut Self {
        self.world.register_stat_parser::<T>();
        self
    }

    fn register_stat_default<S: Stat>(&mut self, stat: S, value: S::Data) -> &mut Self {
        self.world.register_stat_default::<S>(stat, value);
        self
    }

    fn register_stat_min<S: Stat>(&mut self, stat: &S, value: Bounds<S>) -> &mut Self {
        self.world.register_stat_min(stat, value);
        self
    }

    fn register_stat_max<S: Stat>(&mut self, stat: &S, value: Bounds<S>) -> &mut Self {
        self.world.register_stat_max(stat, value);
        self
    }

    fn query_stat<E: GenericQuerier, S: Stat>(
        &mut self,
        entity: Entity,
        qualifier: &QualifierQuery<E::Qualifier>,
        stat: &S,
    ) -> Option<S::Data> {
        self.world.query_stat::<E, S>(entity, qualifier, stat)
    }
}
