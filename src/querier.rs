use std::marker::PhantomData;

use bevy_ecs::{entity::Entity, query::With, system::{In, Query, ReadOnlySystem, Res, StaticSystemParam, SystemParam}};
use bevy_hierarchy::Children;
use bevy_utils::hashbrown::HashMap;
use dyn_clone::clone_box;
use crate::{param::IntrinsicParam, types::DynStatValue, DynStat, QualifierFlag, QualifierQuery, Stat, StatDefaults, StatParam, StatValuePair, TYPE_ERROR};
use crate::{StatCache, StatEntity, StatValue};

#[derive(SystemParam)]
struct QuerierInner<'w, 's,
    Qualifier: QualifierFlag,
    Intrinsic: IntrinsicParam<Qualifier> + 'static,
    Components: StatParam<Qualifier> + 'static
> {
    defaults: Option<Res<'w, StatDefaults>>,
    units: Query<'w, 's, Option<&'static Children>, With<StatEntity>>,
    intrinsic: StaticSystemParam<'w, 's, Intrinsic>,
    items: StaticSystemParam<'w, 's, Components>,
    p: PhantomData<Qualifier>
}

/// A [`SystemParam`] that allows the user to query stats by [`Entity`].
///
/// This requires immutable access to many components and entities,
/// assume all relevant components are not mutably accessible within the same system.
///
/// Alternatively this can be ran with world access with [`query_stat`](crate::StatExtension::query_stat).
#[derive(SystemParam)]
pub struct StatQuerier<'w, 's,
    Qualifier: QualifierFlag,
    Intrinsic: IntrinsicParam<Qualifier> + 'static,
    Components: StatParam<Qualifier> + 'static
> {
    querier: QuerierInner<'w, 's, Qualifier, Intrinsic, Components>,
    cache: Query<'w, 's, &'static StatCache<Qualifier>, With<StatEntity>>,
}

struct QueryStack<'w, 's, 'w2, 's2, 't, Q: QualifierFlag, D: IntrinsicParam<Q> + 'static, A: StatParam<Q> + 'static> {
    world_cache: &'t Query<'w2, 's2, &'static StatCache<Q>, With<StatEntity>>,
    current_cache: HashMap<(Entity, QualifierQuery<Q>, Box<dyn DynStat>), Box<dyn DynStatValue>>,
    querier: &'t QuerierInner<'w, 's, Q, D, A>,
    stack: Vec<(QualifierQuery<Q>, Box<dyn DynStat>, Entity)>,
}

trait DynQuerier<Q: QualifierFlag> {
    fn query(&mut self, qualifier: &QualifierQuery<Q>, stat: &dyn DynStat) -> Option<Box<dyn DynStatValue>>;
    fn query_other(&mut self, entity: Entity, qualifier: &crate::QualifierQuery<Q>, stat: &dyn DynStat) -> Option<Box<dyn DynStatValue>>;
    fn query_distance(&mut self, entity: Entity, qualifier: &crate::QualifierQuery<Q>, stat: &dyn DynStat) -> Option<Box<dyn DynStatValue>>;
}

impl<Q: QualifierFlag, D: IntrinsicParam<Q> + 'static, A: StatParam<Q> + 'static> DynQuerier<Q> for QueryStack<'_, '_, '_, '_, '_, Q, D, A> {
    fn query(&mut self, qualifier: &QualifierQuery<Q>, stat: &dyn DynStat) -> Option<Box<dyn DynStatValue>> {
        let entity = self.stack.last().expect("Must call query_other on the first call.").2;
        self.query_other(entity, qualifier, stat)
    }

    fn query_other(&mut self, entity: Entity, qualifier: &crate::QualifierQuery<Q>, stat: &dyn DynStat) -> Option<Box<dyn DynStatValue>> {
        if self.stack.iter().any(|(q, s, e)| q == qualifier && s.as_ref() == stat && e == &entity) {
            panic!("A cycle detected in stat dependencies when querying ({:?}, {:?}, {:?}). \
                This query cannot be completed {:?}.",
                qualifier, stat, entity, self.stack
            )
        };
        self.stack.push((qualifier.clone(), clone_box(stat), entity));
        let Ok(children) = self.querier.units.get(entity) else { return None; };

        if let Some(cached) = match self.world_cache.get(entity){
            Ok(cache) => cache.try_get_cached_dyn(qualifier, stat),
            Err(_) => None,
        } {
            return Some(cached)
        }
        let queried = match children {
            Some(children) => children.iter(),
            None => [].iter(),
        };
        let mut stat_value = self.querier.defaults.as_ref()
            .map(|x| x.get_dyn(stat))
            .unwrap_or_else(||stat.default_value());
        let mut pair = StatValuePair(stat, stat_value.as_mut());
        A::stream(&*self.querier.items, queried, qualifier, &mut pair, &mut QuerierRef(self));
        let Some(_) = self.stack.pop() else {panic!("Stack mismatch.")};
        if let Ok(cache) = self.world_cache.get(entity) {
            cache.cache_dyn(qualifier.clone(), clone_box(stat), stat_value.clone());
        } else {
            self.current_cache.insert(
                (entity, qualifier.clone(), clone_box(stat)),
                stat_value.clone()
            );
        }
        Some(stat_value)
    }

    fn query_distance(&mut self, entity: Entity, qualifier: &crate::QualifierQuery<Q>, stat: &dyn DynStat) -> Option<Box<dyn DynStatValue>> {
        let curr = self.stack.last().expect("Must call query_other on first call.").2;
        if !self.querier.units.contains(entity) || !self.querier.units.contains(curr) {
            return None;
        }
        let mut stat_value = self.querier.defaults.as_ref()
            .map(|x| x.get_dyn(stat))
            .unwrap_or_else(||stat.default_value());        
        let mut pair = StatValuePair(stat as &dyn DynStat, stat_value.as_mut());
        D::distance_stream(&*self.querier.intrinsic, curr, entity, qualifier, &mut pair, &mut QuerierRef(self));
        Some(stat_value)
    }
}

/// Erased querier with a typed interface.
pub struct QuerierRef<'t, Q: QualifierFlag>(&'t mut dyn DynQuerier<Q>);

type SOut<S> = <<S as Stat>::Data as StatValue>::Out;

impl<Q: QualifierFlag> QuerierRef<'_, Q> {

    /// Look for a [`StatValue`] on this entity, returns `None` if entity is missing.
    pub fn query<S: Stat>(&mut self, qualifier: &QualifierQuery<Q>, stat: &S) -> Option<S::Data> {
        self.0.query(qualifier, stat)
            .map(|x| *x.downcast().expect(TYPE_ERROR))
    }

    /// Look for a stat output on this entity, returns `None` if entity is missing.
    pub fn query_eval<S: Stat>(&mut self, qualifier: &QualifierQuery<Q>, stat: &S) -> Option<SOut<S>> {
        self.query(qualifier, stat).map(|x| x.eval())
    }

    /// Look for a [`StatValue`] on another entity, returns `None` if entity is missing.
    pub fn query_other<S: Stat>(&mut self, entity: Entity, qualifier: &crate::QualifierQuery<Q>, stat: &S) -> Option<S::Data> {
        self.0.query_other(entity, qualifier, stat)
            .map(|x| *x.downcast().expect(TYPE_ERROR))    
    }

    /// Look for a stat output on another entity, returns `None` if entity is missing.
    pub fn query_eval_other<S: Stat>(&mut self, entity: Entity, qualifier: &crate::QualifierQuery<Q>, stat: &S) -> Option<SOut<S>> {
        self.query_other(entity, qualifier, stat).map(|x| x.eval())    
    }

    /// Look for a relation between two entities, returns `None` if an entity is missing.
    pub fn query_distance<S: Stat>(&mut self, entity: Entity, qualifier: &crate::QualifierQuery<Q>, stat: &S) -> Option<S::Data> {
        self.0.query_distance(entity, qualifier, stat)
            .map(|x| *x.downcast().expect(TYPE_ERROR))    
    }

    /// Look for a relation between two entities, returns `None` if an entity is missing.
    pub fn query_eval_distance<S: Stat>(&mut self, entity: Entity, qualifier: &crate::QualifierQuery<Q>, stat: &S) -> Option<SOut<S>> {
        self.query_distance(entity, qualifier, stat).map(|x| x.eval())   
    }
}

impl<'w, 's, Q: QualifierFlag, D: IntrinsicParam<Q> + 'static, A: StatParam<Q> + 'static> QuerierInner<'w, 's, Q, D, A> {
    fn as_query_stack<'w2, 's2, 't>(&'t self, cache: &'t Query<'w2, 's2, &'static StatCache<Q>, With<StatEntity>>) -> QueryStack<'w, 's, 'w2, 's2, 't, Q, D, A> {
        QueryStack {
            querier: self,
            stack: Vec::new(),
            world_cache: cache,
            current_cache: HashMap::default(),
        }
    }

    pub fn query<S: Stat>(&self,
        cache: &Query<'_, '_, &'static StatCache<Q>, With<StatEntity>>,
        entity: Entity,
        qualifier: &QualifierQuery<Q>,
        stat: &S
    ) -> Option<S::Data> {
        self.as_query_stack(cache).query_other(entity, qualifier, stat)
            .map(|x| *x.downcast().expect(TYPE_ERROR))
    }
}


impl<'w, 's, Q: QualifierFlag, D: IntrinsicParam<Q> + 'static, A: StatParam<Q> + 'static> StatQuerier<'w, 's, Q, D, A> {
    pub fn query<S: Stat>(&self,
        entity: Entity,
        qualifier: &QualifierQuery<Q>,
        stat: &S
    ) -> Option<S::Data> {
        self.querier.query(&self.cache, entity, qualifier, stat)
    }

    pub fn query_eval<S: Stat>(&self,
        entity: Entity,
        qualifier: &QualifierQuery<Q>,
        stat: &S
    ) -> Option<<S::Data as StatValue>::Out> {
        self.query(entity, qualifier, stat).map(|x| x.eval())
    }
}

/// Type erased but non-dynamic [`StatQuerier`] with no generics.
pub trait GenericQuerier: SystemParam + 'static {
    type Qualifier: QualifierFlag;
    fn query<S: Stat>(&mut self,
        entity: Entity,
        qualifier: &QualifierQuery<Self::Qualifier>,
        stat: &S
    ) -> Option<S::Data>;

    fn system<S: Stat>(
        input: In<(Entity, QualifierQuery<Self::Qualifier>, S)>,
        this: StaticSystemParam<Self>,
    ) -> Option<S::Data>;

    fn as_boxed_readonly_system<S: Stat>() -> Box<dyn ReadOnlySystem<In=(Entity, QualifierQuery<Self::Qualifier>, S), Out=Option<S::Data>>>;
}

pub type QuerierIn<Q, S> = (Entity, QualifierQuery<Q>, S);

impl<Q: QualifierFlag, D: IntrinsicParam<Q> + 'static, A: StatParam<Q> + 'static> GenericQuerier for StatQuerier<'static, 'static, Q, D, A> {
    type Qualifier = Q;

    fn query<S: Stat>(&mut self,
        entity: Entity,
        qualifier: &QualifierQuery<Self::Qualifier>,
        stat: &S
    ) -> Option<S::Data> {
        StatQuerier::query(self, entity, qualifier, stat)
    }

    fn system<S: Stat>(
        input: In<(Entity, QualifierQuery<Self::Qualifier>, S)>,
        this: StaticSystemParam<Self>,
    ) -> Option<S::Data> {
        let (entity, qualifier, stat) = input.0;
        this.query(entity, &qualifier, &stat)
    }

    fn as_boxed_readonly_system<S: Stat>() -> Box<dyn ReadOnlySystem<In=(Entity, QualifierQuery<Self::Qualifier>, S), Out=Option<S::Data>>> {
        use bevy_ecs::system::IntoSystem;
        Box::new(IntoSystem::into_system(Self::system::<S>))
    }
}

#[allow(unused)]
pub use crate::stream::{ExternalStream, IntrinsicStream};
/// Construct a [`StatQuerier`] type alias from arguments.
/// The result can be used as a [`SystemParam`].
///
/// # Syntax
///
/// ```
/// querier!(pub MyQuerier {
///     qualifier: MyQualifier,
///     intrinsic: {
///         Allegiance,
///         Position
///     },
///     external: {
///         MyStat,
///         MyWeapon,
///         MyBuff,
///     }
/// });
/// ```
/// 
/// * qualifier: implements [`QualifierFlag`]
/// * intrinsic: { implements [`IntrinsicStream`], .. }
/// * components: { implements [`ExternalStream`], .. }
///
/// This generates
///
/// ```
/// pub type MyQuerier = Querier<..>
/// ```
#[macro_export]
macro_rules! querier {
    (@hints $qualifier: ident: $ty: ty, $($name: ident: $ignored: tt),* $(,)?) => {
        #[automatically_derived]
        #[allow(unused, clippy::needless_update)]
        const _: () = {
            || {
                $crate::hints::ImplQuerier {
                    $qualifier: ::std::default::Default::default(),
                    $($name: ::std::default::Default::default(),)*
                    ..::std::default::Default::default()
                };
            };
        };
    };
    (@join $qualifier: ty) => {
        ()
    };
    (@join $qualifier: ty, $first: ty $(,$ty: ty)*) => {
        ($crate::ChildStatParam<'static, 'static,
            $first,
            $qualifier,
        >, $crate::querier!(@join $qualifier $(,$ty)*))
    };
    (
        $vis: vis $name: ident {
            $($tt: tt)*
        }
    ) => {
        $crate::querier!(@main $vis $name {$($tt)*});
        $crate::querier!(@hints $($tt)*);
    };
    (
        @main
        $vis: vis $name: ident {
            qualifier: $qualifier: ty,
            intrinsic: {
                $($intrinsics: ty),* $(,)?
            },
            external: {
                $($ty: ty),* $(,)?
            } $(,)?
        }
    ) => {
        $vis type $name<'w, 's> = $crate::StatQuerier<'w, 's,
            $qualifier,
            $crate::querier!(@join $qualifier $(,$intrinsics)*),
            $crate::querier!(@join $qualifier $(,$ty)*)
        >;
    };
}

#[allow(unused)]
#[doc(hidden)]
pub mod hints {
    #[doc(hidden)]
    #[allow(nonstandard_style)]
    #[derive(Default)]
    pub struct impl_QualifierFlags;

    #[doc(hidden)]
    #[allow(nonstandard_style)]
    #[derive(Default)]
    pub struct impl_IntrinsicStream;

    #[doc(hidden)]
    #[allow(nonstandard_style)]
    #[derive(Default)]
    pub struct impl_ExternalStream;

    #[doc(hidden)]
    #[allow(nonstandard_style)]
    #[derive(Default)]
    pub struct List<T>(T);

    use crate::{QualifierFlag, ExternalStream, IntrinsicStream};
    use bevy_ecs::query::QueryData;

    #[doc(hidden)]
    #[derive(Default)]
    pub struct ImplQuerier {
        /// The qualifier of the type, should be a [`QualifierFlag`](crate::QualifierFlag)
        pub qualifier: impl_QualifierFlags,
        /// A list of [`IntrinsicStream`] types to pull data from a `StatEntity`'s components.
        pub intrinsic: List<impl_IntrinsicStream>,
        /// A list of [`ExternalStream`] types to pull data from a `StatEntity`'s children.
        pub external: List<impl_ExternalStream>,
    }
}
