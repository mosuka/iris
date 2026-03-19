# Movies Example

Index and search ~32,000 movies from the [Meilisearch movies dataset](https://github.com/meilisearch/datasets).

## Schema

The [schema.toml](schema.toml) defines the following fields:

| Field | Type | Indexed | Stored | Description |
| ----- | ---- | ------- | ------ | ----------- |
| `title` | Text | Yes | Yes | Movie title |
| `overview` | Text | Yes | Yes | Plot summary |
| `genres` | Text | Yes | Yes | Comma-separated genre list |
| `poster` | Text | No | Yes | Poster image URL |
| `release_date` | Integer | Yes | Yes | Unix timestamp |

Default search fields: `title`, `overview`

## Usage

### 1. Create the index

```bash
bash examples/movies/scripts/create_index.sh
```

This builds the release binary and creates an empty index at `examples/movies/index/` using the schema.

### 2. Index all movies

```bash
bash examples/movies/scripts/index_movies.sh
```

Reads `movies.json` from the dataset, converts each record into a laurus document, and pipes all commands into the REPL in a single process. Documents are committed every 1,000 records.

### 3. Run example searches

```bash
bash examples/movies/scripts/search_movies.sh
```

Runs several example queries:

- `star wars` — full-text search across default fields
- `title:nemo` — field-specific search
- `genres:comedy` — search by genre
- `overview:robot` — search within plot summaries
- JSON output format

### Manual search

You can also search directly:

```bash
./target/release/laurus --data-dir examples/movies/index search "title:matrix" --limit 10
```

Or start an interactive session:

```bash
./target/release/laurus --data-dir examples/movies/index repl
```

## File structure

```text
examples/movies/
├── README.md
├── schema.toml          # Index schema definition
├── scripts/
│   ├── create_index.sh  # Create the index
│   ├── index_movies.sh  # Index the dataset
│   └── search_movies.sh # Example search queries
└── index/               # Generated index data (git-ignored)
```
