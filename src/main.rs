use std::env;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use lazy_static::lazy_static;
use log::warn;
use regex::Regex;
use serde::Deserialize;
use serde_json::{Value as JSONValue};

type ShortId = usize;
type FieldId = usize;

enum TreeNode {
    Leaf(HashMap<ShortId, HashMap<FieldId, usize>>),
    Children(HashMap<String, TreeNode>),
}

struct Index {
    field_ids: HashMap<String, usize>,
    document_ids: HashMap<usize, String>,
    next_id: usize,
    document_count: usize,
    index: TreeNode,
    // TODO: custom tokenizer
}

fn tokenize(text: &str) -> Vec<String> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"[\n\r -#%-*,-/:;?@\[-\]_{}\u00A0\u00A1\u00A7\u00AB\u00B6\u00B7\u00BB\u00BF\u037E\u0387\u055A-\u055F\u0589\u058A\u05BE\u05C0\u05C3\u05C6\u05F3\u05F4\u0609\u060A\u060C\u060D\u061B\u061E\u061F\u066A-\u066D\u06D4\u0700-\u070D\u07F7-\u07F9\u0830-\u083E\u085E\u0964\u0965\u0970\u09FD\u0A76\u0AF0\u0C77\u0C84\u0DF4\u0E4F\u0E5A\u0E5B\u0F04-\u0F12\u0F14\u0F3A-\u0F3D\u0F85\u0FD0-\u0FD4\u0FD9\u0FDA\u104A-\u104F\u10FB\u1360-\u1368\u1400\u166E\u1680\u169B\u169C\u16EB-\u16ED\u1735\u1736\u17D4-\u17D6\u17D8-\u17DA\u1800-\u180A\u1944\u1945\u1A1E\u1A1F\u1AA0-\u1AA6\u1AA8-\u1AAD\u1B5A-\u1B60\u1BFC-\u1BFF\u1C3B-\u1C3F\u1C7E\u1C7F\u1CC0-\u1CC7\u1CD3\u2000-\u200A\u2010-\u2029\u202F-\u2043\u2045-\u2051\u2053-\u205F\u207D\u207E\u208D\u208E\u2308-\u230B\u2329\u232A\u2768-\u2775\u27C5\u27C6\u27E6-\u27EF\u2983-\u2998\u29D8-\u29DB\u29FC\u29FD\u2CF9-\u2CFC\u2CFE\u2CFF\u2D70\u2E00-\u2E2E\u2E30-\u2E4F\u3000-\u3003\u3008-\u3011\u3014-\u301F\u3030\u303D\u30A0\u30FB\uA4FE\uA4FF\uA60D-\uA60F\uA673\uA67E\uA6F2-\uA6F7\uA874-\uA877\uA8CE\uA8CF\uA8F8-\uA8FA\uA8FC\uA92E\uA92F\uA95F\uA9C1-\uA9CD\uA9DE\uA9DF\uAA5C-\uAA5F\uAADE\uAADF\uAAF0\uAAF1\uABEB\uFD3E\uFD3F\uFE10-\uFE19\uFE30-\uFE52\uFE54-\uFE61\uFE63\uFE68\uFE6A\uFE6B\uFF01-\uFF03\uFF05-\uFF0A\uFF0C-\uFF0F\uFF1A\uFF1B\uFF1F\uFF20\uFF3B-\uFF3D\uFF3F\uFF5B\uFF5D\uFF5F-\uFF65]+").unwrap();
    }
    // TODO: do not collect
    RE.split(text).map(|x| x.to_owned()).collect()
}

impl Index {
    fn new(config: &IndexConfig) -> Self {
        let field_ids = config.fields.iter().enumerate().map(|(i, v)| (v.to_owned(), i)).collect::<HashMap<String, usize>>();
        Index {
            field_ids,
            document_ids: HashMap::new(),
            next_id: 0,
            document_count: 0,
            index: TreeNode::Leaf(HashMap::new()),
        }
    }


    fn add_document(&mut self, document: HashMap<String, String>) {
        let document_id = document.get("id").unwrap();
        let field_ids = self.field_ids.clone();
        for (tokens, field_id) in field_ids.iter().map(|(field_name, field_id)| {
            let default = &"".to_owned();
            let text = document.get(field_name).unwrap_or(default);
            let tokens = tokenize(&text);
            (tokens, field_id)
        }) {
            for token in tokens {
                self.add_token(document_id, &token, *field_id)
            }
        }
    }

    fn add_token(&mut self, document_id: &str, token: &str, field_id: usize) {
        unimplemented!();
    }
}

#[derive(Deserialize, Debug)]
struct IndexConfig {
    fields: Vec<String>,
    #[serde(alias = "storeFields")]
    store_fields: Vec<String>,
}

fn json_document_to_text_document(json_document: HashMap<String, JSONValue>, fields: &HashSet<String>) -> HashMap<String, String> {
    json_document.into_iter().filter_map(|(k, v)| {
        if k != "id" && !fields.contains(&k) { return None }
        match v {
            JSONValue::Null => Some((k, "".to_owned())),
            JSONValue::Number(ref n) => Some((k, n.to_string())),
            JSONValue::String(ref s) => Some((k, s.clone())),
            _ => {
                warn!("unsupported type for field {}", k);
                None
            },
        }
    }).collect()
}

fn read_config_from_file<P: AsRef<Path>>(path: P) -> IndexConfig {
    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);
    serde_json::from_reader(reader).unwrap()
}

fn add_documents_from_path(index: &mut Index, path: &str) {
    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);
    let data: Vec<HashMap<String, JSONValue>> = serde_json::from_reader(reader).unwrap();
    let fields = index.field_ids.keys().cloned().collect();
    for doc in data {
        index.add_document(json_document_to_text_document(doc, &fields))
    }
}

fn main() {
    let args = env::args().collect::<Vec<_>>();
    let config_path = &args[1];
    let config = read_config_from_file(config_path);
    let mut index = Index::new(&config);

    let data_path = &args[2];
    add_documents_from_path(&mut index, data_path);
}
