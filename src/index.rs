use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use failure::Error;
use log::debug;
use patricia_tree::PatriciaMap;
use serde::Deserialize;
use serde_json::{Map as JSONMap, Value as JSONValue};

use crate::serializer;

pub struct Index {
    field_ids: HashMap<String, usize>,
    document_ids: JSONMap<String, JSONValue>,
    next_id: usize,
    /* {fieldId: count} */
    field_num_tokens: HashMap<usize, usize>,
    /* {documentId: {fieldId: count} } */
    field_length: HashMap<usize, HashMap<usize, usize>>,
    map: PatriciaMap<Vec<(usize, usize)>>,
    // TODO: custom tokenizer
    // TODO: custom term processing
}

impl Index {
    pub fn new(config: &IndexConfig) -> Self {
        let field_ids = config
            .fields
            .iter()
            .enumerate()
            .map(|(i, v)| (v.to_owned(), i))
            .collect::<HashMap<String, usize>>();
        Index {
            field_ids,
            document_ids: JSONMap::new(),
            field_num_tokens: HashMap::new(),
            field_length: HashMap::new(),
            next_id: 0,
            map: PatriciaMap::new(),
        }
    }

    pub fn insert_document(&mut self, id: JSONValue) -> usize {
        let small_id = self.next_id;
        self.document_ids.insert(small_id.to_string(), id);
        self.next_id += 1;
        return small_id;
    }

    pub fn add_document_tokens<I>(&mut self, document_tokens: I) -> Result<(), failure::Error>
    where
        I: Iterator<Item = (String, usize, usize)>,
    {
        for (token, field_id, small_id) in document_tokens {
            let num_tokens = self.field_num_tokens.get(&field_id).unwrap_or(&0) + 1;
            self.field_num_tokens.insert(field_id, num_tokens);

            let default_document_fields_length = HashMap::new();
            let mut document_fields_length = self
                .field_length
                .remove(&small_id)
                .unwrap_or(default_document_fields_length);
            let num_document_field_length = document_fields_length.get(&field_id).unwrap_or(&0) + 1;
            document_fields_length.insert(field_id, num_document_field_length);
            self.field_length.insert(small_id, document_fields_length);

            self.field_num_tokens.insert(field_id, num_tokens);
            self.add_token(small_id, &process_term(&token), field_id);
        }
        Ok(())
    }

    pub fn add_token(&mut self, document_id: usize, token: &str, field_id: usize) {
        // conditional double insert sounds more efficient than get-insert
        let old = self
            .map
            .insert(token, vec![(document_id.to_owned(), field_id)]);
        if let Some(mut old) = old {
            old.push((document_id.to_owned(), field_id));
            self.map.insert(token, old);
        }
    }

    pub fn field_ids(&self) -> HashMap<String, usize> {
        self.field_ids.clone()
    }

    pub fn into_minisearch_json(self) -> Result<String, failure::Error> {
        let mut h = JSONMap::new();
        h.insert("documentCount".to_string(), self.next_id.into());
        h.insert("nextId".to_string(), self.next_id.into());
        h.insert("documentIds".to_string(), self.document_ids.into());
        h.insert(
            "fieldIds".to_string(),
            serializer::field_ids_json(self.field_ids).into(),
        );
        h.insert(
            "averageFieldLength".to_string(),
            serializer::average_field_length_json(self.field_num_tokens, self.next_id as f64)
                .into(),
        );
        h.insert(
            "fieldLength".to_string(),
            serializer::field_length_json(self.field_length).into(),
        );
        h.insert("index".to_string(), serializer::map_json(self.map)?.into());

        // TODO: storedFields

        return Ok(serde_json::to_string(&JSONValue::Object(h)).unwrap());
    }
}

#[derive(Deserialize, Debug)]
pub struct IndexConfig {
    fields: Vec<String>,
    #[serde(alias = "storeFields")]
    store_fields: Vec<String>,
}

pub fn read_config_from_file<P: AsRef<Path>>(path: P) -> Result<IndexConfig, Error> {
    debug!("reading config from {}", path.as_ref().to_string_lossy());
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    Ok(serde_json::from_reader(reader)?)
}

fn process_term(term: &str) -> String {
    term.to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_insert_document() {
        let mut index = Index::new(&IndexConfig {
            fields: vec!["author".to_string(), "title".to_string()],
            store_fields: vec!["author".to_string(), "title".to_string()],
        });

        index.insert_document("id1".into());
        index.insert_document("id2".into());
        index.insert_document("id3".into());
        index.insert_document(4.into());

        assert_eq!(index.next_id, 4);
        assert_eq!(
            &index.document_ids,
            json!({
                "0": "id1",
                "1": "id2",
                "2": "id3",
                "3": 4,
            })
            .as_object()
            .unwrap()
        );
    }

    #[test]
    fn test_add_document_tokens() {
        let mut index = Index::new(&IndexConfig {
            fields: vec!["author".to_string(), "title".to_string()],
            store_fields: vec!["author".to_string(), "title".to_string()],
        });
        index
            .add_document_tokens(
                vec![
                    ("foo".to_owned(), 0, 0),
                    ("bar".to_owned(), 1, 0),
                    ("foo".to_owned(), 0, 1),
                    ("baz".to_owned(), 1, 1),
                ]
                .into_iter(),
            )
            .unwrap();
        assert_eq!(index.map.get("foo"), Some(&vec![(0, 0), (1, 0)]));
        assert_eq!(index.map.get("bar"), Some(&vec![(0, 1)]));
        assert_eq!(index.map.get("baz"), Some(&vec![(1, 1)]));
    }
}
