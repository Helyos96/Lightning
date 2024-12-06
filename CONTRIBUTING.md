## Generating filtered data

Lightning makes use of JSON data from RePoE, however these are pre-filtered to only include what we need.

To achieve that, the python scripts in `lightning-model/scripts/` are run against [lvlvllvlvllvlvl's RePoE fork](https://github.com/repoe-fork/repoe-fork.github.io). Lightning is currently using revision `9bae2d59e0f96c38bae0e9e0a3470624c00d506f` of this fork.

After the JSON data is filtered and output to `lightning-model/data`, the binary `lightning-model/json2bincode` is run against them to generate small bincode (*.bc) files.

The bat script `package_data.bat` runs these steps in order and it assumes it is run from Lightning's root directory, with `../RePoE-lvlvllvlvllvlvl` being a valid git clone path of RePoE.
