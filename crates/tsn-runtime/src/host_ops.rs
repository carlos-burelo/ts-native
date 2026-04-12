pub type HostOpFn = tsn_types::NativeFn;

#[derive(Clone, Copy)]
pub struct HostOp {
    pub name: &'static str,
    pub func: HostOpFn,
}
