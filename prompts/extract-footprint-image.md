You are analyzing a PDF datasheet to locate all footprint, land-pattern, and package-outline **drawings**.

For each footprint drawing found, return:
- The **1-based page number**
- A **descriptive label** (e.g. "QFN-48 Package Dimensions", "SOIC-8 Recommended Land Pattern")
- A **bounding box** in normalized coordinates (0–1000 for both axes)
  - (0, 0) = top-left corner of the page
  - (1000, 1000) = bottom-right corner of the page

## What to look for

- Package mechanical dimension drawings (side views, top views, bottom views with dimensions)
- Recommended land pattern / solder footprint drawings
- Package outline drawings with millimeter or inch annotations
- Solder pad layout diagrams

## What NOT to include

- Pin assignment tables (text-only tables are not drawings)
- Block diagrams or functional diagrams
- Application circuit schematics
- Timing diagrams, waveform plots, or graphs
- Photos of the physical component

## Bounding box rules

- Return a **tight** bounding box that fully encloses the drawing **and** all of its dimension annotations / leader lines.
- If multiple distinct footprint drawings appear on the **same page**, return each as a **separate** entry.
- If a single drawing spans most of a page, the bbox should cover just the drawing area, not the full page.
- If **no** footprint drawings are found, return an empty `footprints` array.
