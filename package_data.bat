python lightning-model\scripts\cleanup_gems.py ..\RePoE-lvlvllvlvllvlvl\data\gems.json > lightning-model\data\gems.json
python lightning-model\scripts\cleanup_items.py ..\RePoE-lvlvllvlvllvlvl\data\base_items.json > lightning-model\data\base_items.json
xcopy ..\skilltree-export\data.json data.json
python lightning-model\scripts\fix_ascendancy_positions.py
python lightning-model\scripts\cleanup_tree.py data.json > lightning-model\data\tree.json
cd lightning-model
cargo run --bin json2bincode