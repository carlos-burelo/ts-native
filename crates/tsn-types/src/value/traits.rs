use super::{alloc_array, alloc_object, ClassObj, ObjData, Value};
use crate::native::NativeFn;
use std::fmt;
use std::sync::Arc;

impl Value {
    #[inline(always)]
    pub fn native(func: NativeFn, name: &'static str) -> Self {
        Value::NativeFn(Box::new((func, name)))
    }

    #[inline(always)]
    pub fn native_bound(receiver: Value, func: NativeFn, name: &'static str) -> Self {
        Value::NativeBoundMethod(Box::new((receiver, func, name)))
    }

    pub fn instance(class: Arc<ClassObj>) -> Self {
        let ptr = alloc_object();
        unsafe {
            *ptr = ObjData::new_instance(class);
        }
        Value::Object(ptr)
    }

    pub fn plain_object() -> Self {
        Value::Object(alloc_object())
    }

    #[inline(always)]
    pub fn empty_array() -> Self {
        Value::Array(alloc_array())
    }

    pub fn is_truthy(&self) -> Result<bool, String> {
        match self {
            Value::Bool(b) => Ok(*b),
            _ => Err(format!(
                "expected bool for condition, got {}",
                self.type_name()
            )),
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "bool",
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::Str(_) => "str",
            Value::BigInt(_) => "bigint",
            Value::Decimal(_) => "decimal",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
            Value::Closure(_) => "fn",
            Value::Class(_) => "class",
            Value::BoundMethod(_) => "fn",
            Value::NativeFn(_) => "fn",
            Value::NativeBoundMethod(_) => "fn",
            Value::Spread(v) => v.type_name(),
            Value::Future(_) => "Future",
            Value::Range(_) => "range",
            Value::Map(_) => "Map",
            Value::Set(_) => "Set",
            Value::Symbol(_) => "symbol",
            Value::Generator(_) => "generator",
            Value::AsyncQueue(_) => "asyncqueue",
            Value::Char(_) => "char",
        }
    }

    pub fn num_add(&self, rhs: &Value) -> Result<Value, String> {
        match (self, rhs) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 + b)),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a + *b as f64)),
            (Value::Decimal(a), Value::Decimal(b)) => Ok(Value::Decimal(Box::new(**a + **b))),
            (Value::Decimal(a), Value::Int(b)) => Ok(Value::Decimal(Box::new(
                **a + rust_decimal::Decimal::from(*b),
            ))),
            (Value::Int(a), Value::Decimal(b)) => Ok(Value::Decimal(Box::new(
                rust_decimal::Decimal::from(*a) + **b,
            ))),
            (Value::Str(a), Value::Str(b)) => {
                let mut s = String::with_capacity(a.len() + b.len());
                s.push_str(a);
                s.push_str(b);
                Ok(Value::Str(Arc::from(s)))
            }
            (Value::Str(a), other) => {
                let mut s = a.to_string();
                s.push_str(&other.to_string());
                Ok(Value::Str(Arc::from(s)))
            }
            (other, Value::Str(b)) => {
                let mut s = other.to_string();
                s.push_str(b);
                Ok(Value::Str(Arc::from(s)))
            }
            _ => Err(format!(
                "cannot add {} + {}",
                self.type_name(),
                rhs.type_name()
            )),
        }
    }
}

impl std::hash::Hash for Value {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Value::Null => {}
            Value::Bool(b) => b.hash(state),
            Value::Int(n) => n.hash(state),
            Value::Float(f) => f.to_bits().hash(state),
            Value::Str(s) => s.hash(state),
            Value::BigInt(n) => n.hash(state),
            Value::Decimal(d) => d.hash(state),
            Value::Array(a) => (*a as usize).hash(state),
            Value::Object(o) => (*o as usize).hash(state),
            Value::Closure(c) => Arc::as_ptr(c).hash(state),
            Value::Class(c) => Arc::as_ptr(c).hash(state),
            Value::BoundMethod(m) => Arc::as_ptr(m).hash(state),
            Value::NativeFn(b) => (b.0 as usize).hash(state),
            Value::NativeBoundMethod(b) => {
                b.0.hash(state);
                (b.1 as usize).hash(state);
            }
            Value::Spread(v) => v.hash(state),
            Value::Future(f) => f.hash(state),
            Value::Range(r) => r.hash(state),
            Value::Map(m) => (*m as usize).hash(state),
            Value::Set(s) => (*s as usize).hash(state),
            Value::Symbol(s) => s.hash(state),
            Value::Generator(g) => g.hash(state),
            Value::AsyncQueue(q) => Arc::as_ptr(&q.0).hash(state),
            Value::Char(c) => c.hash(state),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Null, Value::Null) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Int(a), Value::Float(b)) => (*a as f64) == *b,
            (Value::Float(a), Value::Int(b)) => *a == (*b as f64),
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::BigInt(a), Value::BigInt(b)) => a == b,
            (Value::Decimal(a), Value::Decimal(b)) => a == b,
            (Value::Range(a), Value::Range(b)) => a == b,
            (Value::Future(a), Value::Future(b)) => a == b,
            (Value::Map(a), Value::Map(b)) => *a == *b,
            (Value::Set(a), Value::Set(b)) => *a == *b,
            (Value::Array(a), Value::Array(b)) => *a == *b,
            (Value::Object(a), Value::Object(b)) => *a == *b,
            (Value::Closure(a), Value::Closure(b)) => Arc::ptr_eq(a, b),
            (Value::Class(a), Value::Class(b)) => Arc::ptr_eq(a, b),
            (Value::Symbol(a), Value::Symbol(b)) => a == b,
            (Value::Generator(a), Value::Generator(b)) => a == b,
            (Value::AsyncQueue(a), Value::AsyncQueue(b)) => Arc::ptr_eq(&a.0, &b.0),
            (Value::Char(a), Value::Char(b)) => a == b,
            _ => false,
        }
    }
}
impl Eq for Value {}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Value::Null, Value::Null) => Some(std::cmp::Ordering::Equal),
            (Value::Bool(a), Value::Bool(b)) => a.partial_cmp(b),
            (Value::Int(a), Value::Int(b)) => a.partial_cmp(b),
            (Value::Float(a), Value::Float(b)) => a.partial_cmp(b),
            (Value::Int(a), Value::Float(b)) => (*a as f64).partial_cmp(b),
            (Value::Float(a), Value::Int(b)) => a.partial_cmp(&(*b as f64)),
            (Value::Str(a), Value::Str(b)) => a.partial_cmp(b),
            (Value::Char(a), Value::Char(b)) => a.partial_cmp(b),
            (Value::BigInt(a), Value::BigInt(b)) => a.partial_cmp(b),
            (Value::Decimal(a), Value::Decimal(b)) => a.partial_cmp(b),
            _ => {
                if self == other {
                    Some(std::cmp::Ordering::Equal)
                } else {
                    None
                }
            }
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => write!(f, "null"),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Int(n) => write!(f, "{}", n),
            Value::Float(d) => {
                if d.fract() == 0.0 && d.abs() < 9_007_199_254_740_992.0 {
                    write!(f, "{}", *d as i64)
                } else {
                    write!(f, "{}", d)
                }
            }
            Value::Str(s) => write!(f, "{}", s),
            Value::BigInt(n) => write!(f, "{}n", n),
            Value::Decimal(d) => write!(f, "{}", d),
            Value::Array(ptr) => {
                let v = unsafe { &**ptr };
                write!(f, "[")?;
                for (i, val) in v.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", val)?;
                }
                write!(f, "]")
            }
            Value::Object(ptr) => {
                let obj = unsafe { &**ptr };
                if let Some(class) = &obj.class {
                    write!(f, "[object {}]", class.name)
                } else {
                    write!(f, "{{ ")?;
                    for (i, (k, v)) in obj.fields.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}: {}", k, v)?;
                    }
                    write!(f, " }}")
                }
            }
            Value::Closure(c) => write!(
                f,
                "[Function: {}]",
                c.proto.name.as_deref().unwrap_or("<fn>")
            ),
            Value::Class(c) => write!(f, "[class {}]", c.name),
            Value::BoundMethod(m) => write!(
                f,
                "[BoundMethod: {}]",
                m.method.proto.name.as_deref().unwrap_or("<method>")
            ),
            Value::NativeFn(b) => write!(f, "[NativeFn: {}]", b.1),
            Value::NativeBoundMethod(b) => write!(f, "[Function: {}]", b.2),
            Value::Spread(v) => write!(f, "{}", v),
            Value::Future(fut) => match fut.peek_state() {
                crate::future::FutureState::Pending => write!(f, "Future(<pending>)"),
                crate::future::FutureState::Resolved(v) => write!(f, "Future({})", v),
                crate::future::FutureState::Rejected(v) => write!(f, "Future(<rejected:{}>)", v),
            },
            Value::Range(r) => write!(f, "{}..{}", r.start, r.end),
            Value::Map(ptr) => {
                let m = unsafe { &**ptr };
                write!(f, "Map({})", m.len())
            }
            Value::Set(ptr) => {
                let s = unsafe { &**ptr };
                write!(f, "Set({})", s.len())
            }
            Value::Symbol(s) => write!(f, "{}", s),
            Value::Generator(_) => write!(f, "[Generator]"),
            Value::AsyncQueue(_) => write!(f, "[AsyncQueue]"),
            Value::Char(c) => write!(f, "'{}'", c),
        }
    }
}
