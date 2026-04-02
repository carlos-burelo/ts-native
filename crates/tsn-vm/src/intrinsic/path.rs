use std::sync::Arc;
use tsn_types::value::Value;

pub fn path_normalize(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let s = args.first().map(|v| v.to_string()).unwrap_or_default();
    let p = std::path::Path::new(&s);
    let mut comps: Vec<&str> = vec![];
    for comp in p.components() {
        use std::path::Component;
        match comp {
            Component::ParentDir => {
                comps.pop();
            }
            Component::CurDir => {}
            Component::Normal(s) => comps.push(s.to_str().unwrap_or("")),
            Component::RootDir => comps.push(""),
            Component::Prefix(p) => comps.push(p.as_os_str().to_str().unwrap_or("")),
        }
    }
    Ok(Value::Str(Arc::from(
        comps.join(std::path::MAIN_SEPARATOR_STR),
    )))
}
