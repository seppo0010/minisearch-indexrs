# Minisearch Indexrs

**This project is deprecated and incompatible with minisearch**

Creates a serialized index for
[Minisearch](https://lucaong.github.io/minisearch/). In most cases, with large
a corpus, it should be faster than using minisearch itself to build the index.

## Usage

Create a configuration json file such as the one in minisearch (`fields`,
`storedFields`) and a data json file. Then run
`minisearch-indexrs <config_path> <data_path> > index.json`.
The file can then be imported into minisearch using
[`loadJSON`](https://lucaong.github.io/minisearch/classes/_minisearch_.minisearch.html#loadjson).

## Limitations

This project is not a minisearch full implementation. It only creates an index
from scratch. It does not do search or autosuggest, and the created indices
cannot be updated.

Currently custom tokenizers, preprocessors and nested fields are not available.
They might get added later.

## Benchmark

Using `billboard_1965-2015.json` (439K) it shows a speedup of 2.5X, from
~0.233s to ~0.096s on a MacBookPro16,2.

## Install

You can install this using cargo:

`cargo install --git https://github.com/seppo0010/minisearch-indexrs.git  --tag 0.0.1 --locked`
