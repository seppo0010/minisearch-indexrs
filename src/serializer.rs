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

pub fn map_json(map: PatriciaMap<Vec<(usize, usize)>>) -> Result<JSONMap<String, JSONValue>, failure::Error> {
    let node = Node::from(map);

    let mut index = JSONMap::new();
    index.insert("_prefix".to_string(), "".into());
    let mut stack = vec![("".to_owned(), JSONMap::new())];
    for (level, node) in node.into_iter() {
        let label = std::str::from_utf8(node.label())?.to_owned();
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
    Ok(index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::array::IntoIter;
    use serde_test::{assert_tokens, Token};
    use assert_json_diff::assert_json_eq;

    #[test]
    fn test_serialize_fields() {
        let mut field_ids = HashMap::new();
        field_ids.insert("title".to_owned(), 1usize);
        field_ids.insert("author".to_owned(), 2usize);
        field_ids.insert("year".to_owned(), 3usize);
        let s = field_ids_json(field_ids);
        assert_tokens(
            &s,
            &[
                Token::Map { len: Some(3) },
                Token::Str("author"),
                Token::U64(2),
                Token::Str("title"),
                Token::U64(1),
                Token::Str("year"),
                Token::U64(3),
                Token::MapEnd,
            ],
        );
    }

    #[test]
    fn test_average_field_length_json() {
        let mut field_num_tokens = HashMap::new();
        let next_id = 100f64;
        field_num_tokens.insert(100, 18);
        field_num_tokens.insert(101, 123);
        let json = average_field_length_json(field_num_tokens, next_id);
        assert_tokens(
            &json,
            &[
                Token::Map { len: Some(2) },
                Token::Str("100"),
                Token::F64(0.18),
                Token::Str("101"),
                Token::F64(1.23),
                Token::MapEnd,
            ],
        );
    }

    #[test]
    fn test_field_length_json() {
        let field_length_src = HashMap::<_, _>::from_iter(
            IntoIter::new([
                (1, HashMap::<_, _>::from_iter(IntoIter::new([(1, 4)]))),
                (3, HashMap::<_, _>::from_iter(IntoIter::new([(1, 5), (2, 6)]))),
            ])
        ); 
        let json = field_length_json(field_length_src);
        assert_tokens(
            &json,
            &[
                Token::Map { len: Some(2) },
                    Token::Str("1"),
                    Token::Map { len: Some(1) },
                        Token::Str("1"),
                        Token::U64(4),
                    Token::MapEnd,
                    Token::Str("3"),
                    Token::Map { len: Some(2) },
                        Token::Str("1"),
                        Token::U64(5),
                        Token::Str("2"),
                        Token::U64(6),
                    Token::MapEnd,
                Token::MapEnd,
            ],
        );
    }

    #[test]
    fn test_map_json() {
        let mut map = PatriciaMap::new();
        map.insert("harry", vec![(0, 0), (1, 0)]);
        map.insert("potter", vec![(0, 0), (1, 0)]);
        map.insert("and", vec![(0, 0), (1, 0)]);
        map.insert("the", vec![(0, 0), (1, 0)]);
        map.insert("philosopher", vec![(0, 0)]);
        map.insert("s", vec![(0, 0)]);
        map.insert("stone", vec![(0, 0)]);
        map.insert("chamber", vec![(1, 0)]);
        map.insert("of", vec![(1, 0), (2, 0)]);
        map.insert("secrets", vec![(1, 0)]);
        map.insert("homo", vec![(2, 0)]);
        map.insert("deus", vec![(2, 0)]);
        map.insert("a", vec![(2, 0), (3, 0)]);
        map.insert("history", vec![(2, 0)]);
        map.insert("tomorrow", vec![(2, 0)]);
        map.insert("to", vec![(3, 0)]);
        map.insert("kill", vec![(3, 0)]);
        map.insert("mockingbird", vec![(3, 0)]);
        map.insert("life", vec![(4, 0), (4, 0)]);
        map.insert("after", vec![(4, 0)]);
        let json = map_json(map).unwrap();
        assert_json_eq!(
            json,
            json!(
                {
                  "_prefix" : "",
                  "_tree" : {
                     "a" : {
                        "" : {
                           "0" : {
                              "df" : 2,
                              "ds" : {
                                 "2" : 1,
                                 "3" : 1
                              }
                           }
                        },
                        "fter" : {
                           "" : {
                              "0" : {
                                 "df" : 1,
                                 "ds" : {
                                    "4" : 1
                                 }
                              }
                           }
                        },
                        "nd" : {
                           "" : {
                              "0" : {
                                 "df" : 2,
                                 "ds" : {
                                    "0" : 1,
                                    "1" : 1
                                 }
                              }
                           }
                        }
                     },
                     "chamber" : {
                        "" : {
                           "0" : {
                              "df" : 1,
                              "ds" : {
                                 "1" : 1
                              }
                           }
                        }
                     },
                     "deus" : {
                        "" : {
                           "0" : {
                              "df" : 1,
                              "ds" : {
                                 "2" : 1
                              }
                           }
                        }
                     },
                     "h" : {
                        "arry" : {
                           "" : {
                              "0" : {
                                 "df" : 2,
                                 "ds" : {
                                    "0" : 1,
                                    "1" : 1
                                 }
                              }
                           }
                        },
                        "istory" : {
                           "" : {
                              "0" : {
                                 "df" : 1,
                                 "ds" : {
                                    "2" : 1
                                 }
                              }
                           }
                        },
                        "omo" : {
                           "" : {
                              "0" : {
                                 "df" : 1,
                                 "ds" : {
                                    "2" : 1
                                 }
                              }
                           }
                        }
                     },
                     "kill" : {
                        "" : {
                           "0" : {
                              "df" : 1,
                              "ds" : {
                                 "3" : 1
                              }
                           }
                        }
                     },
                     "life" : {
                        "" : {
                           "0" : {
                              "df" : 1,
                              "ds" : {
                                 "4" : 2
                              }
                           }
                        }
                     },
                     "mockingbird" : {
                        "" : {
                           "0" : {
                              "df" : 1,
                              "ds" : {
                                 "3" : 1
                              }
                           }
                        }
                     },
                     "of" : {
                        "" : {
                           "0" : {
                              "df" : 2,
                              "ds" : {
                                 "1" : 1,
                                 "2" : 1
                              }
                           }
                        }
                     },
                     "p" : {
                        "hilosopher" : {
                           "" : {
                              "0" : {
                                 "df" : 1,
                                 "ds" : {
                                    "0" : 1
                                 }
                              }
                           }
                        },
                        "otter" : {
                           "" : {
                              "0" : {
                                 "df" : 2,
                                 "ds" : {
                                    "0" : 1,
                                    "1" : 1
                                 }
                              }
                           }
                        }
                     },
                     "s" : {
                        "" : {
                           "0" : {
                              "df" : 1,
                              "ds" : {
                                 "0" : 1
                              }
                           }
                        },
                        "ecrets" : {
                           "" : {
                              "0" : {
                                 "df" : 1,
                                 "ds" : {
                                    "1" : 1
                                 }
                              }
                           }
                        },
                        "tone" : {
                           "" : {
                              "0" : {
                                 "df" : 1,
                                 "ds" : {
                                    "0" : 1
                                 }
                              }
                           }
                        }
                     },
                     "t" : {
                        "he" : {
                           "" : {
                              "0" : {
                                 "df" : 2,
                                 "ds" : {
                                    "0" : 1,
                                    "1" : 1
                                 }
                              }
                           }
                        },
                        "o" : {
                           "" : {
                              "0" : {
                                 "df" : 1,
                                 "ds" : {
                                    "3" : 1
                                 }
                              }
                           },
                           "morrow" : {
                              "" : {
                                 "0" : {
                                    "df" : 1,
                                    "ds" : {
                                       "2" : 1
                                    }
                                 }
                              }
                           }
                        }
                     }
                  }
               }
            ),
        )
    }
}
