## Generating filtered data

Lightning makes use of JSON data from RePoE, however these are pre-filtered to only include what we need.

To achieve that, the python scripts in `lightning-model/scripts/` are run against [lvlvllvlvllvlvl's RePoE fork](https://github.com/repoe-fork/repoe-fork.github.io).

After the JSON data is filtered via python scripts and saved to `lightning-model/data`, the binary `lightning-model/json2bincode` is run against these .json files to generate smaller bincode (\*.bc) files.

The bat script `package_data.bat` runs these steps in order and it assumes it is run from Lightning's root directory, with `../RePoE-lvlvllvlvllvlvl` being a valid git clone path of RePoE.
