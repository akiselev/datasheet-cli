**CRITICAL REQUIREMENT:** You MUST analyze the actual PDF document provided. DO NOT hallucinate, guess, or use prior knowledge. ONLY extract information explicitly present in THIS document.

**Role:** Act as a Senior Embedded Systems Engineer and Register Map Architect.

**Objective:** Extract a complete, machine-readable register map from the attached microcontroller or peripheral datasheet. The output will be used to generate Rust PAC (Peripheral Access Crate) code via svd2rust or chiptool.

**Context:** This data will feed automated PAC generation tools. Accuracy is critical — a wrong address or bit range causes silent hardware bugs.

---

## ANTI-HALLUCINATION VERIFICATION (MANDATORY)

Before generating ANY output, you MUST:
1. Confirm you can read the PDF document
2. Extract the EXACT part number from the document title or first pages
3. Extract the EXACT datasheet revision/date if present
4. Include these in `part_details` as proof of document reading
5. If you cannot read the PDF or find register information, return an error response instead of guessing

---

## EXTRACTION INSTRUCTIONS

### Step 1: Identify All Peripherals
- Locate sections titled "Register Map", "Register Description", "Memory Map", or similar
- List every peripheral that has a register table (e.g., GPIO, SPI, UART, ADC, TIM, RCC, DMA)
- Record the base address of each peripheral from the memory map table

### Step 2: For Each Peripheral, Extract All Registers

For EVERY register in EACH peripheral:

| Field | Requirement |
|-------|-------------|
| `name` | EXACT register name as shown (e.g., `CR1`, `SR`, `DR`) |
| `description` | Brief description from datasheet |
| `offset` | Byte offset from peripheral base address (hex string, e.g., `"0x00"`) |
| `size` | Register width in bits (typically 32) |
| `reset_value` | Reset/default value as hex string (e.g., `"0x00000000"`), or null if not specified |
| `access` | `"read-write"`, `"read-only"`, `"write-only"`, or `"read-writeOnce"` |

### Step 3: For Each Register, Extract All Fields (Bit Fields)

For EVERY field in EACH register:

| Field | Requirement |
|-------|-------------|
| `name` | EXACT field name (e.g., `SPE`, `RXNE`, `BR`) |
| `description` | EXACT description from datasheet |
| `bit_offset` | LSB position (0-indexed, e.g., `6` for bit 6) |
| `bit_width` | Number of bits (e.g., `1` for single bit, `3` for 3-bit field) |
| `access` | `"read-write"`, `"read-only"`, `"write-only"` — inherit from register if not specified |
| `enumerated_values` | Array of named values if datasheet defines them, else `[]` |

### Step 4: Handle Special Cases

**Reserved bits:**
- DO NOT include reserved bits as fields
- They will be inferred from gaps in bit coverage

**Write-clear / Read-clear flags:**
- Set `access` to `"read-writeOnce"` for write-1-to-clear flags
- Set `access` to `"read-only"` for hardware-set status flags

**Shared register names across peripherals:**
- Each peripheral gets its own register list — do not deduplicate

---

## CONSISTENCY REQUIREMENTS

1. **Addresses:** All addresses as lowercase hex strings with `0x` prefix
2. **Completeness:** Extract ALL peripherals with register tables. Missing peripherals = missing PAC coverage.
3. **Exactness:** Use EXACT names from datasheet. Do not normalize or abbreviate.
4. **Arrays:** `enumerated_values` MUST be an array, even if empty (`[]`)

---

## IF DATA NOT FOUND

- If the document has NO register tables: Return `{"error": "No register map found in document", "part_number": "...", "pages_searched": [...]}`
- If a peripheral has incomplete register data: Include partial data with `"incomplete": true` flag
- If reset value is not specified: Use `null`

---

## OUTPUT SCHEMA

Provide a SINGLE valid JSON object.

```json
{
  "part_details": {
    "part_number": "EXACT part number from document",
    "datasheet_revision": "Revision/date string or null",
    "description": "Brief component description"
  },
  "peripherals": [
    {
      "name": "SPI1",
      "description": "Serial Peripheral Interface 1",
      "base_address": "0x40013000",
      "source_page": 42,
      "incomplete": false,
      "registers": [
        {
          "name": "CR1",
          "description": "SPI control register 1",
          "offset": "0x00",
          "size": 32,
          "reset_value": "0x00000000",
          "access": "read-write",
          "fields": [
            {
              "name": "BIDIMODE",
              "description": "Bidirectional data mode enable",
              "bit_offset": 15,
              "bit_width": 1,
              "access": "read-write",
              "enumerated_values": [
                {"name": "Unidirectional", "value": 0, "description": "2-line unidirectional data mode selected"},
                {"name": "Bidirectional",  "value": 1, "description": "1-line bidirectional data mode selected"}
              ]
            },
            {
              "name": "SPE",
              "description": "SPI enable",
              "bit_offset": 6,
              "bit_width": 1,
              "access": "read-write",
              "enumerated_values": []
            }
          ]
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
- [ ] Every peripheral with a register table has an entry
- [ ] Every register in each peripheral is included
- [ ] Every field has `bit_offset` and `bit_width` (not just a description)
- [ ] All addresses are hex strings with `0x` prefix
- [ ] `enumerated_values` is always an array
