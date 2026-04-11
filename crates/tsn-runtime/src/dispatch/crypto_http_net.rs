use super::entry::{op, DispatchEntry};
use crate::modules::{crypto, http, net};
use tsn_core::intrinsic::IntrinsicId;

pub(crate) static OPS: &[DispatchEntry] = &[
    op(IntrinsicId::CryptoSha256, "crypto_sha256", crypto::crypto_sha256),
    op(IntrinsicId::CryptoSha512, "crypto_sha512", crypto::crypto_sha512),
    op(IntrinsicId::CryptoRandomBytes, "crypto_random_bytes", crypto::crypto_random_bytes),
    op(IntrinsicId::CryptoRandomHex, "crypto_random_hex", crypto::crypto_random_hex),
    op(IntrinsicId::CryptoBase64Enc, "crypto_base64_enc", crypto::crypto_base64_encode),
    op(IntrinsicId::CryptoBase64Dec, "crypto_base64_dec", crypto::crypto_base64_decode),
    op(IntrinsicId::CryptoHmac, "crypto_hmac", crypto::crypto_hmac),
    op(IntrinsicId::CryptoUuid, "crypto_uuid", crypto::crypto_uuid),
    op(IntrinsicId::NetIsIp, "net_is_ip", net::net_is_ip),
    op(IntrinsicId::NetIsIpv4, "net_is_ipv4", net::net_is_ipv4),
    op(IntrinsicId::NetIsIpv6, "net_is_ipv6", net::net_is_ipv6),
    op(IntrinsicId::NetJoinHostPort, "net_join_host_port", net::net_join_host_port),
    op(IntrinsicId::NetSplitHostPort, "net_split_host_port", net::net_split_host_port),
    op(IntrinsicId::NetParseUrl, "net_parse_url", net::net_parse_url),
    op(IntrinsicId::NetResolveUrl, "net_resolve_url", net::net_resolve_url),
    op(IntrinsicId::NetParseQuery, "net_parse_query", net::net_parse_query),
    op(IntrinsicId::NetBuildQuery, "net_build_query", net::net_build_query),
    op(IntrinsicId::NetAppendQuery, "net_append_query", net::net_append_query),
    op(IntrinsicId::NetEncUriComponent, "net_enc_uri_component", net::net_encode_uri),
    op(IntrinsicId::NetDecUriComponent, "net_dec_uri_component", net::net_decode_uri),
    op(IntrinsicId::NetBasicAuth, "net_basic_auth", net::net_basic_auth),
    op(IntrinsicId::HttpFetch, "http_fetch", http::http_fetch),
    op(IntrinsicId::HttpServerCreate, "http_server_create", http::http_server_create),
    op(IntrinsicId::HttpServerRoute, "http_server_route", http::http_server_route),
    op(IntrinsicId::HttpServerListen, "http_server_listen", http::http_server_listen),
    op(IntrinsicId::HttpResponseSend, "http_response_send", http::http_response_send),
];