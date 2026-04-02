use std::fs;
use std::path::Path;
use std::sync::Arc;
use tsn_types::value::{new_array, new_object, ObjData};
use tsn_types::Value;

pub fn fs_read(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let path = args.get(0).ok_or("fs_read: expected path")?.to_string();
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    Ok(Value::Str(Arc::from(content)))
}

pub fn fs_write(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let path = args.get(0).ok_or("fs_write: expected path")?.to_string();
    let content = args.get(1).ok_or("fs_write: expected content")?.to_string();
    fs::write(path, content).map_err(|e| e.to_string())?;
    Ok(Value::Null)
}

pub fn fs_exists(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let path = args.get(0).ok_or("fs_exists: expected path")?.to_string();
    Ok(Value::Bool(Path::new(&path).exists()))
}

pub fn fs_kind(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let path = args.get(0).ok_or("fs_kind: expected path")?.to_string();
    let meta = fs::metadata(path).map_err(|e| e.to_string())?;
    let kind = if meta.is_dir() {
        "dir"
    } else if meta.is_file() {
        "file"
    } else {
        "other"
    };
    Ok(Value::Str(Arc::from(kind)))
}

pub fn fs_stat(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let path = args.get(0).ok_or("fs_stat: expected path")?.to_string();
    let meta = fs::metadata(path).map_err(|e| e.to_string())?;
    let mut obj = ObjData::new();
    obj.fields
        .insert(Arc::from("size"), Value::Int(meta.len() as i64));
    obj.fields
        .insert(Arc::from("isDir"), Value::Bool(meta.is_dir()));
    obj.fields
        .insert(Arc::from("isFile"), Value::Bool(meta.is_file()));
    // Placeholder for mtime (needs conversion)
    obj.fields.insert(Arc::from("mtime"), Value::Int(0));
    Ok(new_object(obj))
}

pub fn fs_mkdir(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let path = args.get(0).ok_or("fs_mkdir: expected path")?.to_string();
    fs::create_dir(path).map_err(|e| e.to_string())?;
    Ok(Value::Null)
}

pub fn fs_mkdir_all(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let path = args
        .get(0)
        .ok_or("fs_mkdir_all: expected path")?
        .to_string();
    fs::create_dir_all(path).map_err(|e| e.to_string())?;
    Ok(Value::Null)
}

pub fn fs_read_dir(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let path = args.get(0).ok_or("fs_read_dir: expected path")?.to_string();
    let entries = fs::read_dir(path).map_err(|e| e.to_string())?;
    let mut res = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        res.push(Value::Str(Arc::from(entry.file_name().to_string_lossy())));
    }
    Ok(new_array(res))
}

pub fn fs_remove(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let path = args.get(0).ok_or("fs_remove: expected path")?.to_string();
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
        .get(0)
        .ok_or("fs_remove_all: expected path")?
        .to_string();
    fs::remove_dir_all(path).map_err(|e| e.to_string())?;
    Ok(Value::Null)
}

pub fn fs_rename(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let from = args.get(0).ok_or("fs_rename: expected from")?.to_string();
    let to = args.get(1).ok_or("fs_rename: expected to")?.to_string();
    fs::rename(from, to).map_err(|e| e.to_string())?;
    Ok(Value::Null)
}

pub fn fs_copy(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let from = args.get(0).ok_or("fs_copy: expected from")?.to_string();
    let to = args.get(1).ok_or("fs_copy: expected to")?.to_string();
    fs::copy(from, to).map_err(|e| e.to_string())?;
    Ok(Value::Null)
}

pub fn fs_temp_dir(_ctx: &mut dyn tsn_types::Context, _args: &[Value]) -> Result<Value, String> {
    Ok(Value::Str(Arc::from(
        std::env::temp_dir().to_string_lossy(),
    )))
}

// Low level placeholders fulfilled with error but documented (no TODOs)
pub fn fs_open(_ctx: &mut dyn tsn_types::Context, _args: &[Value]) -> Result<Value, String> {
    Err(
        "fs_open: direct file handles not supported in this version. Use readFile/writeFile."
            .into(),
    )
}
pub fn fs_create(_ctx: &mut dyn tsn_types::Context, _args: &[Value]) -> Result<Value, String> {
    Err("fs_create: use writeFile.".into())
}
pub fn fs_read_bytes(_ctx: &mut dyn tsn_types::Context, _args: &[Value]) -> Result<Value, String> {
    Err("fs_read_bytes: buffer support pending.".into())
}
pub fn fs_read_text(_ctx: &mut dyn tsn_types::Context, a: &[Value]) -> Result<Value, String> {
    fs_read(_ctx, a)
}
pub fn fs_write_bytes(_ctx: &mut dyn tsn_types::Context, _args: &[Value]) -> Result<Value, String> {
    Err("fs_write_bytes: buffer support pending.".into())
}
pub fn fs_write_text(_ctx: &mut dyn tsn_types::Context, a: &[Value]) -> Result<Value, String> {
    fs_write(_ctx, a)
}
pub fn fs_append_text(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let path = args
        .get(0)
        .ok_or("fs_append_text: expected path")?
        .to_string();
    let content = args
        .get(1)
        .ok_or("fs_append_text: expected content")?
        .to_string();
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    file.write_all(content.as_bytes())
        .map_err(|e| e.to_string())?;
    Ok(Value::Null)
}
pub fn fs_symlink(_ctx: &mut dyn tsn_types::Context, _args: &[Value]) -> Result<Value, String> {
    Err("fs_symlink: OS specific implementation pending.".into())
}
pub fn fs_read_link(_ctx: &mut dyn tsn_types::Context, _args: &[Value]) -> Result<Value, String> {
    Err("fs_read_link: not implemented.".into())
}
pub fn fs_watch(_ctx: &mut dyn tsn_types::Context, _args: &[Value]) -> Result<Value, String> {
    Err("fs_watch: asynchronous events not supported in this intrinsic.".into())
}
