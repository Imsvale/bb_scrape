# README.md

## Brutalball Scraper

A super-lightweight scraper that pulls player data from the Dozerverse/Brutalball website and writes it to a file in CSV format.

---

## Usage (CLI)

```
Usage: --all | -t <id> [-o <output.csv>]

Options:
  -a, --all             Scrape all teams
  -t, --team <id>       Scrape a single team by ID
  -o, --out <file>      Output CSV path (default: players.csv or players_all.csv)
  -h, --help            Show this help
```

### Examples

```bash
# Single team (ID 20: Failurewood Hills) -> players.csv
./bb-scrape -t 20

# Single team to custom file
./bb-scrape -t 20 -o failurewood.csv

# All teams -> players_all.csv
./bb-scrape --all

# All teams to custom file
./bb-scrape --all -o league.csv
```

---

## Output format

* **No header row** (intended for easy paste into spreadsheets).
* Each row is **CSV**.

**Columns (in order):**

1. `Name` — player name
2. `Number` — player number, e.g. `#27` (the hash is **kept**)
3. `Race` — e.g. `Common Orc`
4. `Team` — team name
5. Remaining numeric attributes as shown on the site, left-to-right

---

## Build

```bash
# Release build (recommended)
cargo build --release
# Binary at: target/release/bb-scrape
```
