use std::marker::PhantomData;

use bevy_app::{App, Plugin};
use bevy_ecs::{entity::Entity, system::{Resource, SystemId}, world::World};

use crate::{calc::StatDefaults, querier::ErasedQuerier, Stat, QualifierFlag, QualifierQuery, StatComponents, StatOperation};

/// [`Plugin`] for the stat engine.
#[derive(Debug, Default)]
pub struct StatEnginePlugin;

impl Plugin for StatEnginePlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.init_resource::<StatDefaults>();
    }
}

#[derive(Debug, Resource)]
pub struct QuerySysId<Q: QualifierFlag, S: Stat>(SystemId<(Entity, QualifierQuery<Q>, S), Option<S::Data>>, PhantomData<(Q, S)>);

type Bounds<T> = <<T as Stat>::Data as StatComponents>::Bounds;

/// Extension on [`World`] and [`App`]
pub trait StatExtension {
    /// Register a default stat value.
    ///
    /// Since a component can be supplied instead of a raw value,
    /// this is the standard way
    /// to add default bounds to a stat, e.g, in `1..15`.
    fn register_stat_default<S: Stat>(&mut self, stat: S, value: S::Data);

    /// Register the minimum value of a stat.
    fn register_stat_min<S: Stat>(&mut self, stat: &S, value: Bounds<S>);

    /// Register the maximum value of a stat.
    fn register_stat_max<S: Stat>(&mut self, stat: &S, value: Bounds<S>);

    /// Query for a stat on an [`Entity`] with [`World`] access.
    fn query_stat<E: ErasedQuerier, S: Stat>(
        &mut self,
        entity: Entity,
        qualifier: &QualifierQuery<E::Qualifier>,
        stat: &S,
    ) -> Option<S::Data>;

    /// Query for a stat on an [`Entity`] with [`World`] access.
    fn query_eval_stat<E: ErasedQuerier, S: Stat>(
        &mut self,
        entity: Entity,
        qualifier: &QualifierQuery<E::Qualifier>,
        stat: &S,
    ) -> Option<<S::Data as StatComponents>::Out> {
        self.query_stat::<E, S>(entity, qualifier, stat)
            .map(|x| x.eval())
    }
}

impl StatExtension for World {
    fn register_stat_default<S: Stat>(&mut self, stat: S, value: S::Data) {
        self.get_resource_mut::<StatDefaults>().unwrap()
            .insert(stat, value);
    }

    fn register_stat_min<S: Stat>(&mut self, stat: &S, value: Bounds<S>) {
        self.get_resource_mut::<StatDefaults>().unwrap()
            .patch(stat, StatOperation::Min(value))
    }

    fn register_stat_max<S: Stat>(&mut self, stat: &S, value: Bounds<S>) {
        self.get_resource_mut::<StatDefaults>().unwrap()
            .patch(stat, StatOperation::Max(value))
    }

    fn query_stat<E: ErasedQuerier, S: Stat>(
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
    fn register_stat_default<S: Stat>(&mut self, stat: S, value: S::Data) {
        self.world.get_resource_mut::<StatDefaults>().unwrap()
            .insert(stat, value);
    }

    fn register_stat_min<S: Stat>(&mut self, stat: &S, value: Bounds<S>) {
        self.world.register_stat_min(stat, value)
    }

    fn register_stat_max<S: Stat>(&mut self, stat: &S, value: Bounds<S>) {
        self.world.register_stat_max(stat, value)
    }

    fn query_stat<E: ErasedQuerier, S: Stat>(
        &mut self,
        entity: Entity,
        qualifier: &QualifierQuery<E::Qualifier>,
        stat: &S,
    ) -> Option<S::Data> {
        self.world.query_stat::<E, S>(entity, qualifier, stat)
    }
}
