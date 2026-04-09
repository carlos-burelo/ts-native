use base64::Engine;
use hmac::{Hmac, Mac};
use rand::RngCore;
use sha2::{Digest, Sha256, Sha512};
use std::sync::Arc;
use tsn_op_macros::op;
use tsn_types::{value::new_array, Value};
use uuid::Uuid;

#[op("sha256")]
pub fn crypto_sha256(args: &[Value]) -> Result<Value, String> {
    let input = args.get(0).map(|v| v.to_string()).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    Ok(Value::Str(Arc::from(format!("{:x}", result))))
}

#[op("sha512")]
pub fn crypto_sha512(args: &[Value]) -> Result<Value, String> {
    let input = args.get(0).map(|v| v.to_string()).unwrap_or_default();
    let mut hasher = Sha512::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    Ok(Value::Str(Arc::from(format!("{:x}", result))))
}

#[op("randomBytes")]
pub fn crypto_random_bytes(args: &[Value]) -> Result<Value, String> {
    let len = match args.get(0) {
        Some(Value::Int(i)) if *i >= 0 => *i as usize,
        _ => return Err("Crypto.randomBytes: expected non-negative int".into()),
    };
    let mut buf = vec![0u8; len];
    rand::thread_rng().fill_bytes(&mut buf);
    let items: Vec<Value> = buf.into_iter().map(|b| Value::Int(b as i64)).collect();
    Ok(new_array(items))
}

#[op("randomHex")]
pub fn crypto_random_hex(args: &[Value]) -> Result<Value, String> {
    let len = match args.get(0) {
        Some(Value::Int(i)) if *i >= 0 => *i as usize,
        _ => return Err("Crypto.randomHex: expected non-negative int".into()),
    };
    let mut buf = vec![0u8; len];
    rand::thread_rng().fill_bytes(&mut buf);
    Ok(Value::Str(Arc::from(hex::encode(buf))))
}

#[op("base64Encode")]
pub fn crypto_base64_enc(args: &[Value]) -> Result<Value, String> {
    let input = args.get(0).map(|v| v.to_string()).unwrap_or_default();
    Ok(Value::Str(Arc::from(
        base64::engine::general_purpose::STANDARD.encode(input.as_bytes()),
    )))
}

#[op("base64Decode")]
pub fn crypto_base64_dec(args: &[Value]) -> Result<Value, String> {
    let input = args.get(0).map(|v| v.to_string()).unwrap_or_default();
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(input.as_bytes())
        .map_err(|e| format!("Crypto.base64Dec: {}", e))?;
    let s =
        String::from_utf8(decoded).map_err(|e| format!("Crypto.base64Dec: invalid utf8: {}", e))?;
    Ok(Value::Str(Arc::from(s)))
}

#[op("hmac")]
pub fn crypto_hmac(args: &[Value]) -> Result<Value, String> {
    let algo = args.get(0).map(|v| v.to_string()).unwrap_or_default();
    let key = args.get(1).map(|v| v.to_string()).unwrap_or_default();
    let data = args.get(2).map(|v| v.to_string()).unwrap_or_default();

    let digest = match algo.as_str() {
        "sha256" => {
            let mut mac =
                Hmac::<Sha256>::new_from_slice(key.as_bytes()).map_err(|e| e.to_string())?;
            mac.update(data.as_bytes());
            mac.finalize().into_bytes().to_vec()
        }
        "sha512" => {
            let mut mac =
                Hmac::<Sha512>::new_from_slice(key.as_bytes()).map_err(|e| e.to_string())?;
            mac.update(data.as_bytes());
            mac.finalize().into_bytes().to_vec()
        }
        _ => return Err(format!("Crypto.hmac: unsupported algorithm: {}", algo)),
    };
    Ok(Value::Str(Arc::from(hex::encode(digest))))
}

#[op("uuid")]
pub fn crypto_uuid(_args: &[Value]) -> Result<Value, String> {
    Ok(Value::Str(Arc::from(Uuid::new_v4().to_string())))
}

pub const OPS: &[crate::host_ops::HostOp] = &[
    crypto_sha256_OP,
    crypto_sha512_OP,
    crypto_random_bytes_OP,
    crypto_random_hex_OP,
    crypto_base64_enc_OP,
    crypto_base64_dec_OP,
    crypto_hmac_OP,
    crypto_uuid_OP,
];
