# README.md

## Brutalball Scraper

A **std-only**, tiny Rust scraper that pulls player data from the Dozerverse/Brutalball website and writes a CSV **without headers**.

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
./bb-scrape --all -o all_players.csv
```

---

## Output format

* **No header row** (intended for easy paste into spreadsheets).
* Each row is **CSV** with standard quoting:

  * Fields containing `,` or `"` or newline are quoted.
  * Inner quotes are doubled (`"` → `""`).

**Columns (in order):**

1. `Name` — player name
2. `Number` — player number, e.g. `#27` (the hash is **kept**)
3. `Race` — e.g. `Common Orc`
4. `Team` — team name
5. Remaining numeric attributes as shown on the site, left-to-right

---

## Behavior & assumptions

* Only parses `<table class=teamroster>`.
* Team name is taken from the first row’s first cell (trimmed before `" Team owner"` or `" | "` if present).
* Player rows are recognized by `class="playerrow"` / `class="playerrow1"`.
* The first data cell is expected to look like:
  `Name #<digits> <race…>`
  It’s split at the first `#` into **Name**, **#Number**, **Race**.
* Minimal HTML handling:

  * Naive tag stripping, whitespace collapsing, and entity decoding for `&nbsp;`/`&amp;` only.
* Uses HTTP/1.0 (`Connection: close`) over **plain HTTP**.

If the site’s HTML changes, this will break.

---

## Troubleshooting

* **Empty output / “teamroster table not found”**
  The page didn’t contain `<table class=teamroster>`. Verify the team ID and that the site structure hasn’t changed.

* **HTTP error**
  The server didn’t return 200. Check connectivity or try the URL in a browser:
  `http://dozerverse.com/brutalball/team.php?i=X`

* **Weird characters / entities**
  Add more mappings in `html::normalize_entities()` if needed.

---

## Quick reusability notes

* Core extraction is in `roster::extract_player_rows(html, team_id) -> Vec<Vec<String>>`.
* CSV writing is isolated in `csv::write_csv_row()`.
* If you want headers later, prepend a header row before writing players.
* If you want digits-only player numbers, strip the `#` in `split_first_cell()`.

---

## Build

```bash
# Release build (recommended)
cargo build --release
# Binary at: target/release/bb-scrape
```
