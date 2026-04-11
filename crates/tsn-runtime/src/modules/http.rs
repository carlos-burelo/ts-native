use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use tsn_types::{
    future::AsyncFuture,
    value::{new_object, ObjData},
    NativeFn, Value,
};

type RouteEntry = (String, String, Value);

static SERVER_ROUTES: OnceLock<Mutex<HashMap<i64, Vec<RouteEntry>>>> = OnceLock::new();
static NEXT_SERVER_ID: AtomicI64 = AtomicI64::new(1);

pub fn routes_map() -> &'static Mutex<HashMap<i64, Vec<RouteEntry>>> {
    SERVER_ROUTES.get_or_init(|| Mutex::new(HashMap::new()))
}

struct PendingResponse {
    status: u16,
    body: String,
    content_type: String,
    extra_headers: Vec<(String, String)>,
}

thread_local! {
    static RESPONSE_SLOT: RefCell<Option<PendingResponse>> = const { RefCell::new(None) };
}

pub fn http_fetch(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let url = args.first().map(|v| v.to_string()).unwrap_or_default();
    let method = args.get(1).map(|v| v.to_string()).unwrap_or_else(|| "GET".into());
    let headers = extract_obj_pairs(args.get(2));
    let body = match args.get(3) {
        Some(Value::Null) | None => None,
        Some(v) => Some(v.to_string()),
    };
    let timeout_ms = match args.get(4) {
        Some(Value::Int(i)) => *i as u64,
        _ => 30_000,
    };

    let vtable = tsn_types::value::get_global_vtable().ok_or("http_fetch: heap not initialized")?;
    let fut = AsyncFuture::pending();
    let fut2 = fut.clone();

    std::thread::spawn(move || {
        tsn_types::value::install_allocator(vtable);
        match do_fetch(&url, &method, &headers, body.as_deref(), timeout_ms) {
            Ok(v) => fut2.resolve(v),
            Err(e) => fut2.reject_msg(e),
        }
    });

    Ok(Value::Future(fut))
}

pub fn http_server_create(_ctx: &mut dyn tsn_types::Context, _args: &[Value]) -> Result<Value, String> {
    let id = NEXT_SERVER_ID.fetch_add(1, Ordering::SeqCst);
    routes_map().lock().unwrap().insert(id, Vec::new());
    Ok(Value::Int(id))
}

pub fn http_server_route(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let id = match args.first() {
        Some(Value::Int(i)) => *i,
        _ => return Err("server_route: expected server id".into()),
    };
    let method = args.get(1).map(|v| v.to_string().to_uppercase()).unwrap_or_else(|| "GET".into());
    let pattern = args.get(2).map(|v| v.to_string()).unwrap_or_default();
    let cb = args.get(3).cloned().ok_or("server_route: missing callback")?;
    routes_map().lock().unwrap().entry(id).or_default().push((method, pattern, cb));
    Ok(Value::Null)
}

pub fn http_server_listen(ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let id = match args.first() {
        Some(Value::Int(i)) => *i,
        _ => return Err("server_listen: expected server id".into()),
    };
    let port = match args.get(1) {
        Some(Value::Int(i)) => *i as u16,
        _ => return Err("server_listen: expected port".into()),
    };
    let res_ctor = args.get(2).cloned().ok_or("server_listen: missing ServerResponse ctor")?;

    let listener = tiny_http::Server::http(format!("0.0.0.0:{}", port))
        .map_err(|e| format!("server_listen: {}", e))?;

    println!("Server listening on http://0.0.0.0:{}", port);

    loop {
        let mut raw = match listener.recv() {
            Ok(r) => r,
            Err(e) => { eprintln!("server recv error: {}", e); continue; }
        };

        let method = raw.method().to_string().to_uppercase();
        let full_url = raw.url().to_owned();
        let (path, query_str) = split_path_query(&full_url);
        let mut body_buf = String::new();
        let _ = std::io::Read::read_to_string(raw.as_reader(), &mut body_buf);

        let routes = routes_map().lock().unwrap().get(&id).cloned().unwrap_or_default();
        let matched = find_route(&routes, &method, &path);
        let query_obj = parse_query_string(&query_str);
        let params_obj = matched.as_ref()
            .map(|(_, pattern, _)| extract_params(pattern, &path))
            .unwrap_or_else(|| new_object(ObjData::new()));
        let req_headers = build_headers_obj(raw.headers());
        let req_obj = make_request_obj(&method, &path, query_obj, params_obj, &body_buf, req_headers);

        RESPONSE_SLOT.with(|s| *s.borrow_mut() = None);

        let res_obj = match ctx.call(res_ctor.clone(), &[]) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("ServerResponse ctor failed: {}", e);
                let h = tiny_http::Header::from_bytes("Content-Type".as_bytes(), "text/plain".as_bytes()).unwrap();
                let _ = raw.respond(tiny_http::Response::from_string("Internal Server Error").with_status_code(500).with_header(h));
                continue;
            }
        };

        let cb_result = if let Some((_, _, cb)) = matched {
            ctx.call(cb, &[req_obj, res_obj])
        } else {
            RESPONSE_SLOT.with(|s| {
                *s.borrow_mut() = Some(PendingResponse { status: 404, body: "Not Found".into(), content_type: "text/plain".into(), extra_headers: vec![] });
            });
            Ok(Value::Null)
        };

        if let Err(e) = cb_result {
            RESPONSE_SLOT.with(|s| {
                *s.borrow_mut() = Some(PendingResponse { status: 500, body: e, content_type: "text/plain".into(), extra_headers: vec![] });
            });
        }

        let pending = RESPONSE_SLOT.with(|s| s.borrow_mut().take());
        let (status, body, ct, extra) = match pending {
            Some(p) => (p.status, p.body, p.content_type, p.extra_headers),
            None => (204, String::new(), "text/plain".into(), vec![]),
        };

        let ct_header = tiny_http::Header::from_bytes("Content-Type".as_bytes(), ct.as_bytes()).unwrap();
        let mut response = tiny_http::Response::from_string(body).with_status_code(status).with_header(ct_header);
        for (k, v) in extra {
            if let Ok(h) = tiny_http::Header::from_bytes(k.as_bytes(), v.as_bytes()) {
                response.add_header(h);
            }
        }
        let _ = raw.respond(response);
    }
}

pub fn http_response_send(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    RESPONSE_SLOT.with(|s| {
        let mut slot = s.borrow_mut();
        if slot.is_none() {
            let status = match args.first() { Some(Value::Int(i)) => *i as u16, _ => 200 };
            let body = args.get(1).map(|v| v.to_string()).unwrap_or_default();
            let ct = args.get(2).map(|v| v.to_string()).unwrap_or_else(|| "text/plain".into());
            let extra = extract_obj_pairs(args.get(3));
            *slot = Some(PendingResponse { status, body, content_type: ct, extra_headers: extra });
        }
    });
    Ok(Value::Null)
}

pub fn do_fetch(url: &str, method: &str, headers: &[(String, String)], body: Option<&str>, timeout_ms: u64) -> Result<Value, String> {
    let agent = ureq::AgentBuilder::new().timeout(std::time::Duration::from_millis(timeout_ms)).build();
    let mut req = agent.request(method, url);
    for (k, v) in headers { req = req.set(k, v); }

    let (status, status_text, body_text, resp_headers) = match if let Some(b) = body {
        req.send_string(b)
    } else {
        req.call()
    } {
        Ok(resp) => {
            let s = resp.status();
            let st = resp.status_text().to_owned();
            let rh = resp.headers_names().iter()
                .filter_map(|n| resp.header(n).map(|v| (n.clone(), v.to_owned())))
                .collect::<Vec<_>>();
            let b = resp.into_string().map_err(|e| e.to_string())?;
            (s, st, b, rh)
        }
        Err(ureq::Error::Status(code, resp)) => {
            let st = resp.status_text().to_owned();
            let rh = resp.headers_names().iter()
                .filter_map(|n| resp.header(n).map(|v| (n.clone(), v.to_owned())))
                .collect::<Vec<_>>();
            let b = resp.into_string().unwrap_or_default();
            (code, st, b, rh)
        }
        Err(e) => return Err(e.to_string()),
    };

    let mut headers_obj = ObjData::new();
    for (k, v) in resp_headers {
        headers_obj.fields.insert(Arc::from(k.as_str()), Value::Str(Arc::from(v.as_str())));
    }
    let mut obj = ObjData::new();
    obj.fields.insert(Arc::from("status"),     Value::Int(status as i64));
    obj.fields.insert(Arc::from("statusText"), Value::Str(Arc::from(status_text.as_str())));
    obj.fields.insert(Arc::from("ok"),         Value::Bool(status >= 200 && status < 300));
    obj.fields.insert(Arc::from("body"),       Value::Str(Arc::from(body_text.as_str())));
    obj.fields.insert(Arc::from("headers"),    new_object(headers_obj));
    Ok(new_object(obj))
}

pub fn extract_obj_pairs(v: Option<&Value>) -> Vec<(String, String)> {
    match v {
        Some(Value::Object(o)) => unsafe { &**o }.fields.iter()
            .filter(|(_, v)| !matches!(v, Value::Null))
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        _ => vec![],
    }
}

pub fn split_path_query(url: &str) -> (String, String) {
    match url.find('?') {
        Some(i) => (url[..i].to_owned(), url[i + 1..].to_owned()),
        None => (url.to_owned(), String::new()),
    }
}

pub fn parse_query_string(qs: &str) -> Value {
    let mut obj = ObjData::new();
    for pair in qs.split('&').filter(|s| !s.is_empty()) {
        let (k, v) = match pair.find('=') {
            Some(i) => (&pair[..i], &pair[i + 1..]),
            None => (pair, ""),
        };
        let key = urlencoding::decode(k).map(|s| s.into_owned()).unwrap_or_else(|_| k.to_owned());
        let val = urlencoding::decode(v).map(|s| s.into_owned()).unwrap_or_else(|_| v.to_owned());
        obj.fields.insert(Arc::from(key.as_str()), Value::Str(Arc::from(val.as_str())));
    }
    new_object(obj)
}

pub fn match_pattern(pattern: &str, path: &str) -> Option<Vec<(String, String)>> {
    let pp: Vec<&str> = pattern.split('/').collect();
    let ps: Vec<&str> = path.split('/').collect();
    if pp.len() != ps.len() { return None; }
    let mut params = Vec::new();
    for (seg, val) in pp.iter().zip(ps.iter()) {
        if let Some(name) = seg.strip_prefix(':') {
            params.push((name.to_owned(), (*val).to_owned()));
        } else if seg != val {
            return None;
        }
    }
    Some(params)
}

pub fn find_route(routes: &[RouteEntry], method: &str, path: &str) -> Option<(String, String, Value)> {
    for (m, pattern, cb) in routes {
        if m == method && match_pattern(pattern, path).is_some() {
            return Some((m.clone(), pattern.clone(), cb.clone()));
        }
    }
    None
}

pub fn extract_params(pattern: &str, path: &str) -> Value {
    let mut obj = ObjData::new();
    if let Some(pairs) = match_pattern(pattern, path) {
        for (k, v) in pairs {
            obj.fields.insert(Arc::from(k.as_str()), Value::Str(Arc::from(v.as_str())));
        }
    }
    new_object(obj)
}

pub fn build_headers_obj(headers: &[tiny_http::Header]) -> Value {
    let mut obj = ObjData::new();
    for h in headers {
        let name = h.field.to_string().to_lowercase();
        let val = h.value.to_string();
        obj.fields.insert(Arc::from(name.as_str()), Value::Str(Arc::from(val.as_str())));
    }
    new_object(obj)
}

pub fn make_request_obj(method: &str, path: &str, query: Value, params: Value, body: &str, headers: Value) -> Value {
    let mut obj = ObjData::new();
    obj.fields.insert(Arc::from("method"),  Value::Str(Arc::from(method)));
    obj.fields.insert(Arc::from("path"),    Value::Str(Arc::from(path)));
    obj.fields.insert(Arc::from("query"),   query);
    obj.fields.insert(Arc::from("params"),  params);
    obj.fields.insert(Arc::from("body"),    Value::Str(Arc::from(body)));
    obj.fields.insert(Arc::from("headers"), headers);
    new_object(obj)
}

pub fn build() -> Value {
    let mut ns = ObjData::new();
    ns.set_field(Arc::from("fetch"),        Value::NativeFn(Box::new((http_fetch          as NativeFn, "fetch"))));
    ns.set_field(Arc::from("createServer"), Value::NativeFn(Box::new((http_server_create  as NativeFn, "createServer"))));
    ns.set_field(Arc::from("addRoute"),     Value::NativeFn(Box::new((http_server_route   as NativeFn, "addRoute"))));
    ns.set_field(Arc::from("listen"),       Value::NativeFn(Box::new((http_server_listen  as NativeFn, "listen"))));
    ns.set_field(Arc::from("sendResponse"), Value::NativeFn(Box::new((http_response_send  as NativeFn, "sendResponse"))));

    let mut exports = ObjData::new();
    exports.set_field(Arc::from("Http"), new_object(ns));
    new_object(exports)
}
