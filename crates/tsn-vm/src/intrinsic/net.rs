use base64::{engine::general_purpose::STANDARD, Engine};
use std::sync::Arc;
use tsn_types::{
    value::{new_array, new_object, ObjData},
    Value,
};
use url::Url;

pub fn net_is_ip(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let s = args.get(0).map(|v| v.to_string()).unwrap_or_default();
    Ok(Value::Bool(s.parse::<std::net::IpAddr>().is_ok()))
}

pub fn net_is_ipv4(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let s = args.get(0).map(|v| v.to_string()).unwrap_or_default();
    Ok(Value::Bool(s.parse::<std::net::Ipv4Addr>().is_ok()))
}

pub fn net_is_ipv6(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let s = args.get(0).map(|v| v.to_string()).unwrap_or_default();
    Ok(Value::Bool(s.parse::<std::net::Ipv6Addr>().is_ok()))
}

pub fn net_join_host_port(
    _ctx: &mut dyn tsn_types::Context,
    args: &[Value],
) -> Result<Value, String> {
    let host = args.get(0).map(|v| v.to_string()).unwrap_or_default();
    let port = match args.get(1) {
        Some(Value::Int(i)) => *i,
        _ => return Err("Net.joinHostPort: expected int port".into()),
    };
    let res = if host.contains(':') && !host.starts_with('[') {
        format!("[{}]:{}", host, port)
    } else {
        format!("{}:{}", host, port)
    };
    Ok(Value::Str(Arc::from(res)))
}

pub fn net_split_host_port(
    _ctx: &mut dyn tsn_types::Context,
    args: &[Value],
) -> Result<Value, String> {
    let input = args.get(0).map(|v| v.to_string()).unwrap_or_default();
    let (host, port) = if input.starts_with('[') {
        let end = input.find(']').ok_or("Net.splitHostPort: invalid IPv6")?;
        let host = &input[1..end];
        let rest = &input[end + 1..];
        let port = rest.strip_prefix(':').unwrap_or("");
        (host, port)
    } else {
        let colon = input.rfind(':').ok_or("Net.splitHostPort: missing colon")?;
        (&input[..colon], &input[colon + 1..])
    };
    Ok(new_array(vec![
        Value::Str(Arc::from(host)),
        Value::Str(Arc::from(port)),
    ]))
}

pub fn net_parse_url(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let s = args.get(0).map(|v| v.to_string()).unwrap_or_default();
    let url = Url::parse(&s).map_err(|e| format!("Net.parseURL: {}", e))?;
    Ok(url_to_value(&url))
}

pub fn net_resolve_url(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let base = args.get(0).map(|v| v.to_string()).unwrap_or_default();
    let next = args.get(1).map(|v| v.to_string()).unwrap_or_default();
    let b = Url::parse(&base).map_err(|e| format!("Net.resolveURL (base): {}", e))?;
    let res = b
        .join(&next)
        .map_err(|e| format!("Net.resolveURL: {}", e))?;
    Ok(Value::Str(Arc::from(res.to_string())))
}

pub fn net_parse_query(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let s = args.get(0).map(|v| v.to_string()).unwrap_or_default();
    let s = s.trim_start_matches('?');
    let mut data = ObjData::new();
    for (k, v) in url::form_urlencoded::parse(s.as_bytes()) {
        data.fields
            .insert(Arc::from(k.as_ref()), Value::Str(Arc::from(v.into_owned())));
    }
    Ok(new_object(data))
}

pub fn net_build_query(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let q = args.get(0).ok_or("Net.buildQuery: missing argument")?;
    let pairs = to_kv_pairs(q)?;
    let mut ser = url::form_urlencoded::Serializer::new(String::new());
    for (k, v) in pairs {
        ser.append_pair(&k, &v);
    }
    Ok(Value::Str(Arc::from(ser.finish())))
}

pub fn net_append_query(
    _ctx: &mut dyn tsn_types::Context,
    args: &[Value],
) -> Result<Value, String> {
    let s = args.get(0).map(|v| v.to_string()).unwrap_or_default();
    let q = args.get(1).ok_or("Net.appendQuery: missing query")?;
    let mut url = Url::parse(&s).map_err(|e| format!("Net.appendQuery: {}", e))?;
    let pairs = to_kv_pairs(q)?;
    {
        let mut qp = url.query_pairs_mut();
        for (k, v) in pairs {
            qp.append_pair(&k, &v);
        }
    }
    Ok(Value::Str(Arc::from(url.to_string())))
}

pub fn net_enc_uri_component(
    _ctx: &mut dyn tsn_types::Context,
    args: &[Value],
) -> Result<Value, String> {
    let s = args.get(0).map(|v| v.to_string()).unwrap_or_default();
    Ok(Value::Str(Arc::from(urlencoding::encode(&s).into_owned())))
}

pub fn net_dec_uri_component(
    _ctx: &mut dyn tsn_types::Context,
    args: &[Value],
) -> Result<Value, String> {
    let s = args.get(0).map(|v| v.to_string()).unwrap_or_default();
    let decoded = urlencoding::decode(&s).map_err(|e| format!("Net.decURI: {}", e))?;
    Ok(Value::Str(Arc::from(decoded.into_owned())))
}

pub fn net_basic_auth(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let user = args.get(0).map(|v| v.to_string()).unwrap_or_default();
    let pass = args.get(1).map(|v| v.to_string()).unwrap_or_default();
    let auth = format!("{}:{}", user, pass);
    let enc = STANDARD.encode(auth.as_bytes());
    Ok(Value::Str(Arc::from(format!("Basic {}", enc))))
}

// Helpers
fn url_to_value(url: &Url) -> Value {
    let mut obj = ObjData::new();
    obj.fields
        .insert(Arc::from("href"), Value::Str(Arc::from(url.as_str())));
    obj.fields.insert(
        Arc::from("protocol"),
        Value::Str(Arc::from(format!("{}:", url.scheme()))),
    );
    obj.fields.insert(
        Arc::from("hostname"),
        Value::Str(Arc::from(url.host_str().unwrap_or(""))),
    );
    obj.fields.insert(
        Arc::from("port"),
        match url.port() {
            Some(p) => Value::Int(p as i64),
            None => Value::Null,
        },
    );
    obj.fields
        .insert(Arc::from("pathname"), Value::Str(Arc::from(url.path())));
    obj.fields.insert(
        Arc::from("search"),
        Value::Str(Arc::from(
            url.query().map(|q| format!("?{}", q)).unwrap_or_default(),
        )),
    );
    obj.fields.insert(
        Arc::from("hash"),
        Value::Str(Arc::from(
            url.fragment()
                .map(|f| format!("#{}", f))
                .unwrap_or_default(),
        )),
    );
    new_object(obj)
}

fn to_kv_pairs(v: &Value) -> Result<Vec<(String, String)>, String> {
    match v {
        Value::Object(o) => {
            let mut res = Vec::new();
            // Safety: standard fields iteration
            for (k, v) in unsafe { &**o }.fields.iter() {
                res.push((k.to_string(), v.to_string()));
            }
            Ok(res)
        }
        _ => Err("expected object for query params".into()),
    }
}
