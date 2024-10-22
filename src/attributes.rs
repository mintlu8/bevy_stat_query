use bevy_ecs::{component::Component, entity::Entity};
use ecow::EcoString;
use rustc_hash::FxHashSet;

use crate::{ComponentStream, QualifierFlag};

/// A component containing string attributes.
#[derive(Debug, Clone, Component, Default)]
pub struct AttributeMap(FxHashSet<EcoString>);

impl AttributeMap {
    pub fn new() -> Self {
        AttributeMap(FxHashSet::default())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn with(mut self, attribute: &str) -> Self {
        self.0.insert(attribute.into());
        self
    }

    pub fn insert(&mut self, attribute: &str) {
        self.0.insert(attribute.into());
    }

    pub fn remove(&mut self, attribute: &str) {
        self.0.remove(attribute);
    }

    pub fn contains(&self, attribute: &str) -> bool {
        self.0.contains(attribute)
    }
}

impl<Q: QualifierFlag> ComponentStream<Q> for &AttributeMap {
    type Cx = ();

    fn has_attribute(
        _: Entity,
        _: &<Self::Cx as bevy_ecs::system::SystemParam>::Item<'_, '_>,
        component: <Self::ReadOnly as bevy_ecs::query::WorldQuery>::Item<'_>,
        attribute: &str,
        _: crate::Querier<Q>,
    ) -> bool {
        component.contains(attribute)
    }
}
