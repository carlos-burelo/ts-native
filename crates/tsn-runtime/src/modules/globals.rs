use std::sync::Arc;
use tsn_types::value::{new_object, ClassObj, ObjData};
use tsn_types::Value;

pub fn global_print(ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    super::console::console_log(ctx, args)
}

pub fn global_debug(ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    super::console::console_debug(ctx, args)
}

pub fn global_input(ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    super::io::io_read_line(ctx, args)
}

pub fn global_assert(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let label = args.first().map(Value::to_string).unwrap_or_else(|| "assert failed".to_owned());
    let cond = matches!(args.get(1), Some(Value::Bool(true)));
    if cond {
        Ok(Value::Null)
    } else {
        Err(label)
    }
}

pub fn global_assert_summary(
    ctx: &mut dyn tsn_types::Context,
    args: &[Value],
) -> Result<Value, String> {
    super::testing::testing_summary(ctx, args)
}

pub fn symbol_global() -> Value {
    let mut obj = ObjData::new();
    obj.fields.insert(
        Arc::from("iterator"),
        Value::Symbol(tsn_types::value::SymbolKind::Iterator),
    );
    obj.fields.insert(
        Arc::from("asyncIterator"),
        Value::Symbol(tsn_types::value::SymbolKind::AsyncIterator),
    );
    new_object(obj)
}

pub fn str_type_global() -> Value {
    let mut cls = ClassObj::new_native("str");
    cls.statics.insert(Arc::from("EMPTY"), Value::Str(Arc::from("")));
    cls.statics.insert(
        Arc::from("fromCharCode"),
        Value::native(
            crate::modules::primitives::str_from_char_code,
            "fromCharCode",
        ),
    );
    cls.statics.insert(
        Arc::from("from"),
        Value::native(crate::modules::primitives::str_from_value, "from"),
    );
    cls.statics.insert(
        Arc::from("join"),
        Value::native(crate::modules::array::array_join, "join"),
    );
    Value::Class(Arc::new(cls))
}

pub fn int_type_global() -> Value {
    let mut cls = ClassObj::new_native("int");
    cls.statics
        .insert(Arc::from("MAX_VALUE"), Value::Int(9_223_372_036_854_775_807));
    cls.statics
        .insert(Arc::from("MIN_VALUE"), Value::Int(-9_223_372_036_854_775_808));
    cls.statics.insert(
        Arc::from("parse"),
        Value::native(crate::modules::primitives::int_parse, "parse"),
    );
    cls.statics.insert(
        Arc::from("isInteger"),
        Value::native(crate::modules::primitives::int_is_integer, "isInteger"),
    );
    Value::Class(Arc::new(cls))
}

pub fn float_type_global() -> Value {
    let mut cls = ClassObj::new_native("float");
    cls.statics
        .insert(Arc::from("MAX_VALUE"), Value::Float(f64::MAX));
    cls.statics
        .insert(Arc::from("MIN_VALUE"), Value::Float(f64::MIN_POSITIVE));
    cls.statics
        .insert(Arc::from("EPSILON"), Value::Float(f64::EPSILON));
    cls.statics.insert(
        Arc::from("parse"),
        Value::native(crate::modules::primitives::float_parse, "parse"),
    );
    cls.statics.insert(
        Arc::from("isNaN"),
        Value::native(crate::modules::primitives::float_is_nan, "isNaN"),
    );
    cls.statics.insert(
        Arc::from("isFinite"),
        Value::native(crate::modules::primitives::float_is_finite, "isFinite"),
    );
    Value::Class(Arc::new(cls))
}

pub fn array_type_global() -> Value {
    let mut cls = ClassObj::new_native("Array");
    cls.statics.insert(
        Arc::from("isArray"),
        Value::native(crate::modules::array::array_is_array, "isArray"),
    );
    Value::Class(Arc::new(cls))
}

fn error_message_arg(args: &[Value]) -> Arc<str> {
    match args.get(1) {
        Some(Value::Str(s)) => s.clone(),
        Some(v) => Arc::from(v.to_string()),
        None => Arc::from(""),
    }
}

fn set_error_fields(args: &[Value], name: &'static str) -> Result<Value, String> {
    let this_ptr = match args.first() {
        Some(Value::Object(o)) => *o,
        _ => return Ok(Value::Null),
    };
    let this = unsafe { &mut *this_ptr };
    let message_val = Value::Str(error_message_arg(args));
    let name_val = Value::Str(Arc::from(name));
    let stack_val = Value::Str(Arc::from(""));

    if let Some(cls) = &this.class {
        if let Some(&slot) = cls.field_map.get("message") {
            if slot < this.slots.len() {
                this.slots[slot] = message_val.clone();
            }
        }
        if let Some(&slot) = cls.field_map.get("name") {
            if slot < this.slots.len() {
                this.slots[slot] = name_val.clone();
            }
        }
        if let Some(&slot) = cls.field_map.get("stack") {
            if slot < this.slots.len() {
                this.slots[slot] = stack_val.clone();
            }
        }
    }

    // Keep dynamic fields mirrored for native-friendly introspection paths.
    this.fields.insert(Arc::from("name"), name_val);
    this.fields.insert(Arc::from("message"), message_val);
    this.fields.insert(Arc::from("stack"), stack_val);
    Ok(Value::Null)
}

pub fn error_ctor(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    set_error_fields(args, "Error")
}

pub fn type_error_ctor(
    _ctx: &mut dyn tsn_types::Context,
    args: &[Value],
) -> Result<Value, String> {
    set_error_fields(args, "TypeError")
}

pub fn range_error_ctor(
    _ctx: &mut dyn tsn_types::Context,
    args: &[Value],
) -> Result<Value, String> {
    set_error_fields(args, "RangeError")
}

pub fn error_to_string(
    _ctx: &mut dyn tsn_types::Context,
    args: &[Value],
) -> Result<Value, String> {
    let this_ptr = match args.first() {
        Some(Value::Object(o)) => *o,
        _ => return Ok(Value::Str(Arc::from("Error"))),
    };
    let this = unsafe { &*this_ptr };
    let name = match this.fields.get("name") {
        Some(Value::Str(s)) if !s.is_empty() => s.as_ref(),
        _ => "Error",
    };
    let message = match this.fields.get("message") {
        Some(Value::Str(s)) if !s.is_empty() => s.as_ref(),
        _ => "",
    };
    if message.is_empty() {
        Ok(Value::Str(Arc::from(name.to_owned())))
    } else {
        Ok(Value::Str(Arc::from(format!("{}: {}", name, message))))
    }
}

pub fn error_classes_globals() -> (Value, Value, Value) {
    let mut error_cls = ClassObj::new_native("Error");
    error_cls.declare_field(Arc::from("message"));
    error_cls.declare_field(Arc::from("name"));
    error_cls.declare_field(Arc::from("stack"));
    error_cls.add_method("constructor", Value::native(error_ctor, "constructor"));
    error_cls.add_method("toString", Value::native(error_to_string, "toString"));
    let error_arc = Arc::new(error_cls);

    let mut type_error_cls = ClassObj::new_native("TypeError");
    type_error_cls.superclass = Some(error_arc.clone());
    type_error_cls.add_method(
        "constructor",
        Value::native(type_error_ctor, "constructor"),
    );
    type_error_cls.add_method("toString", Value::native(error_to_string, "toString"));

    let mut range_error_cls = ClassObj::new_native("RangeError");
    range_error_cls.superclass = Some(error_arc.clone());
    range_error_cls.add_method(
        "constructor",
        Value::native(range_error_ctor, "constructor"),
    );
    range_error_cls.add_method("toString", Value::native(error_to_string, "toString"));

    (
        Value::Class(error_arc),
        Value::Class(Arc::new(type_error_cls)),
        Value::Class(Arc::new(range_error_cls)),
    )
}