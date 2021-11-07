use std::collections::{HashMap, HashSet, hash_map::Entry};
use std::env;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use lazy_static::lazy_static;
use log::warn;
use patricia_tree::{node::Node, PatriciaMap};
use regex::Regex;
use serde::Deserialize;
use serde_json::{json, Value as JSONValue};

struct Index {
    field_ids: HashMap<String, usize>,
    document_ids: serde_json::Map<String, JSONValue>,
    next_id: usize,
    /* {fieldId: count} */
    field_num_tokens: HashMap<usize, usize>,
    /* {documentId: {fieldId: count} } */
    field_length: HashMap<usize, HashMap<usize, usize>>,
    map: PatriciaMap<Vec<(usize, usize)>>,
    // TODO: custom tokenizer
    // TODO: custom term processing
}

fn process_term(term: &str) -> String {
    term.to_lowercase()
}

fn tokenize(text: &str) -> Vec<String> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"[\n\r -#%-*,-/:;?@\[-\]_{}\u00A0\u00A1\u00A7\u00AB\u00B6\u00B7\u00BB\u00BF\u037E\u0387\u055A-\u055F\u0589\u058A\u05BE\u05C0\u05C3\u05C6\u05F3\u05F4\u0609\u060A\u060C\u060D\u061B\u061E\u061F\u066A-\u066D\u06D4\u0700-\u070D\u07F7-\u07F9\u0830-\u083E\u085E\u0964\u0965\u0970\u09FD\u0A76\u0AF0\u0C77\u0C84\u0DF4\u0E4F\u0E5A\u0E5B\u0F04-\u0F12\u0F14\u0F3A-\u0F3D\u0F85\u0FD0-\u0FD4\u0FD9\u0FDA\u104A-\u104F\u10FB\u1360-\u1368\u1400\u166E\u1680\u169B\u169C\u16EB-\u16ED\u1735\u1736\u17D4-\u17D6\u17D8-\u17DA\u1800-\u180A\u1944\u1945\u1A1E\u1A1F\u1AA0-\u1AA6\u1AA8-\u1AAD\u1B5A-\u1B60\u1BFC-\u1BFF\u1C3B-\u1C3F\u1C7E\u1C7F\u1CC0-\u1CC7\u1CD3\u2000-\u200A\u2010-\u2029\u202F-\u2043\u2045-\u2051\u2053-\u205F\u207D\u207E\u208D\u208E\u2308-\u230B\u2329\u232A\u2768-\u2775\u27C5\u27C6\u27E6-\u27EF\u2983-\u2998\u29D8-\u29DB\u29FC\u29FD\u2CF9-\u2CFC\u2CFE\u2CFF\u2D70\u2E00-\u2E2E\u2E30-\u2E4F\u3000-\u3003\u3008-\u3011\u3014-\u301F\u3030\u303D\u30A0\u30FB\uA4FE\uA4FF\uA60D-\uA60F\uA673\uA67E\uA6F2-\uA6F7\uA874-\uA877\uA8CE\uA8CF\uA8F8-\uA8FA\uA8FC\uA92E\uA92F\uA95F\uA9C1-\uA9CD\uA9DE\uA9DF\uAA5C-\uAA5F\uAADE\uAADF\uAAF0\uAAF1\uABEB\uFD3E\uFD3F\uFE10-\uFE19\uFE30-\uFE52\uFE54-\uFE61\uFE63\uFE68\uFE6A\uFE6B\uFF01-\uFF03\uFF05-\uFF0A\uFF0C-\uFF0F\uFF1A\uFF1B\uFF1F\uFF20\uFF3B-\uFF3D\uFF3F\uFF5B\uFF5D\uFF5F-\uFF65]+").unwrap();
    }
    // TODO: do not collect
    RE.split(text).map(|x| x.to_owned()).collect()
}

fn get_document_tokens(
    field_ids: &HashMap<String, usize>,
    document: &HashMap<String, String>,
    document_id: usize,
) -> Vec<(String, usize, usize)> {
    field_ids
        .iter()
        .flat_map(|(field_name, field_id)| {
            let default = &"".to_owned();
            let text = document.get(field_name).unwrap_or(default);
            let tokens = tokenize(&text);
            tokens
                .into_iter()
                .map(|x| (x, *field_id, document_id.to_owned()))
        })
        .collect()
}

impl Index {
    fn new(config: &IndexConfig) -> Self {
        let field_ids = config
            .fields
            .iter()
            .enumerate()
            .map(|(i, v)| (v.to_owned(), i))
            .collect::<HashMap<String, usize>>();
        Index {
            field_ids,
            document_ids: serde_json::Map::new(),
            field_num_tokens: HashMap::new(),
            field_length: HashMap::new(),
            next_id: 0,
            map: PatriciaMap::new(),
        }
    }

    fn insert_document(&mut self, doc: &str) -> usize {
        let small_id = self.next_id;
        self.document_ids.insert(small_id.to_string(), doc.into());
        self.next_id += 1;
        return small_id;
    }

    fn add_document_tokens<I>(&mut self, document_tokens: I)
    where
        I: Iterator<Item = (String, usize, usize)>,
    {
        for (token, field_id, small_id) in document_tokens {
            let num_tokens = self.field_num_tokens.get(&field_id).unwrap_or(&0) + 1;
            self.field_num_tokens.insert(field_id, num_tokens);

            let default_document_fields_length = HashMap::new();
            let mut document_fields_length = self.field_length.remove(&small_id).unwrap_or(default_document_fields_length);
            let num_document_field_length = document_fields_length.get(&field_id).unwrap_or(&0) + 1;
            document_fields_length.insert(field_id, num_document_field_length);
            self.field_length.insert(small_id, document_fields_length);

            self.field_num_tokens.insert(field_id, num_tokens);
            self.add_token(small_id, &process_term(&token), field_id);
        }
    }

    fn add_token(&mut self, document_id: usize, token: &str, field_id: usize) {
        // conditional double insert sounds more efficient than get-insert
        let old = self
            .map
            .insert(token, vec![(document_id.to_owned(), field_id)]);
        if let Some(mut old) = old {
            old.push((document_id.to_owned(), field_id));
            self.map.insert(token, old);
        }
    }

    fn into_minisearch_json(self) -> String {
        let mut h = serde_json::Map::new();
        h.insert("documentCount".to_string(), self.next_id.into());
        h.insert("nextId".to_string(), self.next_id.into());

        h.insert("documentIds".to_string(), self.document_ids.into());

        let mut field_ids = serde_json::Map::new();
        for (k, v) in self.field_ids.into_iter() {
            field_ids.insert(k.to_string(), v.into());
        }
        h.insert("fieldIds".to_string(), field_ids.into());

        let mut average_field_length = serde_json::Map::new();
        for (field_id, num_tokens) in self.field_num_tokens.into_iter() {
            average_field_length.insert(field_id.to_string(), (num_tokens as f64 / self.next_id as f64).into());
        }
        h.insert("averageFieldLength".to_string(), average_field_length.into());

        let mut field_length = serde_json::Map::new();
        for (small_id, field_lengths) in self.field_length.into_iter() {
            let mut json_field_lengths = serde_json::Map::new();
            for (field_id, length) in field_lengths.into_iter() {
                json_field_lengths.insert(field_id.to_string(), length.into());
            }
            field_length.insert(small_id.to_string(), json_field_lengths.into());
        }
        h.insert("fieldLength".to_string(), field_length.into());

        // TODO: storedFields

        let node = Node::from(self.map);

        let mut index = serde_json::Map::new();
        index.insert("_prefix".to_string(), "".into());
        let mut stack = vec![("".to_owned(), serde_json::Map::new())];
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
            let mut val = serde_json::Map::new();
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
                            Entry::Vacant(v) => { v.insert(1); },
                        };
                    }
                    for (small_id, counts) in tree.into_iter() {
                        let df = counts.len();
                        let mut ds = serde_json::Map::new();
                        for (field_id, count) in counts.into_iter() {
                            ds.insert(field_id.to_string(), count.into());
                        }
                        val.insert("".to_string(), json!({
                            small_id.to_string(): {
                                "df": df,
                                "ds": ds,
                            }
                        }).into());
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
        h.insert("index".to_string(), index.into());

        return serde_json::to_string(&JSONValue::Object(h)).unwrap();
    }
}

#[derive(Deserialize, Debug)]
struct IndexConfig {
    fields: Vec<String>,
    #[serde(alias = "storeFields")]
    store_fields: Vec<String>,
}

fn json_document_to_text_document(
    json_document: HashMap<String, JSONValue>,
    fields: &HashSet<String>,
) -> HashMap<String, String> {
    json_document
        .into_iter()
        .filter_map(|(k, v)| {
            if k != "id" && !fields.contains(&k) {
                return None;
            }
            match v {
                JSONValue::Null => Some((k, "".to_owned())),
                JSONValue::Number(ref n) => Some((k, n.to_string())),
                JSONValue::String(ref s) => Some((k, s.clone())),
                _ => {
                    warn!("unsupported type for field {}", k);
                    None
                }
            }
        })
        .collect()
}

fn read_config_from_file<P: AsRef<Path>>(path: P) -> IndexConfig {
    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);
    serde_json::from_reader(reader).unwrap()
}

fn get_path_documents(path: &str) -> Vec<HashMap<String, JSONValue>> {
    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);
    serde_json::from_reader(reader).unwrap()
}

fn main() {
    let args = env::args().collect::<Vec<_>>();
    let config_path = &args[1];
    let config = read_config_from_file(config_path);
    let data_path = &args[2];

    let mut index = Index::new(&config);
    let fields = index.field_ids.keys().cloned().collect();
    let field_ids = index.field_ids.clone();

    let docs = get_path_documents(data_path)
        .into_iter()
        .map(|doc| json_document_to_text_document(doc, &fields))
        .map(|doc| {
            let id = doc.get("id").unwrap().clone();
            (doc, index.insert_document(&id))
        })
        .flat_map(|(doc, id)| get_document_tokens(&field_ids, &doc, id))
        .collect::<Vec<_>>();

    index.add_document_tokens(docs.into_iter());
    println!("{}", index.into_minisearch_json());
}
