# Lightning-data

Tools to extract & process Path of Exile 1/2 game data files.

Pre-requisites:
* A [dat schema](https://github.com/poe-tool-dev/dat-schema/releases/download/latest/schema.min.json)
* [bun_extract_file](https://github.com/zao/ooz/releases)
* Optionally for PoE2 tree visual assets: [magick](https://imagemagick.org/script/download.php)
* An installation of PoE

Run `cargo run -- --help` for usage information. Use `-e` only on the first run and on game updates. `-d` only if DDS files change.
