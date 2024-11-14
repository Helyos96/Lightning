#!/bin/python3

import json
import sys

if len(sys.argv) != 2:
    exit("Usage: {} <gems.json path>".format(sys.argv[0]))

f = open(sys.argv[1])
gems = json.load(f)
cull = []

for (k,v) in gems.items():
    if "Royale" in k:
        cull.append(k)
        continue
    if "base_item" not in v or v["base_item"] is None:
        cull.append(k)
        continue
    if "release_state" not in v["base_item"]:
        cull.append(k)
        continue
    if v["base_item"]["release_state"] != "released":
        cull.append(k)
        continue
    if v["tags"] is None:
        v.pop("tags", None)

for k in cull:
    #print("culling", k)
    gems.pop(k, None)

print(json.dumps(dict(sorted(gems.items())),indent=2))

