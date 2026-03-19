You are analyzing a PDF document to locate specific pages and regions.

Find all pages and regions matching the following description(s):

{DESCRIPTIONS}

For each match found, return:
- The **1-based page number**
- A **descriptive label** (what was found)
- A **bounding box** in normalized coordinates (0-1000 for both axes)
  - (0, 0) = top-left corner of the page
  - (1000, 1000) = bottom-right corner of the page

## Bounding box rules

- Return a **tight** bounding box that fully encloses the content and any associated labels, annotations, or legends.
- If multiple distinct matches appear on the **same page**, return each as a **separate** entry.
- If a single item spans most of a page, the bbox should cover just the item area, not the full page.
- If **no** matches are found, return an empty `results` array.
- When in doubt about whether something matches, include it — the user can filter later.
