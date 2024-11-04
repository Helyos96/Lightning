#!/bin/python

import json
import sys

if len(sys.argv) != 2:
    exit("Usage: {} <base_items.json path>".format(sys.argv[0]))

f = open(sys.argv[1])
base_items = json.load(f)
out = {}

# Only keep items of these classes. Make sure to match item::ItemClass
allowed_class = {
    "Unarmed",
    "Ring",
    "Amulet",
    "Claw",
    "Dagger",
    "Wand",
    "Bow",
    "Staff",
    "Warstaff",
    "Shield",
    "Sceptre",
    "FishingRod",
    "Quiver",
    "Boots",
    "Belt",
    "Helmet",
    "Gloves",
    "LifeFlask",
    "ManaFlask",
    "HybridFlask",
    "UtilityFlask",
    "AbyssJewel",
    "Jewel",
    "Body Armour",
    "Rune Dagger",
    "One Hand Sword",
    "Thrusting One Hand Sword",
    "One Hand Axe",
    "One Hand Mace",
    "Two Hand Sword",
    "Two Hand Axe",
    "Two Hand Mace",
}

for v in base_items.values():
    if v["item_class"] in allowed_class:
        out[v["name"]] = v

print(json.dumps(out,indent=2))

