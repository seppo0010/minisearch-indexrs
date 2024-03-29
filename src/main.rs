use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io;
use std::io::BufReader;
use std::io::Write;
use std::path::Path;
use std::process;

use indicatif::ProgressBar;
use lazy_static::lazy_static;
use log::{debug, warn};
use regex::Regex;
use serde_json::Value as JSONValue;
use structopt::StructOpt;

mod errors;
mod index;
mod serializer;

fn tokenize(text: &str) -> impl Iterator<Item = &str> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"[\n\r -#%-*,-/:;?@\[-\]_{}\u00A0\u00A1\u00A7\u00AB\u00B6\u00B7\u00BB\u00BF\u037E\u0387\u055A-\u055F\u0589\u058A\u05BE\u05C0\u05C3\u05C6\u05F3\u05F4\u0609\u060A\u060C\u060D\u061B\u061E\u061F\u066A-\u066D\u06D4\u0700-\u070D\u07F7-\u07F9\u0830-\u083E\u085E\u0964\u0965\u0970\u09FD\u0A76\u0AF0\u0C77\u0C84\u0DF4\u0E4F\u0E5A\u0E5B\u0F04-\u0F12\u0F14\u0F3A-\u0F3D\u0F85\u0FD0-\u0FD4\u0FD9\u0FDA\u104A-\u104F\u10FB\u1360-\u1368\u1400\u166E\u1680\u169B\u169C\u16EB-\u16ED\u1735\u1736\u17D4-\u17D6\u17D8-\u17DA\u1800-\u180A\u1944\u1945\u1A1E\u1A1F\u1AA0-\u1AA6\u1AA8-\u1AAD\u1B5A-\u1B60\u1BFC-\u1BFF\u1C3B-\u1C3F\u1C7E\u1C7F\u1CC0-\u1CC7\u1CD3\u2000-\u200A\u2010-\u2029\u202F-\u2043\u2045-\u2051\u2053-\u205F\u207D\u207E\u208D\u208E\u2308-\u230B\u2329\u232A\u2768-\u2775\u27C5\u27C6\u27E6-\u27EF\u2983-\u2998\u29D8-\u29DB\u29FC\u29FD\u2CF9-\u2CFC\u2CFE\u2CFF\u2D70\u2E00-\u2E2E\u2E30-\u2E4F\u3000-\u3003\u3008-\u3011\u3014-\u301F\u3030\u303D\u30A0\u30FB\uA4FE\uA4FF\uA60D-\uA60F\uA673\uA67E\uA6F2-\uA6F7\uA874-\uA877\uA8CE\uA8CF\uA8F8-\uA8FA\uA8FC\uA92E\uA92F\uA95F\uA9C1-\uA9CD\uA9DE\uA9DF\uAA5C-\uAA5F\uAADE\uAADF\uAAF0\uAAF1\uABEB\uFD3E\uFD3F\uFE10-\uFE19\uFE30-\uFE52\uFE54-\uFE61\uFE63\uFE68\uFE6A\uFE6B\uFF01-\uFF03\uFF05-\uFF0A\uFF0C-\uFF0F\uFF1A\uFF1B\uFF1F\uFF20\uFF3B-\uFF3D\uFF3F\uFF5B\uFF5D\uFF5F-\uFF65]+").unwrap();
    }
    RE.split(text)
}

fn get_document_tokens(
    field_ids: &HashMap<String, usize>,
    document: &HashMap<String, String>,
    document_id: usize,
) -> Vec<(String, usize, usize)> {
    let default = &"".to_owned();
    field_ids
        .iter()
        .flat_map(|(field_name, field_id)| {
            let text = document.get(field_name).unwrap_or(default);
            let tokens = tokenize(text);
            tokens.map(|x| (x.to_owned(), *field_id, document_id.to_owned()))
        })
        .collect()
}

fn json_document_to_text_document(
    json_document: &HashMap<String, JSONValue>,
    fields: &HashSet<String>,
) -> HashMap<String, String> {
    json_document
        .iter()
        .filter_map(|(k, v)| {
            if k != "id" && !fields.contains(k) {
                return None;
            }
            match v {
                JSONValue::Null => Some((k.clone(), "".to_owned())),
                JSONValue::Number(ref n) => Some((k.clone(), n.to_string())),
                JSONValue::String(ref s) => Some((k.clone(), s.clone())),
                _ => {
                    warn!("unsupported type for field {}", k);
                    None
                }
            }
        })
        .collect()
}

fn get_path_documents<P: AsRef<Path>>(
    path: P,
) -> Result<Vec<HashMap<String, JSONValue>>, failure::Error> {
    debug!("reading documents from {}", path.as_ref().to_string_lossy());
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    Ok(serde_json::from_reader(reader)?)
}

fn create_index(
    docs: Vec<HashMap<String, JSONValue>>,
    config: index::IndexConfig,
    progress: Option<&ProgressBar>,
) -> Result<index::Index, failure::Error> {
    let mut index = index::Index::new(config);
    let field_ids = index.field_ids();
    let fields = field_ids.keys().cloned().collect();

    let docs = docs
        .into_iter()
        .map(|mut d| {
            let small_id = index.insert_document(
                d.remove("id")
                    .ok_or(errors::MinisearchIndexrsError::MissingId)?,
            );
            Ok((small_id, d))
        })
        .collect::<Result<Vec<_>, failure::Error>>()?;

    index.add_document_tokens(docs.iter().flat_map(|(small_id, doc)| {
        if let Some(p) = progress {
            p.inc(1);
        }
        let doc = json_document_to_text_document(doc, &fields);
        get_document_tokens(&field_ids, &doc, *small_id)
    }))?;
    index.add_document_fields(docs.into_iter());
    Ok(index)
}

#[derive(StructOpt)]
struct Cli {
    #[structopt(parse(from_os_str))]
    config_path: std::path::PathBuf,
    #[structopt(parse(from_os_str))]
    data_path: std::path::PathBuf,
    #[structopt(default_value = "0")]
    benchmark: usize,
}

fn inner_main<W: Write>(args: Cli, writer: &mut W) -> Result<(), failure::Error> {
    let config = index::read_config_from_file(args.config_path)?;
    let docs = get_path_documents(args.data_path)?;

    if args.benchmark > 0 {
        for (docs, config) in (1..args.benchmark)
            .into_iter()
            .map(|_| (docs.clone(), config.clone()))
        {
            create_index(docs, config, None)?.into_minisearch_json()?;
        }
    } else {
        let progress = ProgressBar::new(docs.len().try_into().unwrap());
        writeln!(
            writer,
            "{}",
            create_index(docs, config, Some(&progress))?.into_minisearch_json()?
        )?;
    }
    Ok(())
}

fn main() {
    env_logger::init();
    let args = Cli::from_args();
    process::exit(match inner_main(args, &mut io::stdout()) {
        Ok(_) => 0,
        Err(ref e) => {
            eprintln!("{}", e);
            1
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_json_diff::assert_json_eq;
    use serde_json::json;
    use tempfile::NamedTempFile;

    #[test]
    fn test_integration() {
        let mut output = Vec::<u8>::new();
        let mut config = NamedTempFile::new().unwrap();
        config
            .write_all(r#"{"fields":["a","b"],"store_fields":["a"]}"#.as_bytes())
            .unwrap();

        let mut data = NamedTempFile::new().unwrap();
        data.write_all(
            r#"[{"id":"bar","a":"1","b":"123","c":"124"},{"id":"foo","a":"a","b":"b","c":"cd"}]"#
                .as_bytes(),
        )
        .unwrap();
        inner_main(
            Cli {
                config_path: config.path().to_path_buf(),
                data_path: data.path().to_path_buf(),
                benchmark: 0,
            },
            &mut output,
        )
        .unwrap();
        let json: JSONValue = serde_json::from_str(std::str::from_utf8(&output).unwrap()).unwrap();
        assert_json_eq!(
            json,
            json!({
               "averageFieldLength" : {
                  "0" : 1.0,
                  "1" : 1.0
               },
               "documentCount" : 2,
               "documentIds" : {
                  "0" : "bar",
                  "1" : "foo"
               },
               "fieldIds" : {
                  "a" : 0,
                  "b" : 1
               },
               "fieldLength" : {
                  "0" : {
                     "0" : 1,
                     "1" : 1
                  },
                  "1" : {
                     "0" : 1,
                     "1" : 1
                  }
               },
               "index" : {
                  "_prefix" : "",
                  "_tree" : {
                     "1" : {
                        "" : {
                           "0" : {
                              "df" : 1,
                              "ds" : {
                                 "0" : 1
                              }
                           }
                        },
                        "23" : {
                           "" : {
                              "1" : {
                                 "df" : 1,
                                 "ds" : {
                                    "0" : 1
                                 }
                              }
                           }
                        }
                     },
                     "a" : {
                        "" : {
                           "0" : {
                              "df" : 1,
                              "ds" : {
                                 "1" : 1
                              }
                           }
                        }
                     },
                     "b" : {
                        "" : {
                           "1" : {
                              "df" : 1,
                              "ds" : {
                                 "1" : 1
                              }
                           }
                        }
                     }
                  }
               },
               "nextId" : 2,
               "storedFields": {
                   "0": {
                       "a": "1"
                   },
                   "1": {
                       "a": "a"
                   }
               }
            }),
        );
    }
}
