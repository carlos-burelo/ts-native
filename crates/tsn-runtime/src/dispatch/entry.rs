use crate::host_ops::HostOpFn;
use tsn_core::intrinsic::IntrinsicId;

#[derive(Clone, Copy)]
pub(crate) struct DispatchEntry {
    pub(crate) id: u16,
    pub(crate) name: &'static str,
    pub(crate) func: HostOpFn,
}

pub(crate) const fn op(id: IntrinsicId, name: &'static str, func: HostOpFn) -> DispatchEntry {
    DispatchEntry {
        id: id as u16,
        name,
        func,
    }
}
