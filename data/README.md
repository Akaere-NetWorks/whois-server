# Data Directory

This directory contains data files used by the whois-server.

## recipes.json

Chinese recipes data from [HowToCook](https://github.com/Anduin2017/HowToCook) - 程序员做饭指南.

JSON generated from [HowToCook-gen-json](https://github.com/z0z0r4/HowToCook-gen-json).

Used by the `-MEAL-CN` query type to provide random Chinese recipe suggestions.

### Usage

```bash
whois -h whois.akae.re 今天吃什么中国
whois -h whois.akae.re -MEAL-CN
```

### Update

To update the recipes:

```bash
# Download the latest recipes.json from HowToCook repository
wget https://raw.githubusercontent.com/z0z0r4/HowToCook-gen-json/refs/heads/main/filtered_recipes.json -O data/recipes.json
```
