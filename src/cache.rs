use crate::{Attribute, Qualifier, QualifierQuery, ShareableAny, StatUid};
use bevy_ecs::entity::Entity;

/// An internally mutable user defined cache for stats.
#[allow(unused_variables)]
pub trait StatCache<Q: Qualifier> {
    /// Clear cached stats.
    fn clear(&self);
    /// Obtain cached stat if exists.
    fn get_cached_stat(
        &self,
        entity: Entity,
        qualifier: &QualifierQuery<Q>,
        stat: StatUid,
    ) -> Option<Box<dyn ShareableAny>> {
        None
    }
    /// Obtain cached relation if exists.
    fn get_cached_relation(
        &self,
        from: Entity,
        to: Entity,
        qualifier: &QualifierQuery<Q>,
        stat: StatUid,
    ) -> Option<Box<dyn ShareableAny>> {
        None
    }
    /// Obtain cached relation if exists.
    fn get_cached_attribute(&self, entity: Entity, attribute: &Attribute) -> Option<bool> {
        None
    }
    /// Cache a stat.
    ///
    /// Does not to be able to cache everything, can silently fail if not needed or if the value is not cacheable.
    fn cache_stat(
        &self,
        entity: Entity,
        qualifier: &QualifierQuery<Q>,
        stat: StatUid,
        value: &dyn ShareableAny,
    ) {
    }
    /// Cache a stat relation.
    ///
    /// Does not to be able to cache everything, can silently fail if not needed or if the value is not cacheable.
    fn cache_relation(
        &self,
        from: Entity,
        to: Entity,
        qualifier: &QualifierQuery<Q>,
        stat: StatUid,
        value: &dyn ShareableAny,
    ) {
    }
    /// Cache an attribute.
    ///
    /// Does not to be able to cache everything, can silently fail if not needed or if the value is not cacheable.
    fn cache_attribute(&self, entity: Entity, attribute: Attribute, is_true: bool) {}
}

impl<Q: Qualifier> StatCache<Q> for () {
    fn clear(&self) {}
}

impl<Q: Qualifier, A: StatCache<Q>, B: StatCache<Q>> StatCache<Q> for (A, B) {
    fn clear(&self) {
        self.0.clear();
        self.1.clear();
    }

    fn get_cached_stat(
        &self,
        entity: Entity,
        qualifier: &QualifierQuery<Q>,
        stat: StatUid,
    ) -> Option<Box<dyn ShareableAny>> {
        if let Some(result) = self.1.get_cached_stat(entity, qualifier, stat) {
            return Some(result);
        }
        if let Some(result) = self.0.get_cached_stat(entity, qualifier, stat) {
            return Some(result);
        }
        None
    }

    fn get_cached_relation(
        &self,
        from: Entity,
        to: Entity,
        qualifier: &QualifierQuery<Q>,
        stat: StatUid,
    ) -> Option<Box<dyn ShareableAny>> {
        if let Some(result) = self.1.get_cached_relation(from, to, qualifier, stat) {
            return Some(result);
        }
        if let Some(result) = self.0.get_cached_relation(from, to, qualifier, stat) {
            return Some(result);
        }
        None
    }

    fn get_cached_attribute(&self, entity: Entity, attribute: &Attribute) -> Option<bool> {
        if let Some(result) = self.1.get_cached_attribute(entity, attribute) {
            return Some(result);
        }
        if let Some(result) = self.0.get_cached_attribute(entity, attribute) {
            return Some(result);
        }
        None
    }

    fn cache_stat(
        &self,
        entity: Entity,
        qualifier: &QualifierQuery<Q>,
        stat: StatUid,
        value: &dyn ShareableAny,
    ) {
        self.1.cache_stat(entity, qualifier, stat, value);
        self.0.cache_stat(entity, qualifier, stat, value);
    }

    fn cache_relation(
        &self,
        from: Entity,
        to: Entity,
        qualifier: &QualifierQuery<Q>,
        stat: StatUid,
        value: &dyn ShareableAny,
    ) {
        self.1.cache_relation(from, to, qualifier, stat, value);
        self.0.cache_relation(from, to, qualifier, stat, value);
    }

    fn cache_attribute(&self, entity: Entity, attribute: Attribute, is_true: bool) {
        self.1.cache_attribute(entity, attribute, is_true);
        self.0.cache_attribute(entity, attribute, is_true);
    }
}
