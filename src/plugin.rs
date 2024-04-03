use std::{any::{Any, TypeId}, cell::RefCell, future::{ready, Future}, ops::Deref, rc::Rc, str::FromStr};
use bevy_app::{App, First, Plugin};
use bevy_ecs::{entity::Entity, query::With, system::{Query, ReadOnlySystem, Resource, SystemId}, world::World};
use bevy_utils::HashMap;
use crate::{querier::QuerierIn, Data, StatCache, StatEntity, StatInstances};
use crate::{calc::StatDefaults, querier::GenericQuerier, types::DynStatValue, QualifierFlag, QualifierQuery, Stat, StatOperation, StatValue};

#[derive(Debug, Resource)]
pub struct ClearCacheId(SystemId);


#[derive(Default)]
pub struct CachedQueriers{
    init: RefCell<HashMap<TypeId, Box<dyn Any>>>,
    uninit: RefCell<Vec<(TypeId, Box<dyn FnOnce(&mut World) -> Box<dyn Any>>)>>
}


#[derive(Default, Clone)]
pub struct CachedQueriersRes(Rc<CachedQueriers>);

impl Deref for CachedQueriersRes {
    type Target = CachedQueriers;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

type BoxSystem<Q, S> = Box<dyn ReadOnlySystem<In = QuerierIn<Q, S>, Out = Option<<S as Stat>::Data>>>;

impl CachedQueriers {
    pub fn query_stat<Q: GenericQuerier, S: Stat>(&self, world: &mut World, input: QuerierIn<Q::Qualifier, S>) -> Option<S::Data> {
        let mut lock = self.init.borrow_mut();
        if let Some(state) = lock.get_mut(&TypeId::of::<(Q, S)>()) {
            state.downcast_mut::<BoxSystem<Q::Qualifier, S>>().unwrap().run_readonly(input, world)
        } else {
            let mut state = Q::as_boxed_readonly_system::<S>();
            state.initialize(world);
            let result = state.run_readonly(input, world);
            lock.insert(TypeId::of::<(Q, S)>(), Box::new(state));
            result
        }
    }

    pub fn try_query_stat<Q: GenericQuerier, S: Stat>(&self, world: &World, input: QuerierIn<Q::Qualifier, S>) -> Option<Option<S::Data>> {
        let mut lock = self.init.borrow_mut();
        if let Some(state) = lock.get_mut(&TypeId::of::<(Q, S)>()) {
            Some(state.downcast_mut::<BoxSystem<Q::Qualifier, S>>().unwrap().run_readonly(input, world))
        } else {
            self.uninit.borrow_mut().push((TypeId::of::<(Q, S)>(), Box::new(|w| {
                let mut sys = Q::as_boxed_readonly_system::<S>();
                sys.initialize(w);
                Box::new(sys)
            })));
            None
        }
    }

    #[cfg(feature = "futures")]
    pub fn async_query_stat<Q: GenericQuerier, S: Stat>(&self, world: &World, input: QuerierIn<Q::Qualifier, S>) -> impl Future<Output = Option<S::Data>> + 'static {
        use futures::{channel::oneshot::channel, future::Either, FutureExt};
        let mut lock = self.init.borrow_mut();
        if let Some(state) = lock.get_mut(&TypeId::of::<(Q, S)>()) {
            Either::Left(ready(
                state.downcast_mut::<BoxSystem<Q::Qualifier, S>>().unwrap().run_readonly(input, world)
            ))
        } else {
            let (send, recv) = channel();
            self.uninit.borrow_mut().push((TypeId::of::<(Q, S)>(), Box::new(move |w| {
                let mut sys = Q::as_boxed_readonly_system::<S>();
                sys.initialize(w);
                let _ = send.send(sys.run_readonly(input, w));
                Box::new(sys)
            })));
            Either::Right(recv.map(|x| x.ok().flatten()))
        }
    }

    pub fn init<Q: GenericQuerier, S: Stat>(&self, world: &mut World) {
        let mut state = Q::as_boxed_readonly_system::<S>();
        state.initialize(world);
        self.init.borrow_mut().insert(TypeId::of::<(Q, S)>(), Box::new(state));
    }

    pub fn init_all(&self, world: &mut World) {
        self.init.borrow_mut().extend(self.uninit.borrow_mut().drain(..).map(|(k, v)| (k, v(world))));
    }
}

type Bounds<T> = <<T as Stat>::Data as StatValue>::Bounds;

/// Extension on [`World`] and [`App`]
pub trait StatExtension {
    /// Register associated serialization routine for a stat.
    /// 
    /// # Panics
    /// 
    /// If trying to replace a previous stat entry with a different value.
    fn register_stat<T: Stat>(&mut self) -> &mut Self;
    /// Register associated serialization routine for a stat that uses [`FromStr`].
    /// 
    /// # Panics
    /// 
    /// If trying to replace a previous stat entry with a different value.
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

    /// Query for a stat on an [`Entity`] with mutable [`World`] access. 
    /// Returns `None` only if entity is missing.
    fn query_stat<Q: GenericQuerier, S: Stat>(
        &mut self,
        entity: Entity,
        qualifier: &QualifierQuery<Q::Qualifier>,
        stat: &S,
    ) -> Option<S::Data>;

    /// Query for a stat on an [`Entity`] with immutable [`World`] access.
    /// 
    /// Returns `None` if the querier system is not registered, 
    /// by default this is deferred and will be available the next frame.
    /// 
    /// Returns `Some(None)` if the entity is missing.
    fn try_query_stat<Q: GenericQuerier, S: Stat>(
        &self,
        entity: Entity,
        qualifier: &QualifierQuery<Q::Qualifier>,
        stat: &S,
    ) -> Option<Option<S::Data>>;

    /// Query for a stat on an [`Entity`] with immutable [`World`] access
    /// asynchronously. 
    /// 
    /// This completes instantly if `try_query_stat` can complete.
    /// 
    /// Returns `None` only if the entity is missing.
    #[cfg(feature = "futures")]
    fn async_query_stat<Q: GenericQuerier, S: Stat>(
        &self,
        entity: Entity,
        qualifier: &QualifierQuery<Q::Qualifier>,
        stat: &S,
    ) -> impl Future<Output = Option<S::Data>> + Send + Sync + 'static;

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

    fn clear_stat_cache<Q: QualifierFlags>(&mut self);

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

    fn query_stat<Q: GenericQuerier, S: Stat>(
        &mut self,
        entity: Entity,
        qualifier: &QualifierQuery<Q::Qualifier>,
        stat: &S,
    ) -> Option<S::Data> {
        let input = (entity, qualifier.clone(), stat.clone());
        let queriers = self.non_send_resource::<CachedQueriersRes>().clone();
        queriers.query_stat::<Q, S>(self, input)
    }

    fn try_query_stat<Q: GenericQuerier, S: Stat>(
            &self,
            entity: Entity,
            qualifier: &QualifierQuery<Q::Qualifier>,
            stat: &S,
    ) -> Option<Option<S::Data>> {
        let input = (entity, qualifier.clone(), stat.clone());
        let queriers = self.non_send_resource::<CachedQueriersRes>().clone();
        queriers.try_query_stat::<Q, S>(self, input)
    }

    #[cfg(feature = "futures")]
    fn async_query_stat<Q: GenericQuerier, S: Stat>(
        &self,
        entity: Entity,
        qualifier: &QualifierQuery<Q::Qualifier>,
        stat: &S,
    ) -> impl Future<Output = Option<S::Data>> + Send + Sync + 'static {
        let input = (entity, qualifier.clone(), stat.clone());
        let queriers = self.non_send_resource::<CachedQueriersRes>().clone();
        queriers.async_query_stat::<Q, S>(self, input)
    }

    fn clear_stat_cache<Q: QualifierFlags>(&mut self) {
        let id = if let Some(res) = self.get_resource::<ClearCacheId>() {
            res.0
        } else {
            let id = self.register_system(|query: Query<&StatCache<Q>, With<StatEntity>>| {
                query.iter().for_each(StatCache::invalidate_all)
            });
            self.insert_resource(ClearCacheId(id));
            id
        };
        self.run_system(id).unwrap();
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

    fn query_stat<Q: GenericQuerier, S: Stat>(
        &mut self,
        entity: Entity,
        qualifier: &QualifierQuery<Q::Qualifier>,
        stat: &S,
    ) -> Option<S::Data> {
        self.world.query_stat::<Q, S>(entity, qualifier, stat)
    }

    fn try_query_stat<Q: GenericQuerier, S: Stat>(
        &self,
        entity: Entity,
        qualifier: &QualifierQuery<Q::Qualifier>,
        stat: &S,
    ) -> Option<Option<S::Data>> {
        self.world.try_query_stat::<Q, S>(entity, qualifier, stat)
    }

    #[cfg(feature = "futures")]
    fn async_query_stat<Q: GenericQuerier, S: Stat>(
        &self,
        entity: Entity,
        qualifier: &QualifierQuery<Q::Qualifier>,
        stat: &S,
    ) -> impl Future<Output = Option<S::Data>> + Send + Sync + 'static {
        self.world.async_query_stat::<Q, S>(entity, qualifier, stat)
    }

    fn clear_stat_cache<Q: QualifierFlags>(&mut self) {
        self.world.clear_stat_cache::<Q>()
    }
}

/// Optional, enable queriers on the [`World`].
#[derive(Debug, Default, Clone, Copy)]
pub struct StatQueryPlugin;

impl Plugin for StatQueryPlugin {
    fn build(&self, app: &mut App) {
        app.init_non_send_resource::<CachedQueriersRes>();
        app.add_systems(First, |world: &mut World| {
            let queriers = world.non_send_resource::<CachedQueriersRes>().clone();
            queriers.init_all(world)
        });
    }
}