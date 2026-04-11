use std::sync::Arc;
use tsn_types::{value::{new_object, ObjData}, Value};
use tsn_types::NativeFn;

pub fn path_normalize(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let s = args.first().map(|v| v.to_string()).unwrap_or_default();
    let p = std::path::Path::new(&s);
    let mut comps: Vec<&str> = vec![];
    for comp in p.components() {
        use std::path::Component;
        match comp {
            Component::ParentDir => { comps.pop(); }
            Component::CurDir => {}
            Component::Normal(s) => comps.push(s.to_str().unwrap_or("")),
            Component::RootDir => comps.push(""),
            Component::Prefix(p) => comps.push(p.as_os_str().to_str().unwrap_or("")),
        }
    }
    Ok(Value::Str(Arc::from(comps.join(std::path::MAIN_SEPARATOR_STR))))
}

pub fn path_join(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let mut p = std::path::PathBuf::new();
    for arg in args {
        p.push(arg.to_string());
    }
    Ok(Value::Str(Arc::from(p.to_string_lossy())))
}

pub fn path_dirname(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let s = args.first().map(|v| v.to_string()).unwrap_or_default();
    let dir = std::path::Path::new(&s).parent()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| ".".to_owned());
    Ok(Value::Str(Arc::from(dir)))
}

pub fn path_basename(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let s = args.first().map(|v| v.to_string()).unwrap_or_default();
    let name = std::path::Path::new(&s).file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok(Value::Str(Arc::from(name)))
}

pub fn path_extname(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let s = args.first().map(|v| v.to_string()).unwrap_or_default();
    let ext = std::path::Path::new(&s).extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    Ok(Value::Str(Arc::from(ext)))
}

pub fn path_is_absolute(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let s = args.first().map(|v| v.to_string()).unwrap_or_default();
    Ok(Value::Bool(std::path::Path::new(&s).is_absolute()))
}

pub fn build() -> Value {
    let mut ns = ObjData::new();
    ns.set_field(Arc::from("normalize"),  Value::NativeFn(Box::new((path_normalize  as NativeFn, "normalize"))));
    ns.set_field(Arc::from("join"),       Value::NativeFn(Box::new((path_join       as NativeFn, "join"))));
    ns.set_field(Arc::from("dirname"),    Value::NativeFn(Box::new((path_dirname    as NativeFn, "dirname"))));
    ns.set_field(Arc::from("basename"),   Value::NativeFn(Box::new((path_basename   as NativeFn, "basename"))));
    ns.set_field(Arc::from("extname"),    Value::NativeFn(Box::new((path_extname    as NativeFn, "extname"))));
    ns.set_field(Arc::from("isAbsolute"), Value::NativeFn(Box::new((path_is_absolute as NativeFn, "isAbsolute"))));

    let mut exports = ObjData::new();
    exports.set_field(Arc::from("Path"), new_object(ns));
    new_object(exports)
}
