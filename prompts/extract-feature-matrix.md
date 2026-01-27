**CRITICAL REQUIREMENT:** You MUST analyze the actual PDF document provided. DO NOT hallucinate, guess, or use prior knowledge. ONLY extract information explicitly present in THIS document.

**Role:** Act as a Senior Component Engineer and Systems Architect.

**Objective:** Dissect the datasheet to extract the precise "Feature Matrix," Part Number Decoding logic, and Variant differences. The goal is to determine exactly which capabilities apply to which specific orderable part number.

**Context:** The output will be used by an automated design agent to select the correct specific part number (MPN) for a design requirements list, or to validate that a selected part actually supports the required interfaces.

---

## ANTI-HALLUCINATION VERIFICATION (MANDATORY)

Before generating ANY output, you MUST:
1. Verify you can read the PDF document
2. Extract the EXACT part number family from the document
3. Include `family_name` in the output as proof of document reading
4. If you cannot read the PDF, respond with: `{"error": "Cannot read PDF document"}`
5. If this is a single-variant device with no ordering matrix: Include the single variant's specifications

---

## EXTRACTION INSTRUCTIONS

### Step 1: Extract Part Number Decoding
Locate "Ordering Information", "Part Numbering", or "Device Nomenclature" section.

For EACH position in the part number, extract:

| Field | Requirement |
|-------|-------------|
| `position` | Position name (e.g., "Prefix", "Suffix 1", "Character 5-6") |
| `meaning` | What this position represents |
| `values` | Object mapping code to meaning (e.g., {"T": "LQFP-64"}) |

### Step 2: Extract Device Variants
Locate "Device Comparison", "Product Family", or feature comparison table.

For EACH variant, extract:

| Field | Requirement |
|-------|-------------|
| `root_part_number` | Base part number (e.g., "STM32F407") |
| `package_options` | Array of available packages |
| `memory_flash` | Flash memory size |
| `memory_ram` | RAM size |
| `key_features` | Object of boolean feature flags |
| `source_page` | 0-indexed page number |

### Step 3: Extract Key Features as Boolean Flags
For the `key_features` object, standardize these feature names:

**Communication Interfaces:**
- `usb_otg_fs`: USB OTG Full Speed
- `usb_otg_hs`: USB OTG High Speed
- `ethernet_mac`: Ethernet MAC
- `can`: CAN bus
- `can_fd`: CAN-FD
- `i2c_count`: Number of I2C interfaces
- `spi_count`: Number of SPI interfaces
- `uart_count`: Number of UART interfaces

**Special Peripherals:**
- `camera_interface`: Digital camera interface (DCMI)
- `lcd_controller`: LCD/TFT controller
- `crypto_engine`: Hardware cryptography
- `hash_engine`: Hardware hash
- `rng`: Random number generator
- `dac_count`: Number of DAC channels
- `adc_channels`: Number of ADC channels

**Wireless (if applicable):**
- `bluetooth`: Bluetooth support
- `wifi`: WiFi support
- `lora`: LoRa support

### Step 4: Extract Environmental/Quality Grades
Identify temperature range and qualification codes:

| Code Type | Examples |
|-----------|----------|
| Commercial | 0 to 70°C |
| Industrial | -40 to 85°C |
| Extended | -40 to 105°C |
| Automotive (AEC-Q100) | -40 to 125°C |

---

## CONSISTENCY REQUIREMENTS

1. **Ordering:** List variants in the order they appear in the comparison table
2. **Completeness:** Include ALL variants shown in the document
3. **Boolean Features:** Use `true`/`false` for feature presence, not strings
4. **Counts:** Use integers for counts (e.g., `"uart_count": 4`)
5. **Memory:** Use consistent format (e.g., "1MB", "256KB")

---

## IF DATA NOT FOUND

- If no comparison table exists: Extract features from the single device described
- If a feature is not mentioned: Omit from `key_features` (do not assume false)
- If part numbering is not explained: Set `"part_number_decoding": null`
- If package options are unclear: List only those explicitly mentioned

---

## OUTPUT SCHEMA

Provide a SINGLE valid JSON object:

```json
{
  "family_name": "EXACT family name from document (e.g., STM32F4)",
  "source_pages": [3, 8, 15],
  "part_number_decoding": {
    "example_full_part": "STM32F407VGT6",
    "prefix": "STM32F407",
    "fields": [
      {
        "position": "Character 10 (Package)",
        "meaning": "Package Type",
        "values": {
          "V": "LQFP-100",
          "Z": "LQFP-144",
          "I": "BGA-176"
        }
      },
      {
        "position": "Character 11 (Flash)",
        "meaning": "Flash Size",
        "values": {
          "E": "512KB",
          "G": "1MB"
        }
      },
      {
        "position": "Character 12 (Temperature)",
        "meaning": "Temperature Range",
        "values": {
          "6": "Industrial (-40 to 85°C)",
          "7": "Industrial (-40 to 105°C)"
        }
      }
    ]
  },
  "variants": [
    {
      "root_part_number": "STM32F407",
      "description": "High-performance with Ethernet MAC",
      "package_options": ["LQFP-100", "LQFP-144", "BGA-176"],
      "memory_flash": "1MB",
      "memory_ram": "192KB",
      "key_features": {
        "ethernet_mac": true,
        "usb_otg_hs": true,
        "usb_otg_fs": true,
        "camera_interface": true,
        "crypto_engine": false,
        "can": true,
        "i2c_count": 3,
        "spi_count": 3,
        "uart_count": 4,
        "adc_channels": 16
      },
      "source_page": 8
    },
    {
      "root_part_number": "STM32F405",
      "description": "High-performance without Ethernet",
      "package_options": ["LQFP-64", "LQFP-100"],
      "memory_flash": "1MB",
      "memory_ram": "192KB",
      "key_features": {
        "ethernet_mac": false,
        "usb_otg_hs": true,
        "usb_otg_fs": true,
        "camera_interface": false,
        "crypto_engine": false,
        "can": true,
        "i2c_count": 3,
        "spi_count": 3,
        "uart_count": 4,
        "adc_channels": 16
      },
      "source_page": 8
    }
  ],
  "interface_support_summary": {
    "usb_support": "USB 2.0 OTG FS on all variants. HS requires external PHY (ULPI).",
    "ethernet_support": "10/100 Ethernet MAC on F407/F417. Requires external PHY.",
    "wireless_support": "None (external module required)",
    "special_notes": "F417 adds hardware crypto (AES, DES, TDES) and hash (MD5, SHA1)"
  }
}
```

---

## FINAL CHECKLIST

Before submitting, verify:
- [ ] `family_name` matches document exactly
- [ ] ALL variants in the comparison table are included
- [ ] Part number decoding covers all character positions
- [ ] Feature flags use boolean values (not strings)
- [ ] Memory sizes use consistent format
- [ ] Source page numbers are 0-indexed and accurate
