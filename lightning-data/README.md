# Lightning-data

Tools to extract & process Path of Exile 2 game data files.

Pre-requisites:
* A [dat schema](https://github.com/poe-tool-dev/dat-schema/releases/download/latest/schema.min.json)
* [bun_extract_file](https://github.com/zao/ooz/releases)
* [magick](https://imagemagick.org/script/download.php)
* An installation of PoE2

```
cd lightning-data
cargo run --release -- -s <path_to_dat_schema_json> -p <path_to_poe2_dir> -e -d
```

Run `cargo run -- --help` for usage information. Use `-e` only on the first run and on game updates. `-d` only if DDS files change.

IMPORTANT: You'll have to manually convert `stat_descriptions.csd` and `passive_skill_stat_descriptions.csd` to UTF-8, as lightning-data doesn't yet handle the UTF-16 text in those files.
