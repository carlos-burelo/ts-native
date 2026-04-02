use std::sync::Arc;
use tsn_core::OpCode;
use tsn_types::value::{Closure, Value};

impl super::super::Vm {
    pub(super) fn load_module_file(&mut self, abs_path: &str) -> Result<Value, String> {
        let sentinel = Value::plain_object();
        self.modules.insert(abs_path.to_owned(), sentinel);

        let source = std::fs::read_to_string(abs_path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                format!(
                    "cannot read module '{}': Access Denied (is it a directory without index.tsn?)",
                    abs_path
                )
            } else {
                format!("cannot read module '{}': {}", abs_path, e)
            }
        })?;

        let tokens = tsn_lexer::scan(&source, abs_path);
        let program = tsn_parser::parse(tokens, abs_path)
            .map_err(|errs| format!("parse error in '{}': {}", abs_path, errs[0].message))?;

        let check_result = tsn_checker::Checker::check(&program);
        let proto =
            tsn_compiler::compile_with_annotations(&program, &check_result.type_annotations)
                .map_err(|e| format!("compile error in '{}': {}", abs_path, e))?;

        let saved_exports =
            std::mem::replace(&mut self.module_exports, tsn_types::RuntimeObject::new());
        self.run_proto_inline(proto)?;
        let exports = self.take_module_exports();
        self.module_exports = saved_exports;

        Ok(exports)
    }

    pub(super) fn run_proto_inline(
        &mut self,
        proto: tsn_compiler::FunctionProto,
    ) -> Result<Value, String> {
        let closure = Arc::new(Closure {
            proto: Arc::new(proto),
            upvalues: vec![],
        });
        let base = self.stack.len();
        let ic_count = closure.proto.cache_count;
        self.frames.push(super::super::frame::CallFrame {
            closure,
            ip: 0,
            base,
            current_class: None,
            ic_slots: vec![tsn_types::chunk::CacheEntry::default(); ic_count],
        });
        let stop_at = self.frames.len() - 1;
        self.run_until(stop_at)
    }
}

impl super::super::Vm {
    pub(super) fn exec_import_op(&mut self, op: OpCode) -> Result<(), String> {
        match op {
            OpCode::OpImport => {
                let idx = self.read_u16();
                let path = self.get_str_const(idx);

                if let Some(module) = self.modules.get(path.as_ref()).cloned() {
                    self.push(module);
                } else {
                    let abs_path = resolve_import_path(path.as_ref(), &self.frames);
                    match abs_path {
                        None => {
                            self.push(Value::plain_object());
                        }
                        Some(abs) => {
                            if let Some(cached) = self.modules.get(&abs).cloned() {
                                self.push(cached);
                            } else {
                                let exports = self.load_module_file(&abs)?;
                                self.modules.insert(abs.clone(), exports.clone());
                                self.push(exports);
                            }
                        }
                    }
                }
            }
            OpCode::OpReexport => {
                let idx = self.read_u16();
                let path = self.get_str_const(idx);

                let resolved: Option<Value> = {
                    if let Some(v) = self.modules.get(path.as_ref()).cloned() {
                        Some(v)
                    } else {
                        let abs = resolve_import_path(path.as_ref(), &self.frames);
                        match abs {
                            None => None,
                            Some(a) => {
                                if let Some(cached) = self.modules.get(&a).cloned() {
                                    Some(cached)
                                } else {
                                    let exports = self.load_module_file(&a)?;
                                    self.modules.insert(a, exports.clone());
                                    Some(exports)
                                }
                            }
                        }
                    }
                };
                if let Some(Value::Object(map)) = resolved {
                    for (k, v) in unsafe { &*map }.fields.iter() {
                        self.module_exports.insert(Arc::from(k.as_ref()), v.clone());
                    }
                }
            }
            OpCode::OpMergeExports => {
                let idx = self.read_u16();
                let key = self.get_str_const(idx);
                let val = self.pop()?;
                self.module_exports.insert(Arc::from(key.as_ref()), val);
            }

            _ => unreachable!("exec_import_op called with non-import opcode: {:?}", op),
        }
        Ok(())
    }
}

pub(super) fn resolve_import_path(
    specifier: &str,
    frames: &[super::super::frame::CallFrame],
) -> Option<String> {
    if specifier.starts_with('.') {
        let base_file = frames
            .iter()
            .rev()
            .map(|f| &f.closure.proto.chunk.source_file)
            .find(|s| !s.is_empty())?;

        let base_dir = std::path::Path::new(base_file.as_str()).parent()?;
        let mut path = base_dir.join(specifier);

        if path.is_dir() {
            path = path.join("index.tsn");
        }

        let canonical = std::fs::canonicalize(&path).ok()?;
        return Some(canonical.to_string_lossy().into_owned());
    }

    // New SPEC: std: and builtin:
    if let Some(rest) = specifier.strip_prefix("std:") {
        let mut path = std::path::PathBuf::from("tsn-stdlib/std");
        for part in rest.split(':') {
            path.push(part);
        }
        path.push("mod.tsn");
        if path.exists() {
            return Some(path.to_string_lossy().into_owned());
        }
    }

    if let Some(rest) = specifier.strip_prefix("builtin:") {
        let mut path = std::path::PathBuf::from("tsn-stdlib/builtins");
        path.push(format!("{}.tsn", rest));
        if path.exists() {
            return Some(path.to_string_lossy().into_owned());
        }
    }

    None
}

pub(super) fn short_val(v: &Value) -> String {
    match v {
        Value::Null => "null".to_owned(),
        Value::Bool(b) => b.to_string(),
        Value::Int(n) => n.to_string(),
        Value::Float(f) => format!("{:.4}", f),
        Value::Str(s) => {
            if s.chars().count() > 24 {
                let truncated: String = s.chars().take(24).collect();
                format!("{:?}…", truncated)
            } else {
                format!("{:?}", s.as_ref())
            }
        }
        Value::BigInt(n) => n.to_string(),
        Value::Decimal(d) => d.to_string(),
        Value::Array(a) => format!("[..{}]", unsafe { &**a }.len()),
        Value::Object(o) => {
            let obj = unsafe { &**o };
            if let Some(c) = &obj.class {
                format!("<{}>", c.name)
            } else {
                "{..}".to_owned()
            }
        }
        Value::Closure(c) => format!("<fn {}>", c.proto.name.as_deref().unwrap_or("anon")),
        Value::NativeFn(b) => format!("<native {}>", b.1),
        Value::Class(c) => format!("<class {}>", c.name),
        Value::BoundMethod(b) => {
            format!("<bound {}>", b.method.proto.name.as_deref().unwrap_or("?"))
        }
        Value::NativeBoundMethod(b) => format!("<native-bound {}>", b.2),
        Value::Spread(v) => short_val(v),
        Value::Future(_) => "<future>".to_owned(),
        Value::Range(r) => {
            if r.inclusive {
                format!("{}..={}", r.start, r.end)
            } else {
                format!("{}..{}", r.start, r.end)
            }
        }
        Value::Map(m) => format!("[Map({})]", unsafe { &**m }.len()),
        Value::Set(s) => format!("[Set({})]", unsafe { &**s }.len()),
        Value::Symbol(s) => s.to_string(),
        Value::Generator(_) => "<generator>".to_owned(),
        Value::AsyncQueue(_) => "<asyncqueue>".to_owned(),
        Value::Char(c) => format!("'{}'", c),
    }
}
