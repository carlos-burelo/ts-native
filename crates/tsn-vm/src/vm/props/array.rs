use crate::tsn_types::value::Value;
use tsn_types::value::ArrayRef;
use tsn_runtime::modules::array as rt_array;

pub(super) fn get_property(obj: &Value, arr: &ArrayRef, key: &str) -> Result<Value, String> {
    if key == "length" {
        return Ok(Value::Int(unsafe { &**arr }.len() as i64));
    }
    if let Ok(n) = key.parse::<usize>() {
        return unsafe { &**arr }
            .get(n)
            .cloned()
            .ok_or_else(|| format!("index {} out of bounds for array", n));
    }

    let (method, name) = match key {
        "push" => (rt_array::array_push as _, "push"),
        "pop" => (rt_array::array_pop as _, "pop"),
        "shift" => (rt_array::array_shift as _, "shift"),
        "unshift" => (rt_array::array_unshift as _, "unshift"),
        "join" => (rt_array::array_join as _, "join"),
        "slice" => (rt_array::array_slice as _, "slice"),
        "at" => (rt_array::array_at as _, "at"),
        "concat" => (rt_array::array_concat as _, "concat"),
        "fill" => (rt_array::array_fill as _, "fill"),
        "filter" => (rt_array::array_filter as _, "filter"),
        "find" => (rt_array::array_find as _, "find"),
        "findIndex" => (rt_array::array_find_index as _, "findIndex"),
        "flat" => (rt_array::array_flat as _, "flat"),
        "flatMap" => (rt_array::array_flat_map as _, "flatMap"),
        "forEach" => (rt_array::array_for_each as _, "forEach"),
        "map" => (rt_array::array_map as _, "map"),
        "reduce" => (rt_array::array_reduce as _, "reduce"),
        "reverse" => (rt_array::array_reverse as _, "reverse"),
        "sort" => (rt_array::array_sort as _, "sort"),
        "splice" => (rt_array::array_splice as _, "splice"),
        "every" => (rt_array::array_every as _, "every"),
        "some" => (rt_array::array_some as _, "some"),
        "includes" => (rt_array::array_includes as _, "includes"),
        "indexOf" => (rt_array::array_index_of as _, "indexOf"),
        _ => return Err(format!("method '{}' not found on array", key)),
    };

    Ok(Value::native_bound(obj.clone(), method, name))
}
