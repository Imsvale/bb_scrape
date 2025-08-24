# Brutalball Scraper

A fast site scraper for Brutalball.

![Graphical interface](assets/screenshot_small.png)

---

## Download & Run

* **Windows:** run `bb_scrape.exe` for the GUI.
  Command-line users run `cli.exe`.
* **Linux/macOS (from source):**
  * Install Rust, then:
  * Build GUI: `cargo build --release`
  * Build CLI: `cargo build --release --bin cli --features=cli` (required)

### Quick start (Windows)

1. **Download** `bb_scrape.exe`.
1. **Move** it to a suitable folder.
1. **Run** `bb_scrape.exe`.
1. Click **SCRAPE**.
1. `Copy` to clipboard.
1. `Export` to file → `out/players/all.csv`.

Tip: The left panel lets you pick which teams to scrape.

---

### Features

* **Player data:** `Name, #00, Race, Team, TV, OVR, ..., Dur, Sal`
* **Formats:** 
  * Comma-separated values `(CSV)`
  * Tab-separated values `(TSV)`
* **Toggle headers**
* **Toggle player number `#` sign**
* **Copy to clipboard**
* **Export to file**
  * Single file (all players)
  * Per team
* **Select** which teams to scrape.
  * `All` / `None`
  * `Ctrl + click`: Select individual teams
  * `Shift + click`: Select range of teams
  * `Ctrl + Shift + click`: Select multiple ranges
* **Scrape** to update on demand.
* **Data cached locally**

---

### Defaults

* **Export directory:**
  * `out/players`
* **Format:** `CSV`
* **Export file (single):** `all.csv`
* **Export files (multi):** `<Team_Name>.csv`
* **Local cache:** `.store`

---

### Command line usage

Run:

```bash
./cli
```

Scrapes all teams and outputs all players to default directory and file: `out/players/all.csv`.

Print help:

```bash
./cli -h
```

Common flags:

```
-p, --page players|teams    Which page to scrape (default: players)
-f, --format csv|tsv        Output format (default: csv)
-m, --multi, --per-team     Per-team files
-t, --team <id>             One team by id (0–31)
-o, --out <path>            Output file path (single) or directory (per-team)
-i, --ids <list>            Subset of ids, e.g. 0,2,5-7
-x, --drop-headers          Do not write the header row
-#, --nohash                Strip '#' from player numbers (Players page only)
-l, --list-teams            Print "id,team" for all teams and exit
```

Examples:

```bash
# All teams → single CSV (default path: out/players/all.csv)
cli

# One team (id 0) → TSV with headers
cli --team 0 --format tsv -o out/vuvu.tsv

# A subset of teams → per-team CSVs in a folder
cli --ids 0,2,5-7 -o out/players

# Teams list only
cli --page teams -o out/teams.csv
```

---

### Caching & Refresh

* The app stores raw datasets under `.store/`.
* On startup, it loads the cache if present.
* Team names can be force-refreshed by clicking Refresh.
* Team names are refreshed with a **SCRAPE**.

---

### License

MIT. Use at your own risk.
