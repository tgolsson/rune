//! The `std::vec` module.

use crate::{
    self as runestick, Any, ContextError, FromValue, Module, Protocol, RangeLimits, Ref, Value,
    Vec, VmError, VmErrorKind, VmIntegerRepr,
};

/// Construct the `std::vec` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", &["vec"]);

    module.ty::<Vec>()?;

    module.function(&["Vec", "new"], Vec::new)?;
    module.inst_fn("clear", Vec::clear)?;
    module.inst_fn("clone", Vec::clone)?;
    module.inst_fn("extend", Vec::extend)?;
    module.inst_fn("get", vec_get)?;
    module.inst_fn("iter", Vec::into_iterator)?;
    module.inst_fn("len", Vec::len)?;
    module.inst_fn("pop", Vec::pop)?;
    module.inst_fn("push", Vec::push)?;
    module.inst_fn("remove", Vec::remove)?;

    module.inst_fn(Protocol::INTO_ITER, Vec::into_iterator)?;
    module.inst_fn(Protocol::INDEX_SET, Vec::set)?;
    module.inst_fn(Protocol::INDEX_GET, vec_get)?;

    // TODO: parameterize with generics.
    module.inst_fn("sort_int", sort_int)?;

    module.ty::<Slice>()?;

    module.inst_fn("len", Slice::len)?;
    module.inst_fn(Protocol::INDEX_GET, slice_get)?;
    module.inst_fn(Protocol::INDEX_SET, Slice::set)?;
    module.inst_fn(Protocol::STRING_DEBUG, Slice::string_debug)?;

    Ok(module)
}

/// Sort a vector of integers.
fn sort_int(vec: &mut Vec) {
    vec.sort_by(|a, b| match (a, b) {
        (Value::Integer(a), Value::Integer(b)) => a.cmp(&b),
        // NB: fall back to sorting by address.
        _ => (a as *const _ as usize).cmp(&(b as *const _ as usize)),
    });
}

fn vec_get(vec: Ref<Vec>, key: Value) -> Result<Value, VmError> {
    use crate::{to_value::ToValue, TypeOf};
    use std::convert::TryInto;
    match key {
        Value::Integer(int) => {
            let idx: usize = match int.try_into() {
                Ok(v) => v,
                Err(_) => {
                    return Err(VmError::from(VmErrorKind::ValueToIntegerCoercionError {
                        from: VmIntegerRepr::from(int),
                        to: "usize",
                    }))
                }
            };

            Ok(vec.get(idx).cloned().to_value()?)
        }
        Value::Range(r) => {
            let r = r.borrow_ref()?;
            let start = r
                .start
                .as_ref()
                .map(|v| <usize>::from_value(v.clone()))
                .transpose()?;
            let end = r
                .end
                .as_ref()
                .map(|v| <usize>::from_value(v.clone()))
                .transpose()?;
            Ok(Slice {
                slice: (Ref::map(vec, |value| match (start, end, r.limits) {
                    (Some(start), Some(end), RangeLimits::HalfOpen) => &value[start..end],
                    (Some(start), Some(end), RangeLimits::Closed) => &value[start..=end],
                    (Some(start), None, _) => &value[start..],
                    (None, Some(end), RangeLimits::HalfOpen) => &value[..end],
                    (None, Some(end), RangeLimits::Closed) => &value[..=end],
                    (None, None, _) => &value[..],
                })),
            }
            .to_value()?)
        }
        index => Err(VmError::from(VmErrorKind::UnsupportedIndexGet {
            target: Vec::type_info(),
            index: index.type_info()?,
        })),
    }
}

#[derive(Any)]
struct Slice {
    slice: Ref<[Value]>,
}

impl Slice {
    fn set(&self, key: usize, value: Value) -> Result<(), VmError> {
        if self.slice.len() <= key {
            return Err(VmError::from(VmErrorKind::OutOfRange {
                index: VmIntegerRepr::from(key),
                len: VmIntegerRepr::from(self.slice.len()),
            }));
        }

        let mut slice = self.slice.upgrade()?;
        slice[key] = value;
        drop(slice);
        Ok(())
    }

    #[inline]
    fn len(&self) -> usize {
        self.slice.len()
    }

    #[inline]
    fn string_debug(&self, s: &mut String) -> std::fmt::Result {
        use std::fmt::Write as _;
        write!(s, "{:?}", self.slice)
    }
}

fn slice_get(slice: &Slice, key: Value) -> Result<Value, VmError> {
    use crate::{to_value::ToValue, TypeOf};
    use std::convert::TryInto;
    match key {
        Value::Integer(int) => {
            let idx: usize = match int.try_into() {
                Ok(v) => v,
                Err(_) => {
                    return Err(VmError::from(VmErrorKind::ValueToIntegerCoercionError {
                        from: VmIntegerRepr::from(int),
                        to: "usize",
                    }))
                }
            };

            Ok(slice.slice.get(idx).cloned().to_value()?)
        }
        Value::Range(r) => {
            let r = r.borrow_ref()?;
            let start = r
                .start
                .as_ref()
                .map(|v| <usize>::from_value(v.clone()))
                .transpose()?;
            let end = r
                .end
                .as_ref()
                .map(|v| <usize>::from_value(v.clone()))
                .transpose()?;
            Ok(Slice {
                slice: (Ref::map(slice.slice.clone(), |value| match (start, end, r.limits) {
                    (Some(start), Some(end), RangeLimits::HalfOpen) => &value[start..end],
                    (Some(start), Some(end), RangeLimits::Closed) => &value[start..=end],
                    (Some(start), None, _) => &value[start..],
                    (None, Some(end), RangeLimits::HalfOpen) => &value[..end],
                    (None, Some(end), RangeLimits::Closed) => &value[..=end],
                    (None, None, _) => &value[..],
                })),
            }
            .to_value()?)
        }
        index => Err(VmError::from(VmErrorKind::UnsupportedIndexGet {
            target: Slice::type_info(),
            index: index.type_info()?,
        })),
    }
}
