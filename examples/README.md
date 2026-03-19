# Examples

This directory contains example scripts for indexing and searching datasets with laurus.

## Prerequisites

### Dataset

The examples use datasets from [meilisearch/datasets](https://github.com/meilisearch/datasets).
Clone the repository next to the laurus project directory:

```bash
cd ..
git clone https://github.com/meilisearch/datasets.git
```

Expected directory layout:

```text
parent/
├── datasets/       # meilisearch/datasets clone
│   └── datasets/
│       └── movies/
│           └── movies.json
└── laurus/         # this project
    └── examples/
```

### Tools

- [jq](https://jqlang.org/) — used by the indexing scripts to parse JSON datasets.

## Movies

Index and search ~32,000 movies from the Meilisearch movies dataset.

```bash
# 1. Create the index
bash examples/movies/scripts/create_index.sh

# 2. Index all movies
bash examples/movies/scripts/index_movies.sh

# 3. Run example searches
bash examples/movies/scripts/search_movies.sh
```

See [examples/movies/schema.toml](movies/schema.toml) for the schema definition.
