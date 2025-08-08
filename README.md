# README.md

## Brutalball Scraper

A super-lightweight scraper that pulls player data from the Dozerverse/Brutalball website and writes it to a file in CSV format.

By default it grabs players from all teams and outputs to `players.csv`. Just run it and wait a few seconds.

To grab players only from a specific team, use the ID from the URL along with the `-t` flag documented below.

* E.g. `team.php?i=20` → ID is `20` which is Failurewood Hills.

---

## Usage (CLI)

```
Usage: [ -t <id> ] [-o <output.csv>]

Options:
  -t, --team <id>       Scrape a single team by ID
  -o, --out <file>      Output CSV path (default: players.csv)
  -h, --help            Show this help
```

### Examples

```bash
# All teams to default file (players.csv)
./bb-scrape

# All teams to custom file
./bb-scrape -o league.csv

# Single team (ID 20: Failurewood Hills)
./bb-scrape -t 20

# Single team to custom file
./bb-scrape -t 20 -o failurewood.csv
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
# Binary at: target/release/bb_scrape
```
