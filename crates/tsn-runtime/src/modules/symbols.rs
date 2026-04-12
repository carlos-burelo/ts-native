use tsn_types::Value;

pub fn symbol_iterator(
    _ctx: &mut dyn tsn_types::Context,
    _args: &[Value],
) -> Result<Value, String> {
    Ok(Value::Symbol(tsn_types::value::SymbolKind::Iterator))
}

pub fn symbol_async_iterator(
    _ctx: &mut dyn tsn_types::Context,
    _args: &[Value],
) -> Result<Value, String> {
    Ok(Value::Symbol(tsn_types::value::SymbolKind::AsyncIterator))
}
