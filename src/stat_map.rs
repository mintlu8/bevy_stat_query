use crate::operations::StatOperation;
use crate::qualifier::QualifierKey;
use crate::stat::StatValuePair;
use crate::{
    QualifierQuery, Querier, Shareable, ShareableAny, Stat, StatStream, StatUid, StatValue,
};
use bevy_ecs::component::Component;
use bevy_ecs::entity::Entity;
use bevy_ecs::reflect::ReflectComponent;
use bevy_reflect::Reflect;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// Either a stat of a enum type of multiple stats.
pub trait StatDispatch: Shareable {
    type Value: Shareable;

    fn get_uid(&self) -> StatUid;
    fn join_to(&self, value: &Self::Value, into: &mut dyn ShareableAny);
}

impl<S: Stat> StatDispatch for S {
    type Value = S::Value;

    fn get_uid(&self) -> StatUid {
        S::as_uid(self)
    }

    fn join_to(&self, value: &Self::Value, into: &mut dyn ShareableAny) {
        if let Some(item) = into.downcast_mut::<Self::Value>() {
            item.join_by_ref(value);
        }
    }
}

/// Either a stat of a enum type of multiple stats.
pub trait StatDispatchTo<T: Stat>: StatDispatch {
    fn from_stat(stat: T) -> Self;
    fn from_value(value: T::Value) -> Self::Value;
    fn get_value(value: &Self::Value) -> Option<&T::Value>;
    fn get_value_mut(value: &mut Self::Value) -> Option<&mut T::Value>;
    fn get_value_owned(value: Self::Value) -> Option<T::Value>;
    fn try_join_to(&self, value: &Self::Value, into: &mut T::Value);
}

impl<S: Stat> StatDispatchTo<S> for S {
    fn from_stat(stat: S) -> Self {
        stat
    }

    fn from_value(value: <S as Stat>::Value) -> Self::Value {
        value
    }

    fn get_value(value: &Self::Value) -> Option<&<S as Stat>::Value> {
        Some(value)
    }

    fn get_value_mut(value: &mut Self::Value) -> Option<&mut <S as Stat>::Value> {
        Some(value)
    }

    fn get_value_owned(value: Self::Value) -> Option<<S as Stat>::Value> {
        Some(value)
    }

    fn try_join_to(&self, value: &Self::Value, into: &mut <S as Stat>::Value) {
        into.join_by_ref(value);
    }
}

#[derive(Debug, Clone, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub(crate) struct StatMapEntry<Q: QualifierKey, S: StatDispatch> {
    stat: S,
    qualifier: Q,
    value: S::Value,
}

/// A storage component of qualified stats.
///
/// # Performance
///
/// The type is implemented as a sorted VecMap and prioritizes querying,
/// not optimized for rapid insertion or removal.
#[derive(Debug, Clone, Component, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(bound(serialize = "Q: Serialize, S: Serialize, S::Value: Serialize"))
)]
#[cfg_attr(
    feature = "serde",
    serde(bound(
        deserialize = "Q: Deserialize<'de>, S: Deserialize<'de>, S::Value: Deserialize<'de>"
    ))
)]
#[reflect(Component)]
pub struct StatMapBase<Q: QualifierKey, S: StatDispatch> {
    #[reflect(ignore)]
    inner: Vec<StatMapEntry<Q, S>>,
}

impl<Q: QualifierKey, S: StatDispatch> Default for StatMapBase<Q, S> {
    fn default() -> Self {
        StatMapBase { inner: Vec::new() }
    }
}

impl<Q: QualifierKey, T: StatDispatch> StatMapBase<Q, T> {
    pub const fn new() -> Self {
        Self { inner: Vec::new() }
    }

    /// Drops all items in the map.
    pub fn clear(&mut self) {
        self.inner.clear()
    }

    /// Returns true if the map contains no elements.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns the number of elements in the map.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Performs a binary search for a value.
    fn binary_search(&self, qualifier: &Q, stat: StatUid) -> Result<usize, usize> {
        self.inner.binary_search_by(
            |StatMapEntry {
                 stat: s,
                 qualifier: q,
                 ..
             }| { (s.get_uid(), q).cmp(&(stat, qualifier)) },
        )
    }

    /// Performs a binary search for a value.
    fn find_slice(&self, stat: StatUid) -> &[StatMapEntry<Q, T>] {
        let first = self.inner.partition_point(|x| x.stat.get_uid() < stat);
        let last = self.inner.partition_point(|x| x.stat.get_uid() <= stat);
        self.inner.get(first..last).unwrap_or(&[])
    }

    /// Inserts a [`Stat::Value`] in its component form.
    pub fn insert<S: Stat>(&mut self, qualifier: Q, stat: S, value: S::Value)
    where
        T: StatDispatchTo<S>,
    {
        match self.binary_search(&qualifier, stat.get_uid()) {
            Ok(at) => self.inner[at].value = T::from_value(value),
            Err(at) => self.inner.insert(
                at,
                StatMapEntry {
                    stat: T::from_stat(stat),
                    qualifier,
                    value: T::from_value(value),
                },
            ),
        };
    }

    /// Inserts a [`Stat::Value`] in its evaluated form.
    pub fn insert_base<S: Stat>(
        &mut self,
        qualifier: Q,
        stat: S,
        base: <S::Value as StatValue>::Base,
    ) where
        T: StatDispatchTo<S>,
    {
        self.insert(qualifier, stat, S::Value::from_base(base));
    }

    /// Obtains a [`Stat::Value`].
    pub fn get<S: Stat>(&self, qualifier: &Q, stat: &S) -> Option<&S::Value>
    where
        T: StatDispatchTo<S>,
    {
        match self.binary_search(qualifier, stat.as_uid()) {
            Ok(at) => self
                .inner
                .get(at)
                .and_then(|entry| T::get_value(&entry.value)),
            Err(_) => None,
        }
    }

    /// Obtains a mutable [`Stat::Value`].
    pub fn get_mut<S: Stat>(&mut self, qualifier: &Q, stat: &S) -> Option<&mut S::Value>
    where
        T: StatDispatchTo<S>,
    {
        match self.binary_search(qualifier, stat.as_uid()) {
            Ok(at) => self
                .inner
                .get_mut(at)
                .and_then(|entry| T::get_value_mut(&mut entry.value)),
            Err(_) => None,
        }
    }

    /// Removes and obtains a [`Stat::Value`].
    pub fn remove<S: Stat>(&mut self, qualifier: &Q, stat: &S) -> Option<S::Value>
    where
        T: StatDispatchTo<S>,
    {
        match self.binary_search(qualifier, stat.as_uid()) {
            Ok(at) => T::get_value_owned(self.inner.remove(at).value),
            Err(_) => None,
        }
    }

    /// Obtains a [`Stat::Value`] in its evaluated form.
    pub fn get_evaled<S: Stat>(
        &self,
        qualifier: &Q,
        stat: &S,
    ) -> Option<<S::Value as StatValue>::Out>
    where
        T: StatDispatchTo<S>,
    {
        self.get(qualifier, stat).map(|x| x.eval())
    }

    /// Create or modify a stat via a [`StatOperation`].
    ///
    /// Create a [`Default`] stat if non-existent.
    pub fn modify<S: Stat>(&mut self, qualifier: Q, stat: S, op: StatOperation<S::Value>)
    where
        T: StatDispatchTo<S>,
    {
        match self.binary_search(&qualifier, stat.get_uid()) {
            Ok(at) => {
                if let Some(value) = self
                    .inner
                    .get_mut(at)
                    .and_then(|entry| T::get_value_mut(&mut entry.value))
                {
                    op.write_to(value);
                }
            }
            Err(at) => {
                self.inner.insert(
                    at,
                    StatMapEntry {
                        stat: T::from_stat(stat),
                        qualifier,
                        value: T::from_value(op.into_value()),
                    },
                );
            }
        }
    }

    /// Create or modify a stat via a closure.
    ///
    /// Create a [`Default`] stat if non-existent.
    pub fn modify_with<S: Stat>(&mut self, qualifier: Q, stat: S, f: impl FnOnce(&mut S::Value))
    where
        T: StatDispatchTo<S>,
    {
        match self.binary_search(&qualifier, stat.get_uid()) {
            Ok(at) => {
                if let Some(value) = self
                    .inner
                    .get_mut(at)
                    .and_then(|entry| T::get_value_mut(&mut entry.value))
                {
                    f(value)
                }
            }
            Err(at) => {
                let mut value = S::Value::default();
                f(&mut value);
                self.inner.insert(
                    at,
                    StatMapEntry {
                        stat: T::from_stat(stat),
                        qualifier,
                        value: T::from_value(value),
                    },
                );
            }
        }
    }

    pub fn query_stat<S: Stat>(
        &self,
        qualifier: &QualifierQuery<Q::Qualifier>,
        stat: &S,
    ) -> S::Value
    where
        T: StatDispatchTo<S>,
    {
        let mut result = S::Value::default();
        for entry in self.find_slice(stat.as_uid()) {
            if entry.qualifier.qualify_query(qualifier) {
                entry.stat.try_join_to(&entry.value, &mut result);
            }
        }
        result
    }

    pub fn eval_stat<S: Stat>(
        &self,
        qualifier: &QualifierQuery<Q::Qualifier>,
        stat: &S,
    ) -> <S::Value as StatValue>::Out
    where
        T: StatDispatchTo<S>,
    {
        self.query_stat(qualifier, stat).eval()
    }
}

impl<Q: QualifierKey, T: StatDispatch> StatStream for StatMapBase<Q, T> {
    type Qualifier = Q::Qualifier;

    fn stream_stat(
        &self,
        _: Entity,
        qualifier: &crate::QualifierQuery<Q::Qualifier>,
        stat_value: &mut StatValuePair,
        _: Querier<Q::Qualifier>,
    ) {
        for entry in self.find_slice(stat_value.uid()) {
            if entry.qualifier.qualify_query(qualifier) {
                entry.stat.join_to(&entry.value, stat_value.value);
            }
        }
    }
}
