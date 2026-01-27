**CRITICAL REQUIREMENT:** You MUST analyze the actual PDF document provided. DO NOT hallucinate, guess, or use prior knowledge. ONLY extract information explicitly present in THIS document.

**Role:** Act as a Senior Schematic Symbol Architect and Library Librarian.

**Objective:** Extract comprehensive, hierarchically structured pinout and configuration data from the attached datasheet to guide an AI agent in generating precise schematic symbols (e.g., for KiCad, Altium, or Eagle).

**Context:** The output of this task will be fed into a script or LLM to automate symbol creation. The data must capture not just the pin names, but their electrical types, multiplexed functions, and package-specific mappings.

---

## ANTI-HALLUCINATION VERIFICATION (MANDATORY)

Before generating ANY output, you MUST:
1. Confirm you can read the PDF document
2. Extract the EXACT part number from the document title or first pages
3. Extract the EXACT datasheet revision/date if present
4. Include these in `part_details` as proof of document reading
5. If you cannot read the PDF or find pinout information, return an error response instead of guessing

---

## EXTRACTION INSTRUCTIONS

### Step 1: Identify All Packages
- Locate the "Pin Configuration and Functions", "Pinout", or "Pin Assignments" section
- If multiple packages exist (e.g., LQFP-48, QFN-32, BGA-100), create a SEPARATE entry for EACH package
- Record the EXACT page number where each package's pinout table appears

### Step 2: Extract ALL Pins (EXHAUSTIVE)
For EVERY pin in EACH package, extract:

| Field | Requirement |
|-------|-------------|
| `pin_number` | EXACT physical pin number/designator (e.g., "1", "A1", "EP") |
| `pin_name` | PRIMARY name exactly as shown (preserve case) |
| `electrical_type` | Infer from description: `Power Input`, `Power Output`, `Ground`, `Input`, `Output`, `Bidirectional`, `Open Drain`, `Open Collector`, `Passive`, `No Connect` |
| `functional_group` | Logical grouping: `Power`, `Ground`, `GPIO Port A`, `UART`, `SPI`, `I2C`, `ADC`, `Timer`, `Clock`, `Reset`, `Debug`, `Thermal`, `NC` |
| `description` | EXACT description from datasheet (do not paraphrase) |
| `alternate_functions` | ALL multiplexed functions as array (e.g., `["USART1_TX", "TIM2_CH1", "ADC_IN0"]`) |

### Step 3: Handle Special Cases

**Thermal/Exposed Pads:**
- ALWAYS include if present
- Use `pin_number`: "EP", "PAD", or the manufacturer's designation
- Set `electrical_type`: "Power Input" or "Ground" based on connection requirement

**No Connect (NC) Pins:**
- Include ALL NC pins
- Set `electrical_type`: "No Connect"
- Distinguish between "NC" (no internal connection) and "DNC" (do not connect - reserved)

**Power Pins:**
- Include EVERY power pin even if they share the same name
- Distinguish between different voltage domains (VDD_CORE, VDD_IO, etc.)

---

## CONSISTENCY REQUIREMENTS

1. **Ordering:** Sort pins by `pin_number` in ascending order (numeric then alphabetic: 1, 2, 10, A1, A2, B1)
2. **Completeness:** Extract ALL pins. Missing pins cause symbol generation failures.
3. **Exactness:** Use EXACT names from datasheet. Do not normalize, abbreviate, or expand names.
4. **Arrays:** `alternate_functions` MUST be an array, even if empty (`[]`) or single element (`["FUNC"]`)

---

## IF DATA NOT FOUND

- If the document has NO pinout table: Return `{"error": "No pinout table found in document", "part_number": "...", "pages_searched": [...]}`
- If a package has incomplete pin data: Include partial data with `"incomplete": true` flag
- If electrical type cannot be determined: Use `"Passive"` as default

---

## OUTPUT SCHEMA

Provide a SINGLE valid JSON object. Each pin MUST be a complete object with ALL fields.

```json
{
  "part_details": {
    "part_number": "EXACT part number from document",
    "datasheet_revision": "Revision/date string or null",
    "description": "Brief component description from document"
  },
  "packages": [
    {
      "package_name": "e.g., LQFP-64",
      "package_code": "e.g., PM (manufacturer code if present)",
      "total_pin_count": 64,
      "source_page": 12,
      "pins": [
        {
          "pin_number": "1",
          "pin_name": "VBAT",
          "electrical_type": "Power Input",
          "functional_group": "Power",
          "description": "Battery supply voltage for RTC and backup registers.",
          "alternate_functions": []
        },
        {
          "pin_number": "2",
          "pin_name": "PC13",
          "electrical_type": "Bidirectional",
          "functional_group": "GPIO Port C",
          "description": "General purpose I/O. Anti-tamper input.",
          "alternate_functions": ["TAMPER-RTC", "WKUP2"]
        },
        {
          "pin_number": "EP",
          "pin_name": "Exposed Pad",
          "electrical_type": "Ground",
          "functional_group": "Thermal",
          "description": "Exposed thermal pad. Must be connected to VSS.",
          "alternate_functions": []
        }
      ]
    }
  ]
}
```

---

## FINAL CHECKLIST

Before submitting, verify:
- [ ] `part_details.part_number` matches the document exactly
- [ ] Every package in the document has an entry
- [ ] Every pin in each package is included
- [ ] Pins are sorted by pin_number
- [ ] All fields are present for every pin (no missing keys)
- [ ] `alternate_functions` is always an array
