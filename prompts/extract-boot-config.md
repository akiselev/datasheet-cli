**CRITICAL REQUIREMENT:** You MUST analyze the actual PDF document provided. DO NOT hallucinate, guess, or use prior knowledge. ONLY extract information explicitly present in THIS document.

**Role:** Act as an Embedded Hardware Bring-up Engineer.

**Objective:** Extract "Strap" pins, Boot Mode configurations, and Debug interface requirements from the datasheet.

**Context:** The output will be used to configure boot mode hardware and debug interface connections on a custom PCB design.

---

## ANTI-HALLUCINATION VERIFICATION (MANDATORY)

Before generating ANY output, you MUST:
1. Verify you can read the PDF document
2. Extract the EXACT part number from the document
3. Include `part_number` in the output as proof of document reading
4. If you cannot read the PDF, respond with: `{"error": "Cannot read PDF document"}`
5. If NO boot configuration or strap pins exist, respond with: `{"error": "No boot configuration found", "part_number": "...", "pages_searched": [...]}`

---

## EXTRACTION INSTRUCTIONS

### Step 1: Identify ALL Strap/Boot Pins
Search for these section titles:
- "Boot Configuration", "Boot Mode", "Strap Pins", "Configuration Pins"
- "Power-On Reset Configuration", "Device Mode Selection"

For EACH strap pin found, extract:

| Field | Requirement |
|-------|-------------|
| `pin_name` | EXACT name as shown (e.g., "BOOT0", "STRAP1", "GPIO0") |
| `pin_number` | Physical pin number for each package variant |
| `function_description` | EXACT description from datasheet |
| `logic_states` | Map of logic level to function (e.g., {"0": "Flash Boot", "1": "UART Boot"}) |
| `hardware_requirement` | Required external components (resistors, etc.) |
| `sampling_time` | When pin is sampled (e.g., "At power-on reset", "On rising edge of RESET") |
| `source_page` | 0-indexed page number where this information appears |

### Step 2: Extract Debug Interface Information
Search for: "Debug", "JTAG", "SWD", "Serial Wire", "Boundary Scan"

For EACH debug interface, extract:
- Protocol name (JTAG, SWD, cJTAG, etc.)
- ALL required pins with their physical pin numbers
- Required external components (pull-ups, pull-downs, series resistors)
- Voltage levels and signal requirements

### Step 3: Extract Reset Requirements
Search for: "Reset", "Power-On Reset", "POR", "NRST"

Extract:
- Reset pin identification
- Required external circuitry (capacitors, resistors)
- Timing requirements (minimum pulse width, etc.)

---

## CONSISTENCY REQUIREMENTS

1. **Ordering:** List strap pins in pin number order (ascending)
2. **Completeness:** Include ALL strap pins, even those with default states
3. **Exactness:** Use EXACT text from datasheet for descriptions
4. **Logic States:** Express as object with string keys ("0", "1", "HIGH", "LOW")

---

## IF DATA NOT FOUND

- If debug interface is not documented: Set `"debug_interface": null`
- If reset requirements are not specified: Set `"reset_requirements": null`
- If a strap pin's resistor value is not specified: Use `"not specified"` as the value
- If this device has NO boot configuration pins: Return error response (see above)

---

## OUTPUT SCHEMA

Provide a SINGLE valid JSON object:

```json
{
  "part_number": "EXACT part number from document",
  "source_pages": [5, 12, 45],
  "boot_configuration": [
    {
      "pin_name": "BOOT0",
      "pin_number": "44",
      "function_description": "Selects boot memory source",
      "logic_states": {
        "0": "Boot from User Flash",
        "1": "Boot from System Memory (Bootloader)"
      },
      "hardware_requirement": "External 10k pull-down resistor to set default state",
      "sampling_time": "Sampled on fourth rising edge of SYSCLK after reset",
      "source_page": 12
    },
    {
      "pin_name": "BOOT1",
      "pin_number": "PB2",
      "function_description": "Secondary boot source selection",
      "logic_states": {
        "0": "When BOOT0=1: Boot from System Memory",
        "1": "When BOOT0=1: Boot from embedded SRAM"
      },
      "hardware_requirement": "Use 10k pull-up or pull-down based on desired mode",
      "sampling_time": "Sampled with BOOT0",
      "source_page": 12
    }
  ],
  "debug_interface": {
    "protocol": "SWD",
    "pins": {
      "SWCLK": {"pin_number": "20", "direction": "Input"},
      "SWDIO": {"pin_number": "21", "direction": "Bidirectional"}
    },
    "external_components": "Internal pull-ups present. No external components required.",
    "voltage_levels": "3.3V logic levels",
    "notes": "SWO (Serial Wire Output) available on PA3 for trace output",
    "source_page": 45
  },
  "reset_requirements": {
    "pin_name": "NRST",
    "pin_number": "7",
    "external_circuit": "100nF capacitor to GND recommended",
    "timing": "Minimum 20us low pulse for valid reset",
    "internal_features": "Internal pull-up and noise filter present",
    "source_page": 5
  }
}
```

---

## FINAL CHECKLIST

Before submitting, verify:
- [ ] `part_number` matches document exactly
- [ ] ALL strap/boot pins in the document are included
- [ ] Pin numbers match the document exactly
- [ ] Logic state mappings are complete (all combinations documented)
- [ ] Debug pin assignments are accurate
- [ ] Source page numbers are 0-indexed and accurate
