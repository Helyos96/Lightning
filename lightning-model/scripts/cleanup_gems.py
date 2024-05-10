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
    levels = v["per_level"]
    v["per_level"] = []
    for lvl in levels.values():
        stats = []
        if "stats" in lvl:
            for i,stat in enumerate(lvl["stats"]):
                if stat is None:
                    stats.append(None)
                else:
                    if "id" in stat:
                        if not "stats" in v["static"]:
                            cull.append(k)
                            break
                        if v["static"]["stats"][i] is None:
                            v["static"]["stats"][i] = {}
                        v["static"]["stats"][i]["id"] = stat["id"]
                    if "value" not in stat:
                        stats.append(None)
                    else:
                        stats.append(stat["value"])
        lvl["stats"] = stats
        v["per_level"].append(lvl)
    if v["tags"] is None:
        v.pop("tags", None)

for k in cull:
    #print("culling", k)
    gems.pop(k, None)

print(json.dumps(gems,indent=2))

