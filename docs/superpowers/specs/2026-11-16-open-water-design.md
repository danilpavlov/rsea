# Approved open-water view

The user selected **Open water**, **Close to the water**, and **Rich** in
`design/ascii-sea-variants.html`, then asked to use that design in `rsea`.

The terminal scene is always ASCII. It contains a perspective view of an animated
3D sea with overlapping crests, shaded faces, fine ripples, and foam. There is no
sun, moon, bird, or coast. Following preview approval, the user requested colors
that match the terminal. The default `terminal` theme uses the terminal's own
foreground and background; no theme paints a background, including status and
help. The preview's dark blue-gray foreground palette remains available as
`--theme ocean`.

Port the approved height field, first-hit ray intersection, surface normals,
lighting, and glyph selection. Keep resizing, pause, speed, seeded variation,
color modes, snapshots, and terminal restoration. Expose the preview's viewpoint
and detail choices, defaulting to `low` and `rich`. Existing themes become color
palettes only. Seed 42 reproduces the approved reference waves.

The browser preview remains a design reference; the Rust binary is the deliverable.
