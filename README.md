# Brutalball Scraper

A fast site scraper for Brutalball.

![Graphical interface](assets/screenshot_small.png)

---

### Quick start (Windows)

* **Download** the latest release, e.g. `bb_scrape_v1.2.0_windows_x86_64.zip`.
* **Extract** it to a suitable folder.
* **Run** `bb_scrape.exe` for the GUI.
* Click **SCRAPE**.
* `Copy` to clipboard.
* `Export` to file → `out/players/all.csv`.
*  Command-line users run `cli.exe`.

Tip: The left panel lets you pick which teams to scrape.

### Linux/macOS 

**Build from source**:

* Install Rust, then:
* Build GUI: `cargo build --release`
* Build CLI: `cargo build --release --bin cli --features=cli` (required)
* Build both: `cargo build --release`

---

### Features

* **Scrape** for **Players** and **Game results** (current season)
  * Also **teams**, but that's more of an incidental side thing.
* **Player data:** `Name, #00, Race, Team, TV, OVR, ..., Dur, Sal`
* **Formats:** 
  * Comma-separated values `(CSV)`
  * Tab-separated values `(TSV)`
* **Toggle headers**
* **Page-specific optionals:**
  * **Players:** Toggle player number `#` sign
  * **Game results:** Toggle game ids column
* **Copy to clipboard**
* **Export to file**
  * All-in-one file
  * Separate file per team (where applicable)
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
-h, --help                  Show help text
-l, --list-teams            Print id and name for all teams

SCRAPE:
-p, --page <page>           Which page to scrape (default: players)
                            Valid pages: players|game-results|teams
-t, --team <id>             One team by id (0–31)
-i, --ids <list>            Subset of ids, e.g. 0,2,5-7

EXPORT:
-m, --multi, --per-team     Per-team files (Players, Game Results)
-x, --drop-headers          Do not write the header row
-s, --skip-optional         Page-agnostic: Players → remove '#'; Results → drop match id
-f, --format csv|tsv        Output format (default: csv)
-o, --out <path>            Output file path (single) or directory (per-team)
```

Examples:

```bash
# One team (id 16 = BDP) → TSV
./cli --team 16 --format tsv --out out/bdp.tsv

# Same as above with short-form flags
./cli -t 16 -f tsv -o out/bdp.tsv

# A subset of teams → per-team CSVs in a specified folder
./cli --ids 0,2,5-7 -o out/week8

# Fetch and export team names and ids
./cli --page teams -o out/teams.csv
```

---

### Caching & Refresh

* The app stores raw datasets under `.store`.
* On startup, it loads the cache if present.
* Team names are refreshed with a **SCRAPE**.

---

### License

MIT. Use at your own risk.
