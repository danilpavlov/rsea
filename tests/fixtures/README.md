# Approved scene reference

`open-water-low-rich.txt` was captured from the Open water renderer in
`design/ascii-sea-variants.html` after the user selected **Close to the water**
and **Rich** detail on 2026-11-16.

Capture parameters: 91 columns, 32 rows, simulated time 5 seconds, low viewpoint,
rich detail, and a character aspect ratio of 7.8/17. Rust seed 42 reproduces the
reference wave phases. Trailing spaces are omitted; blank sky rows are retained.

The CLI test compares this independent design reference with the Rust output.
Update it only when an intentional change to the approved scene requires it;
do not regenerate it from the Rust implementation to fix a failing test.
