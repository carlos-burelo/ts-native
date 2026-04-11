use base64::Engine;
use hmac::{Hmac, Mac};
use rand::RngCore;
use sha2::{Digest, Sha256, Sha512};
use std::sync::Arc;
use tsn_types::{value::{new_array, new_object, ObjData}, Value};
use tsn_types::NativeFn;
use uuid::Uuid;

pub fn crypto_sha256(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let input = args.first().map(|v| v.to_string()).unwrap_or_default();
    let mut h = Sha256::new();
    h.update(input.as_bytes());
    Ok(Value::Str(Arc::from(format!("{:x}", h.finalize()))))
}

pub fn crypto_sha512(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let input = args.first().map(|v| v.to_string()).unwrap_or_default();
    let mut h = Sha512::new();
    h.update(input.as_bytes());
    Ok(Value::Str(Arc::from(format!("{:x}", h.finalize()))))
}

pub fn crypto_random_bytes(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let len = match args.first() {
        Some(Value::Int(i)) if *i >= 0 => *i as usize,
        _ => return Err("Crypto.randomBytes: expected non-negative int".into()),
    };
    let mut buf = vec![0u8; len];
    rand::thread_rng().fill_bytes(&mut buf);
    let items: Vec<Value> = buf.into_iter().map(|b| Value::Int(b as i64)).collect();
    Ok(new_array(items))
}

pub fn crypto_random_hex(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let len = match args.first() {
        Some(Value::Int(i)) if *i >= 0 => *i as usize,
        _ => return Err("Crypto.randomHex: expected non-negative int".into()),
    };
    let mut buf = vec![0u8; len];
    rand::thread_rng().fill_bytes(&mut buf);
    Ok(Value::Str(Arc::from(hex::encode(buf))))
}

pub fn crypto_base64_encode(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let input = args.first().map(|v| v.to_string()).unwrap_or_default();
    Ok(Value::Str(Arc::from(
        base64::engine::general_purpose::STANDARD.encode(input.as_bytes()),
    )))
}

pub fn crypto_base64_decode(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let input = args.first().map(|v| v.to_string()).unwrap_or_default();
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(input.as_bytes())
        .map_err(|e| format!("Crypto.base64Decode: {}", e))?;
    let s = String::from_utf8(decoded)
        .map_err(|e| format!("Crypto.base64Decode: invalid utf8: {}", e))?;
    Ok(Value::Str(Arc::from(s)))
}

pub fn crypto_hmac(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let algo = args.first().map(|v| v.to_string()).unwrap_or_default();
    let key  = args.get(1).map(|v| v.to_string()).unwrap_or_default();
    let data = args.get(2).map(|v| v.to_string()).unwrap_or_default();
    let digest = match algo.as_str() {
        "sha256" => {
            let mut mac = Hmac::<Sha256>::new_from_slice(key.as_bytes()).map_err(|e| e.to_string())?;
            mac.update(data.as_bytes());
            mac.finalize().into_bytes().to_vec()
        }
        "sha512" => {
            let mut mac = Hmac::<Sha512>::new_from_slice(key.as_bytes()).map_err(|e| e.to_string())?;
            mac.update(data.as_bytes());
            mac.finalize().into_bytes().to_vec()
        }
        _ => return Err(format!("Crypto.hmac: unsupported algorithm: {}", algo)),
    };
    Ok(Value::Str(Arc::from(hex::encode(digest))))
}

pub fn crypto_uuid(_ctx: &mut dyn tsn_types::Context, _args: &[Value]) -> Result<Value, String> {
    Ok(Value::Str(Arc::from(Uuid::new_v4().to_string())))
}

pub fn build() -> Value {
    let mut ns = ObjData::new();
    ns.set_field(Arc::from("sha256"),        Value::NativeFn(Box::new((crypto_sha256        as NativeFn, "sha256"))));
    ns.set_field(Arc::from("sha512"),        Value::NativeFn(Box::new((crypto_sha512        as NativeFn, "sha512"))));
    ns.set_field(Arc::from("randomBytes"),   Value::NativeFn(Box::new((crypto_random_bytes  as NativeFn, "randomBytes"))));
    ns.set_field(Arc::from("randomHex"),     Value::NativeFn(Box::new((crypto_random_hex    as NativeFn, "randomHex"))));
    ns.set_field(Arc::from("base64Encode"),  Value::NativeFn(Box::new((crypto_base64_encode as NativeFn, "base64Encode"))));
    ns.set_field(Arc::from("base64Decode"),  Value::NativeFn(Box::new((crypto_base64_decode as NativeFn, "base64Decode"))));
    ns.set_field(Arc::from("hmac"),          Value::NativeFn(Box::new((crypto_hmac          as NativeFn, "hmac"))));
    ns.set_field(Arc::from("uuid"),          Value::NativeFn(Box::new((crypto_uuid          as NativeFn, "uuid"))));

    let mut exports = ObjData::new();
    exports.set_field(Arc::from("Crypto"), new_object(ns));
    new_object(exports)
}
