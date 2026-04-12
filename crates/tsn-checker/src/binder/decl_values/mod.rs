mod declarations;
mod extensions;
mod namespace_struct;
mod object_members;
mod patterns_sum;

use tsn_core::ast::TypeNode;

pub(super) fn type_node_to_name(node: &TypeNode) -> String {
    use crate::types::well_known;
    use tsn_core::TypeKind;
    match &node.kind {
        TypeKind::Int => well_known::INT.to_owned(),
        TypeKind::Float => well_known::FLOAT.to_owned(),
        TypeKind::Str => well_known::STR.to_owned(),
        TypeKind::Bool => well_known::BOOL.to_owned(),
        TypeKind::Char => well_known::CHAR.to_owned(),
        TypeKind::Named(n, _) => n.clone(),
        TypeKind::Generic(n, _, _) => n.clone(),
        TypeKind::Array(_) => well_known::ARRAY.to_owned(),
        _ => well_known::DYNAMIC.to_owned(),
    }
}
