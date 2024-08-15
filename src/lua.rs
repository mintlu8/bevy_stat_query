use std::{any::TypeId, marker::PhantomData, ptr, sync::Arc};

use bevy_log::error;
use mlua::{Error, FromLua, IntoLua, Lua, UserData, UserDataMethods};
use num_rational::Ratio;
use num_traits::{Bounded, NumCast, Signed};

use crate::{num_traits::NumInteger, Fraction, Int, QualifierFlag, QualifierQuery, Querier, StatStream, StatValue};

/// Safety: safe since types are equal and static.
fn cast<A: 'static, B: 'static>(item: A) -> B {
    assert_eq!(TypeId::of::<A>(), TypeId::of::<B>());
    unsafe {ptr::read(ptr::from_ref(&item) as *const B)}
}

/// # How this works
/// 
/// We simply try a few common lua types to see if they match,
/// if so, the add, sub, etc functions will work.
/// 
/// Types supported are `bool`, `i32`, `u32`, `f32`, `String`, `Fraction<i32>`.
/// 
/// If you want to use an exotic type, add those methods there.
pub struct LuaStatValue<T: StatValue>(pub(crate) T);

impl<T: StatValue> UserData for LuaStatValue<T> {
    fn add_methods<'t, M: UserDataMethods<'t, Self>>(methods: &mut M) {
        macro_rules! tri {
            ($($T: ty),*) => {
                $(
                    if TypeId::of::<$T>() == TypeId::of::<T::Add>() {
                        methods.add_meta_method_mut("add", |_, this, other: $T| {
                            this.0.add(cast(other));
                            Ok(())
                        })
                    }
                    if TypeId::of::<$T>() == TypeId::of::<T::Mul>() {
                        methods.add_meta_method_mut("add", |_, this, other: $T| {
                            this.0.mul(cast(other));
                            Ok(())
                        })
                    }
                    if TypeId::of::<$T>() == TypeId::of::<T::Bounds>() {
                        methods.add_meta_method_mut("max", |_, this, other: $T| {
                            this.0.max(cast(other));
                            Ok(())
                        });
                        methods.add_meta_method_mut("min", |_, this, other: $T| {
                            this.0.min(cast(other));
                            Ok(())
                        });
                    }
                    if TypeId::of::<$T>() == TypeId::of::<T::Bit>() {
                        methods.add_meta_method_mut("or", |_, this, other: $T| {
                            this.0.or(cast(other));
                            Ok(())
                        });
                        methods.add_meta_method_mut("not", |_, this, other: $T| {
                            this.0.not(cast(other));
                            Ok(())
                        });
                    }
                )*
            };
        }
        tri!(i32, u32, f32, bool, String, Fraction<i32>);
    }
}

impl<'l, T: StatValue> FromLua<'l> for LuaStatValue<T> {
    fn from_lua(value: mlua::Value<'l>, _: &'l mlua::Lua) -> mlua::Result<Self> {
        value.as_userdata().ok_or(Error::UserDataTypeMismatch)?.take()
    }
}

impl<I: Int + NumInteger> UserData for Fraction<I> {}

impl<'lua, I: Int + NumInteger + Signed + Bounded + NumCast> FromLua<'lua> for Fraction<I> {
    #[allow(clippy::unnecessary_cast)]
    fn from_lua(value: mlua::Value<'lua>, _: &'lua mlua::Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::Integer(i) => Ok(Self::new(I::from_i64(i as i64), I::ONE)),
            mlua::Value::Number(f) => Ok(Fraction(Ratio::approximate_float(f as f64).ok_or(
                Error::FromLuaConversionError { 
                    from: "float", 
                    to: "fraction",
                    message: Some(format!("{f} is not a valid fraction.")) 
                })?
            )),
            mlua::Value::UserData(data) => data.take(),
            _ => Err(Error::UserDataTypeMismatch)
        }
    }
}

pub struct StatScript<Q> {
    script: Arc<dyn AsRef<str>>,
    p: PhantomData<Q>
}

impl<Q: QualifierFlag> UserData for QualifierQuery<Q> {
    
}

impl<'t, Q: QualifierFlag> UserData for Querier<'t, Q> {
    
}


impl<'lua, Q: QualifierFlag + IntoLua<'lua>> StatStream<Q> for StatScript<Q> {
    fn stream_stat(
        &self,
        qualifier: &crate::QualifierQuery<Q>,
        stat_value: &mut crate::StatValuePair,
        querier: crate::Querier<Q>,
    ) {
        let script = self.script.as_ref().as_ref();
        if let Err(e) = (|| {
            let lua = Lua::new();
            let globals = lua.globals();
            globals.set("qualifier", qualifier.clone())?;
            globals.set("stat", stat_value.stat.name())?;
            globals.set("value", stat_value.to_lua(&lua)?)?;
            lua.load(script).exec()?;
            stat_value.from_lua(&globals, "value")?;
            Ok::<(), Error>(())
        })() {
            error!("Lua stat script error: {e}.\nIn script:\n{script}");
        }
        
    }
}