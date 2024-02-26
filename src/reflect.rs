use crate::traits::DynStat;

impl bevy_reflect::TypePath for Box<dyn DynStat> {
    fn type_path() -> &'static str {
        std::any::type_name::<Self>()
    }

    fn short_type_path() -> &'static str {
        "Box<dyn Stat>"
    }
}

impl bevy_reflect::FromReflect for Box<dyn DynStat> {
    fn from_reflect(reflect: &dyn bevy_reflect::Reflect) -> Option<Self> {
        reflect
    }
}

impl bevy_reflect::Reflect for Box<dyn DynStat> {
    fn get_represented_type_info(&self) -> Option<&'static bevy_reflect::TypeInfo> {
        todo!()
    }

    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn into_reflect(self: Box<Self>) -> Box<dyn bevy_reflect::Reflect> {
        self
    }

    fn as_reflect(&self) -> &dyn bevy_reflect::Reflect {
        self
    }

    fn as_reflect_mut(&mut self) -> &mut dyn bevy_reflect::Reflect {
        self
    }

    fn apply(&mut self, value: &dyn bevy_reflect::Reflect) {
        todo!()
    }

    fn set(&mut self, value: Box<dyn bevy_reflect::Reflect>) -> Result<(), Box<dyn bevy_reflect::Reflect>> {
        *self = *value.downcast::<Box<dyn DynStat>>()?;
        Ok(())
    }

    fn reflect_ref(&self) -> bevy_reflect::ReflectRef {
        todo!()
    }

    fn reflect_mut(&mut self) -> bevy_reflect::ReflectMut {
        todo!()
    }

    fn reflect_owned(self: Box<Self>) -> bevy_reflect::ReflectOwned {
        todo!()
    }

    fn clone_value(&self) -> Box<dyn bevy_reflect::Reflect> {
        todo!()
    }
}
