use std::fs;
use std::io::Write as IoWrite;
use std::path::Path;
use std::sync::Arc;
use tsn_types::NativeFn;
use tsn_types::{
    value::{new_array, new_object, ObjData},
    Value,
};

pub fn fs_read(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let path = args
        .first()
        .ok_or("Fs.readFile: expected path")?
        .to_string();
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    Ok(Value::Str(Arc::from(content)))
}

pub fn fs_write(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let path = args
        .first()
        .ok_or("Fs.writeFile: expected path")?
        .to_string();
    let content = args
        .get(1)
        .ok_or("Fs.writeFile: expected content")?
        .to_string();
    fs::write(path, content).map_err(|e| e.to_string())?;
    Ok(Value::Null)
}

pub fn fs_append(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let path = args
        .first()
        .ok_or("Fs.appendFile: expected path")?
        .to_string();
    let content = args
        .get(1)
        .ok_or("Fs.appendFile: expected content")?
        .to_string();
    let mut file = fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    file.write_all(content.as_bytes())
        .map_err(|e| e.to_string())?;
    Ok(Value::Null)
}

pub fn fs_exists(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let path = args.first().ok_or("Fs.exists: expected path")?.to_string();
    Ok(Value::Bool(Path::new(&path).exists()))
}

pub fn fs_stat(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let path = args.first().ok_or("Fs.stat: expected path")?.to_string();
    let meta = fs::metadata(path).map_err(|e| e.to_string())?;
    let mut obj = ObjData::new();
    obj.fields
        .insert(Arc::from("size"), Value::Int(meta.len() as i64));
    obj.fields
        .insert(Arc::from("isDir"), Value::Bool(meta.is_dir()));
    obj.fields
        .insert(Arc::from("isFile"), Value::Bool(meta.is_file()));
    obj.fields.insert(Arc::from("mtime"), Value::Int(0));
    Ok(new_object(obj))
}

pub fn fs_mkdir(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let path = args.first().ok_or("Fs.mkdir: expected path")?.to_string();
    fs::create_dir(path).map_err(|e| e.to_string())?;
    Ok(Value::Null)
}

pub fn fs_mkdir_all(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let path = args
        .first()
        .ok_or("Fs.mkdirAll: expected path")?
        .to_string();
    fs::create_dir_all(path).map_err(|e| e.to_string())?;
    Ok(Value::Null)
}

pub fn fs_read_dir(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let path = args.first().ok_or("Fs.readDir: expected path")?.to_string();
    let entries = fs::read_dir(path).map_err(|e| e.to_string())?;
    let mut res = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        res.push(Value::Str(Arc::from(entry.file_name().to_string_lossy())));
    }
    Ok(new_array(res))
}

pub fn fs_remove(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let path = args.first().ok_or("Fs.remove: expected path")?.to_string();
    let meta = fs::metadata(&path).map_err(|e| e.to_string())?;
    if meta.is_dir() {
        fs::remove_dir(path).map_err(|e| e.to_string())?;
    } else {
        fs::remove_file(path).map_err(|e| e.to_string())?;
    }
    Ok(Value::Null)
}

pub fn fs_remove_all(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let path = args
        .first()
        .ok_or("Fs.removeAll: expected path")?
        .to_string();
    fs::remove_dir_all(path).map_err(|e| e.to_string())?;
    Ok(Value::Null)
}

pub fn fs_rename(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let from = args.first().ok_or("Fs.rename: expected from")?.to_string();
    let to = args.get(1).ok_or("Fs.rename: expected to")?.to_string();
    fs::rename(from, to).map_err(|e| e.to_string())?;
    Ok(Value::Null)
}

pub fn fs_copy(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let from = args.first().ok_or("Fs.copy: expected from")?.to_string();
    let to = args.get(1).ok_or("Fs.copy: expected to")?.to_string();
    fs::copy(from, to).map_err(|e| e.to_string())?;
    Ok(Value::Null)
}

pub fn fs_temp_dir(_ctx: &mut dyn tsn_types::Context, _args: &[Value]) -> Result<Value, String> {
    Ok(Value::Str(Arc::from(
        std::env::temp_dir().to_string_lossy(),
    )))
}

pub fn build() -> Value {
    let mut ns = ObjData::new();
    ns.set_field(
        Arc::from("readFile"),
        Value::NativeFn(Box::new((fs_read as NativeFn, "readFile"))),
    );
    ns.set_field(
        Arc::from("writeFile"),
        Value::NativeFn(Box::new((fs_write as NativeFn, "writeFile"))),
    );
    ns.set_field(
        Arc::from("appendFile"),
        Value::NativeFn(Box::new((fs_append as NativeFn, "appendFile"))),
    );
    ns.set_field(
        Arc::from("exists"),
        Value::NativeFn(Box::new((fs_exists as NativeFn, "exists"))),
    );
    ns.set_field(
        Arc::from("stat"),
        Value::NativeFn(Box::new((fs_stat as NativeFn, "stat"))),
    );
    ns.set_field(
        Arc::from("mkdir"),
        Value::NativeFn(Box::new((fs_mkdir as NativeFn, "mkdir"))),
    );
    ns.set_field(
        Arc::from("mkdirAll"),
        Value::NativeFn(Box::new((fs_mkdir_all as NativeFn, "mkdirAll"))),
    );
    ns.set_field(
        Arc::from("readDir"),
        Value::NativeFn(Box::new((fs_read_dir as NativeFn, "readDir"))),
    );
    ns.set_field(
        Arc::from("remove"),
        Value::NativeFn(Box::new((fs_remove as NativeFn, "remove"))),
    );
    ns.set_field(
        Arc::from("removeAll"),
        Value::NativeFn(Box::new((fs_remove_all as NativeFn, "removeAll"))),
    );
    ns.set_field(
        Arc::from("rename"),
        Value::NativeFn(Box::new((fs_rename as NativeFn, "rename"))),
    );
    ns.set_field(
        Arc::from("copy"),
        Value::NativeFn(Box::new((fs_copy as NativeFn, "copy"))),
    );
    ns.set_field(
        Arc::from("tempDir"),
        Value::NativeFn(Box::new((fs_temp_dir as NativeFn, "tempDir"))),
    );

    let mut exports = ObjData::new();
    exports.set_field(Arc::from("Fs"), new_object(ns));
    new_object(exports)
}
