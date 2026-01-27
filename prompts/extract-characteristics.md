**CRITICAL REQUIREMENT:** You MUST analyze the actual PDF document provided. DO NOT hallucinate, guess, or use prior knowledge. ONLY extract information explicitly present in THIS document.

**Role:** Act as a Senior Analog Design Engineer and Reliability Specialist.

**Objective:** Extract precise parametric electrical and thermal data from the datasheet to populate a simulation model and power budget calculator.

**Context:** The extracted data will be used for SPICE modeling, thermal analysis, and design margin calculations. Accuracy is critical.

---

## ANTI-HALLUCINATION VERIFICATION (MANDATORY)

Before generating ANY output, you MUST:
1. Verify you can read the PDF document
2. Extract the EXACT part number from the document
3. Include `part_number` in the output as proof of document reading
4. If you cannot read the PDF, respond with: `{"error": "Cannot read PDF document"}`
5. If NO electrical specifications exist, respond with: `{"error": "No electrical specifications found", "part_number": "...", "pages_searched": [...]}`

---

## EXTRACTION INSTRUCTIONS

### Step 1: Extract Absolute Maximum Ratings (EXHAUSTIVE)
Locate the "Absolute Maximum Ratings" table. For EACH parameter:

| Field | Requirement |
|-------|-------------|
| `parameter` | EXACT name as shown (e.g., "Input Voltage (VIN)") |
| `symbol` | Symbol if provided (e.g., "VIN", "IOUT") |
| `limit_min` | Minimum limit with unit (e.g., "-0.3V") or null |
| `limit_max` | Maximum limit with unit (e.g., "6.0V") |
| `condition` | Test condition exactly as stated |
| `notes` | Any warnings or additional notes |
| `source_page` | 0-indexed page number |

### Step 2: Extract Recommended Operating Conditions
Locate "Recommended Operating Conditions" table. For EACH parameter:

| Field | Requirement |
|-------|-------------|
| `parameter` | EXACT name as shown |
| `symbol` | Symbol if provided |
| `range_min` | Minimum value with unit |
| `range_max` | Maximum value with unit |
| `notes` | Any conditions or warnings |

### Step 3: Extract Electrical Specifications (DC & AC)
Locate "Electrical Characteristics" or "DC/AC Characteristics" tables. For EACH parameter:

| Field | Requirement |
|-------|-------------|
| `parameter_name` | EXACT name (e.g., "Quiescent Current") |
| `symbol` | Symbol (e.g., "Iq", "VOS") |
| `test_conditions` | EXACT conditions (e.g., "Enable=High, No Load, 25°C") |
| `min_value` | Minimum value with unit, or null if not specified |
| `typ_value` | Typical value with unit, or null if not specified |
| `max_value` | Maximum value with unit, or null if not specified |
| `unit` | Unit of measurement |
| `source_page` | 0-indexed page number |

### Step 4: Extract Thermal Data
Locate "Thermal Information" or "Package Thermal Data". For EACH package:

| Field | Requirement |
|-------|-------------|
| `package_type` | Package name (e.g., "DSBGA", "SOIC-8") |
| `theta_ja` | Junction-to-ambient thermal resistance with unit |
| `theta_jc` | Junction-to-case thermal resistance (if provided) |
| `theta_jb` | Junction-to-board thermal resistance (if provided) |
| `psi_jt` | Junction-to-top thermal characterization (if provided) |
| `max_junction_temp` | Maximum junction temperature |
| `power_dissipation` | Maximum power dissipation (if provided) |
| `test_conditions` | Board type, airflow conditions, etc. |

### Step 5: Describe Performance Graphs (Optional)
If "Typical Performance Characteristics" graphs exist, describe key trends:
- Graph title
- X-axis and Y-axis parameters
- Key trend description (e.g., "Linear increase", "Exponential decay")
- Notable values or inflection points

---

## CONSISTENCY REQUIREMENTS

1. **Ordering:** List parameters in the order they appear in each table
2. **Completeness:** Extract ALL rows from specification tables
3. **Exactness:** Preserve exact parameter names (do not normalize or simplify)
4. **Units:** Always include units with values (e.g., "25uA" not "25")
5. **Null Values:** Use `null` for unspecified min/typ/max, not empty strings or zeros

---

## IF DATA NOT FOUND

- If a table type is not in the document: Omit that array (e.g., no `thermal_data` key)
- If a parameter has no typical value: Use `"typ_value": null`
- If conditions are not specified: Use `"test_conditions": "not specified"`
- If units are unclear: Include original text and add `"unit_uncertain": true`

---

## OUTPUT SCHEMA

Provide a SINGLE valid JSON object:

```json
{
  "part_number": "EXACT part number from document",
  "datasheet_revision": "Revision/date from document",
  "absolute_maximum_ratings": [
    {
      "parameter": "Input Voltage (VIN)",
      "symbol": "VIN",
      "limit_min": "-0.3V",
      "limit_max": "6.0V",
      "condition": "Referenced to GND",
      "notes": "Exceeding may cause permanent damage",
      "source_page": 3
    },
    {
      "parameter": "Junction Temperature",
      "symbol": "TJ",
      "limit_min": null,
      "limit_max": "150°C",
      "condition": null,
      "notes": null,
      "source_page": 3
    }
  ],
  "recommended_operating_conditions": [
    {
      "parameter": "Input Voltage",
      "symbol": "VIN",
      "range_min": "2.2V",
      "range_max": "5.5V",
      "notes": "Device may not regulate correctly below minimum"
    }
  ],
  "electrical_specifications": [
    {
      "parameter_name": "Quiescent Current",
      "symbol": "Iq",
      "test_conditions": "Enable=High, No Load, VIN=3.3V, TA=25°C",
      "min_value": null,
      "typ_value": "25uA",
      "max_value": "40uA",
      "unit": "uA",
      "source_page": 4
    },
    {
      "parameter_name": "Output Voltage Accuracy",
      "symbol": "VOUT",
      "test_conditions": "VIN=VOU+0.5V to 5.5V, IOUT=1mA to 150mA",
      "min_value": "-2%",
      "typ_value": null,
      "max_value": "+2%",
      "unit": "%",
      "source_page": 4
    }
  ],
  "thermal_data": [
    {
      "package_type": "DSBGA-4",
      "theta_ja": "180°C/W",
      "theta_jc": "15°C/W",
      "theta_jb": null,
      "psi_jt": null,
      "max_junction_temp": "125°C",
      "power_dissipation": "0.7W",
      "test_conditions": "JEDEC standard 4-layer board, still air"
    }
  ],
  "performance_trends": [
    {
      "graph_title": "Quiescent Current vs Temperature",
      "x_axis": "Temperature (°C)",
      "y_axis": "Iq (uA)",
      "trend_description": "Iq increases approximately linearly with temperature, from ~20uA at -40°C to ~35uA at 125°C"
    }
  ]
}
```

---

## FINAL CHECKLIST

Before submitting, verify:
- [ ] `part_number` matches document exactly
- [ ] ALL parameters from each table are included
- [ ] Units are included with all values
- [ ] Test conditions are preserved exactly as written
- [ ] Null is used for unspecified values (not empty strings)
- [ ] Source page numbers are accurate (0-indexed)
