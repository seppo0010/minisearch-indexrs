use std::collections::{hash_map::Entry, HashMap};

use patricia_tree::{node::Node, PatriciaMap};
use serde_json::{json, Map as JSONMap, Value as JSONValue};

pub fn field_ids_json(field_ids_src: HashMap<String, usize>) -> JSONMap<String, JSONValue> {
    let mut field_ids = JSONMap::new();
    for (k, v) in field_ids_src.into_iter() {
        field_ids.insert(k.to_string(), v.into());
    }
    return field_ids;
}

pub fn average_field_length_json(
    field_num_tokens: HashMap<usize, usize>,
    next_id: f64,
) -> JSONMap<String, JSONValue> {
    let mut average_field_length = JSONMap::new();
    for (field_id, num_tokens) in field_num_tokens.into_iter() {
        average_field_length.insert(field_id.to_string(), (num_tokens as f64 / next_id).into());
    }
    average_field_length
}

pub fn field_length_json(
    field_length_src: HashMap<usize, HashMap<usize, usize>>,
) -> JSONMap<String, JSONValue> {
    let mut field_length = JSONMap::new();
    for (small_id, field_lengths) in field_length_src.into_iter() {
        let mut json_field_lengths = JSONMap::new();
        for (field_id, length) in field_lengths.into_iter() {
            json_field_lengths.insert(field_id.to_string(), length.into());
        }
        field_length.insert(small_id.to_string(), json_field_lengths.into());
    }
    field_length
}

pub fn map_json(map: PatriciaMap<Vec<(usize, usize)>>) -> JSONMap<String, JSONValue> {
    let node = Node::from(map);

    let mut index = JSONMap::new();
    index.insert("_prefix".to_string(), "".into());
    let mut stack = vec![("".to_owned(), JSONMap::new())];
    for (level, node) in node.into_iter() {
        let label = std::str::from_utf8(node.label()).unwrap().to_owned();
        if level == 0 {
            continue;
        }
        while level + 1 <= stack.len() {
            let level = stack.len() - 2;
            let (key, val) = stack.pop().unwrap();
            stack[level].1.insert(key, val.into());
        }
        let mut val = JSONMap::new();
        if level + 1 > stack.len() {
            if let Some(nodes) = node.value() {
                let mut tree = HashMap::new();
                for (small_id, field_id) in nodes {
                    let subtree = match tree.entry(field_id) {
                        Entry::Occupied(o) => o.into_mut(),
                        Entry::Vacant(v) => v.insert(HashMap::<usize, usize>::new()),
                    };
                    match subtree.entry(*small_id) {
                        Entry::Occupied(o) => *o.into_mut() += 1,
                        Entry::Vacant(v) => {
                            v.insert(1);
                        }
                    };
                }
                for (small_id, counts) in tree.into_iter() {
                    let df = counts.len();
                    let mut ds = JSONMap::new();
                    for (field_id, count) in counts.into_iter() {
                        ds.insert(field_id.to_string(), count.into());
                    }
                    val.insert(
                        "".to_string(),
                        json!({
                            small_id.to_string(): {
                                "df": df,
                                "ds": ds,
                            }
                        })
                        .into(),
                    );
                }
            }
            stack.push((label, val));
        }
    }
    while stack.len() > 1 {
        let level = stack.len() - 2;
        let (key, val) = stack.pop().unwrap();
        stack[level].1.insert(key, val.into());
    }
    index.insert("_tree".to_string(), stack.pop().unwrap().1.into());
    index
}
