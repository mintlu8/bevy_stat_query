use std::slice::Iter;
use bevy_ecs::{entity::Entity, query::{QueryData, With}, system::{In, Query, Res, StaticSystemParam, SystemParam}};
use bevy_hierarchy::Children;
use rustc_hash::FxHashMap;
use crate::{sealed::Sealed, Data, DefaultStatLogic, DynStat, QualifierFlag, QualifierQuery, Stat, StatDefaults, StatMapInner, StatParam};
use crate::{FromIntrinsics, StatQuerier, StatComponents};
use crate::{StatCache, StatEntity};

#[derive(SystemParam)]
struct QuerierInner<'w, 's,
    Qualifier: QualifierFlag,
    Intrinsic: QueryData + 'static,
    Components: StatParam<Qualifier, Intrinsic> + 'static
> {
    defaults: Res<'w, StatDefaults>,
    units: Query<'w, 's, (Option<&'static StatMapInner<Qualifier>>, Option<&'static Children>), With<StatEntity>>,
    distance: Query<'w, 's, Intrinsic>,
    singleton: Query<'w, 's, &'static Children, With<DefaultStatLogic<Qualifier>>>,
    items: StaticSystemParam<'w, 's, Components>,
}

/// A [`SystemParam`] that allows the user to query stats by [`Entity`].
///
/// This requires immutable access to many components and entities,
/// assume all relavent components are not mutably accessble within the same system.
///
/// Alternatively this can be ran with world access with [`query_stat`](crate::StatExtension::query_stat).
#[derive(SystemParam)]
pub struct Querier<'w, 's,
    Qualifier: QualifierFlag,
    Intrinsic: QueryData + 'static,
    Components: StatParam<Qualifier, Intrinsic> + 'static
> {
    querier: QuerierInner<'w, 's, Qualifier, Intrinsic, Components>,
    cache: Query<'w, 's, &'static mut StatCache<Qualifier>, With<StatEntity>>,
}

struct QueryStack<'w, 's, 'w2, 's2, 't, Q: QualifierFlag, D: QueryData + 'static, A: StatParam<Q, D> + 'static> {
    world_cache: &'t mut Query<'w2, 's2, &'static mut StatCache<Q>, With<StatEntity>>,
    current_cache: FxHashMap<(Entity, QualifierQuery<Q>, Box<dyn DynStat>), Box<dyn Data>>,
    querier: &'t QuerierInner<'w, 's, Q, D, A>,
    stack: Vec<(QualifierQuery<Q>, Box<dyn DynStat>, Entity)>,
    default_logic: Iter<'t, Entity>,
}

impl<Q: QualifierFlag, D: QueryData + 'static, A: StatParam<Q, D> + 'static> Sealed
        for QueryStack<'_, '_, '_, '_, '_, Q, D, A> {}

impl<Q: QualifierFlag, D: QueryData + 'static, A: StatParam<Q, D> + 'static> StatQuerier<Q, D>
        for QueryStack<'_, '_, '_, '_, '_, Q, D, A> {
    fn query<S: Stat>(&mut self, qualifier: &QualifierQuery<Q>, stat: &S) -> Option<S::Data> {
        let entity = self.stack.last().expect("Must call query_other on the first call.").2;
        self.query_other(entity, qualifier, stat)
    }

    fn query_other<S: Stat>(&mut self, entity: Entity, qualifier: &crate::QualifierQuery<Q>, stat: &S) -> Option<S::Data> {
        if self.stack.iter().any(|(q, s, e)| q == qualifier && s == stat && e == &entity) {
            panic!("A cycle detected in stat dependencies when querying ({:?}, {:?}, {:?}). \
                This query cannot be completed {:?}.",
                qualifier, stat, entity, self.stack
            )
        };
        self.stack.push((qualifier.clone(), Box::new(stat.clone()), entity));
        let Ok((stat_map, children)) = self.querier.units.get(entity) else { return None; };

        if let Some(cached) = match self.world_cache.get(entity){
            Ok(cache) => cache.try_get_cached(qualifier, stat),
            Err(_) => None,
        } {
            return Some(cached)
        }
        let queried = match children {
            Some(children) => children.iter(),
            None => [].iter(),
        }.chain(self.default_logic.clone());
        let mut stat_value = self.querier.defaults.get(stat);
        if let Some(stat_map) = stat_map {
            for (q, v) in stat_map.iter_stat(stat) {
                if q.qualifies_as(qualifier) {
                    <S::Data as StatComponents>::from_out(v.clone()).write_to(&mut stat_value);
                }
            }
        }
        A::stream(&*self.querier.items, queried, &mut stat_value, qualifier, stat, self);
        let Some(_) = self.stack.pop() else {panic!("Stack mismatch.")};
        if let Ok(mut cache) = self.world_cache.get_mut(entity) {
            cache.cache(qualifier.clone(), stat.clone(), stat_value.clone());
        } else {
            self.current_cache.insert(
                (entity, qualifier.clone(), Box::new(stat.clone())),
                stat_value.dyn_clone()
            );
        }
        Some(stat_value)
    }

    fn query_intrinsic<S: Stat + FromIntrinsics<IntrisicQuery = D>>(&mut self, stat: &S) -> Option<S::Data> {
        let curr = self.stack.last().expect("Must call query_other on the first call.").2;
        let Ok(ctx) = self.querier.distance.get(curr) else { return None; };
        Some(stat.from_intrinsic(&ctx))
    }

    fn query_distance<S: Stat + FromIntrinsics<IntrisicQuery = D>>(&mut self, entity: Entity, stat: &S) -> Option<S::Data> {
        let curr = self.stack.last().expect("Must call query_other on first call.").2;
        let Ok(ctx1) = self.querier.distance.get(curr) else { return None; };
        let Ok(ctx2) = self.querier.distance.get(entity) else { return None; };
        Some(stat.from_distance(&ctx1, &ctx2))
    }
}

impl<'w, 's, Q: QualifierFlag, D: QueryData + 'static, A: StatParam<Q, D> + 'static> QuerierInner<'w, 's, Q, D, A> {
    fn as_query_stack<'w2, 's2, 't>(&'t self, cache: &'t mut Query<'w2, 's2, &'static mut StatCache<Q>, With<StatEntity>>) -> QueryStack<'w, 's, 'w2, 's2, 't, Q, D, A> {
        QueryStack {
            querier: self,
            stack: Vec::new(),
            world_cache: cache,
            default_logic: match self.singleton.get_single() {
                Ok(children) => children.iter(),
                _ => [].iter()
            },
            current_cache: FxHashMap::default(),
        }
    }

    pub fn query<S: Stat>(&self,
        cache: &mut Query<'_, '_, &'static mut StatCache<Q>, With<StatEntity>>,
        entity: Entity,
        qualifier: &QualifierQuery<Q>,
        stat: &S
    ) -> Option<S::Data> {
        self.as_query_stack(cache).query_other(entity, qualifier, stat)
    }
}


impl<'w, 's, Q: QualifierFlag, D: QueryData + 'static, A: StatParam<Q, D> + 'static> Querier<'w, 's, Q, D, A> {
    pub fn query<S: Stat>(&mut self,
        entity: Entity,
        qualifier: &QualifierQuery<Q>,
        stat: &S
    ) -> Option<S::Data> {
        self.querier.as_query_stack(&mut self.cache).query_other(entity, qualifier, stat)
    }

    pub fn query_eval<S: Stat>(&mut self,
        entity: Entity,
        qualifier: &QualifierQuery<Q>,
        stat: &S
    ) -> Option<<S::Data as StatComponents>::Out> {
        self.query(entity, qualifier, stat).map(|x| x.eval())
    }
}

// Type erased [`Querier`] with no generics.
pub trait ErasedQuerier: SystemParam + 'static {
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
}

impl<Q: QualifierFlag, D: QueryData + 'static, A: StatParam<Q, D> + 'static> ErasedQuerier for Querier<'static, 'static, Q, D, A> {
    type Qualifier = Q;

    fn query<S: Stat>(&mut self,
        entity: Entity,
        qualifier: &QualifierQuery<Self::Qualifier>,
        stat: &S
    ) -> Option<S::Data> {
        self.query(entity, qualifier, stat)
    }

    fn system<S: Stat>(
        input: In<(Entity, QualifierQuery<Self::Qualifier>, S)>,
        mut this: StaticSystemParam<Self>,
    ) -> Option<S::Data>{
        let (entity, qualifier, stat) = input.0;
        this.query(entity, &qualifier, &stat)
    }
}

/// Construct a [`Querier`] type alias from arguments.
/// The result can be used as a [`SystemParam`].
///
/// # Syntax
///
/// ```
/// querier!(pub MyQuerier {
///     qualifier: MyQualifier,
///     intrinsic: (&'static UnitPosition, &'static UnitInfo),
///     components: {
///         MyStat,
///         MyWeapon,
///         MyBuff,
///     }
/// });
/// ```
///
/// This generates
///
/// ```
/// pub type MyQuerier = Querier<..>
/// ```
#[macro_export]
macro_rules! querier {
    (@hints $($name:ident: $ignored: expr),* $(,)?) => {
        #[automatically_derived]
        #[allow(unused, clippy::needless_update)]
        const _: () = {
            || {
                $crate::hints::ImplQuerier {
                    $($name: ::std::default::Default::default(),)*
                    ..::std::default::Default::default()
                };
            };
        };
    };
    (@join $qualifier: ty, $ctx: ty) => {
        ()
    };
    (@join $qualifier: ty, $ctx: ty, $first: ty $(,$ty: ty)*) => {
        ($crate::ChildStatParam<'static, 'static,
            $first,
            $qualifier,
            $ctx
        >, $crate::querier!(@join $qualifier, $ctx $(,$ty)*))
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
            intrinsic: $ctx: ty,
            components: {
                $($ty: ty),*
            } $(,)?
        }
    ) => {
        $vis type $name<'w, 's> = $crate::Querier<'w, 's,
            $qualifier,
            $ctx,
            $crate::querier!(@join $qualifier, $ctx $(,$ty)*)
        >;
    };
    (
        @main
        $vis: vis $name: ident {
            qualifier: $qualifier: ty,
            components: {
                $($ty: ty),*
            } $(,)?
        }
    ) => {
        $vis type $name<'w, 's> = $crate::Querier<'w, 's,
            $qualifier,
            (),
            $crate::querier!(@join $($ty),*)
        >;
    };
}

querier!(pub StatQuerier2 {
   qualifier: u32,
   intrinsic: (),
   components: {}
});

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
    pub struct impl_QueryData;

    #[doc(hidden)]
    #[allow(nonstandard_style)]
    #[derive(Default)]
    pub struct impl_StatStream;

    #[doc(hidden)]
    #[allow(nonstandard_style)]
    #[derive(Default)]
    pub struct List<T>(T);

    use crate::{QualifierFlag, StatStream};
    use bevy_ecs::query::QueryData;

    #[doc(hidden)]
    #[derive(Default)]
    pub struct ImplQuerier {
        /// The qualifier of the type, should be a [`QualifierFlag`](crate::QualifierFlag)
        pub qualifier: impl_QualifierFlags,
        /// A [`QueryData`] that is used to obtain intrinsic information about a stat's owner.
        pub intrinsic: impl_QueryData,
        /// A list of [`StatStream`] types to pull data from.
        pub components: List<impl_StatStream>,
    }
}
