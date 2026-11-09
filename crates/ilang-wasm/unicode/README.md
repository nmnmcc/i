# Unicode data

`UnicodeData.txt` is from the Unicode Character Database, version 18.0.0:
<https://www.unicode.org/Public/18.0.0/ucd/UnicodeData.txt>
(SHA-256 `0736451de439ae7baf1425136617da495e09ee5afbe6e394374db7009ea08950`).
It is under the Unicode License v3, in `LICENSE`.

`scripts/gen-case-tables.py` makes `src/runtime/case_tables.rs` from it: the
simple case mappings that `core.to_upper` and `core.to_lower` use (spec §13).
To move to a new version of Unicode, put the new file here, change
`UNICODE_VERSION` in the script and this file, and run the script again.
`scripts/gen-case-tables.py --check` fails when the tables are not up to date.
