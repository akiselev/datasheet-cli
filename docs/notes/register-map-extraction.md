# Register Map & Peripheral Configuration Extraction — Research Note

Date: 2026-03-17

## 1. Goal

Add extraction tasks to datasheet-cli for:
1. **Register maps** — peripheral registers with addresses, bitfields, access types, reset values
2. **Peripheral configuration** — clock trees, pin muxing tables, DMA channel assignments

These feed into Rust firmware generation (svd2rust / chiptool PAC workflow).

---

## 2. Analysis of Register Map Data in Real Datasheets

### 2.1 Data Formats Observed

Examined CH32V003 (WCH, RISC-V) and RP2350 (Raspberry Pi, Cortex-M33 / Hazard3) datasheets from the ee-template project.

**RP2350 register definitions (pages 54-66 of RP2350A.pdf):**

The RP2350 datasheet uses a two-level structure:

1. **Register summary table** — lists all registers in a peripheral block:
   - Columns: Offset, Name, Info (description)
   - Example: `0x004 | GPIO_IN | Input value for GPIO0...31`

2. **Per-register bitfield table** — one section per register:
   - Header: `SIO: GPIO_HI_IN Register`, `Offset: 0x008`
   - Columns: Bits, Description, Type, Reset
   - Example row: `31:28 | QSPI_SD: Input value on QSPI SD0... | RO | 0x0`
   - Bit ranges like `31:28`, `27`, `23:16`, `15:0`
   - Access types: RO, RW, WO
   - Reset values in hex: `0x0`, `0x00000000`, `-` for reserved

This is the gold standard — highly structured, consistent formatting across hundreds of pages.

**CH32V003 register definitions (pages 20-30 of CH32V003F4P6.pdf):**

The CH32V003 datasheet is primarily in Chinese with a different structure:
- Memory map diagram showing peripheral base addresses (TIM2 at 0x40000000, WWDG at 0x40002C00, etc.)
- Register details are in the Reference Manual (separate document), not the pin-level datasheet
- The datasheet itself contains only the memory map overview and electrical characteristics

This is a critical finding: **many MCU datasheets separate the register reference from the pin/package datasheet**. The register map is typically in a "Reference Manual" or "Programming Manual", not the main datasheet.

### 2.2 Data Elements in Register Maps

A complete register definition contains:

| Element | Description | Example |
|---------|-------------|---------|
| Peripheral name | Block name | `SIO`, `UART0`, `GPIO` |
| Base address | Absolute address of peripheral | `0xd0000000` |
| Register name | Individual register | `GPIO_IN` |
| Register offset | Offset from base | `0x004` |
| Register width | Usually 32-bit | `32` |
| Register description | Human-readable purpose | "Input value for GPIO0...31" |
| Field name | Bitfield name | `QSPI_SD` |
| Bit range | MSB:LSB or single bit | `31:28`, `27`, `15:0` |
| Field description | Purpose of field | "Input value on QSPI SD0 (MOSI)..." |
| Access type | Read/write behavior | `RO`, `RW`, `WO`, `W1C`, `RC` |
| Reset value | Value after reset | `0x0`, `0x00000000` |
| Enumerated values | Named constants for field values | `{0: "Disabled", 1: "Enabled"}` |

### 2.3 Challenges for PDF Extraction

1. **Volume**: A typical MCU has 50-200+ peripherals, each with 5-50 registers. The RP2350 datasheet is 1300+ pages, with register definitions spanning hundreds of pages. This is far larger than any current extraction task.

2. **Cross-page tables**: Register tables frequently span multiple pages. The PDF splitter already handles this to some extent.

3. **Visual formatting**: Some datasheets use graphical bitfield diagrams (colored boxes showing bit positions) rather than tables. These are harder for LLMs to parse accurately.

4. **Inconsistency across vendors**: Every vendor formats register tables differently. ST uses one style, TI another, NXP yet another. WCH's Chinese-language datasheets are particularly challenging.

5. **Separation of documents**: Register maps are often in Reference Manuals (500-3000 pages), not in the main datasheet. The CH32V003 Reference Manual is a separate ~200-page PDF.

---

## 3. SVD File Availability per Manufacturer

SVD (System View Description) is an ARM-defined XML format (influenced by IP-XACT) that provides machine-readable register definitions. One SVD file describes an entire device: processor, peripherals, registers, and bitfields.

### 3.1 Availability Matrix

| Manufacturer | Chip Family | SVD Available? | Source | Quality | License |
|-------------|-------------|---------------|--------|---------|---------|
| **Raspberry Pi** | RP2040, RP2350 | Yes | pico-sdk GitHub repo | Excellent — generated from same source as datasheet | BSD |
| **Espressif** | ESP32, ESP32-C3, ESP32-C6, ESP32-S3 | Yes | github.com/espressif/svd | In-progress — may be missing peripherals/registers | Apache-2.0 |
| **WCH** | CH32V003, CH32V103, CH32V2xx, CH32V3xx | Partial | MounRiver Studio 2 (bundled), ch32-rs community | SVD exists but community-maintained YAML (ch32-data) is better curated | Mixed |
| **ST Micro** | All STM32 families | Yes | CMSIS packs, modm-io/cmsis-svd-stm32 | Good — official, but sometimes has errors that stm32-rs patches | Apache-2.0 |
| **Nordic** | nRF52, nRF53, nRF91 | Yes | CMSIS packs, nRF MDK | Good | Nordic license |
| **NXP** | LPC, i.MX RT, Kinetis | Yes | CMSIS packs | Good | BSD-3-Clause |
| **TI** | MSP432, SimpleLink | Partial | CCS/SDK bundles | Varies | TI license |
| **Microchip/Atmel** | SAM, PIC32, AVR | Partial | Atmel DFPs for SAM | SAM good, PIC32/AVR limited | Microchip license |
| **GigaDevice** | GD32 | Yes | CMSIS packs | Reasonable — some gaps | GD license |
| **SiFive** | FE310, U74 | Limited | SiFive Freedom SDK | Basic coverage | Apache-2.0 |

### 3.2 Aggregated SVD Repositories

- **cmsis-svd-data** (github.com/cmsis-svd/cmsis-svd-data): The main aggregation repo. Contains SVDs from dozens of manufacturers. Includes patched versions via svdtools.
- **stm32-rs** (github.com/stm32-rs/stm32-rs): STM32-specific, applies extensive patches to fix ST's SVD errors.
- **ch32-data** (github.com/ch32-rs/ch32-data): WCH-specific, manually curated YAML register definitions using chiptool format.

### 3.3 SVD for Chips in ee-template

| Chip | SVD Source | Notes |
|------|-----------|-------|
| ESP32-C6-WROOM-1 | github.com/espressif/svd/blob/main/svd/esp32c6.svd | Available, may be incomplete |
| CH32V003F4P6 | MounRiver Studio (CH32V003xx.svd), ch32-data YAML | SVD bundled in IDE; community YAML more actively maintained |
| RP2350A | pico-sdk repo (RP2350.svd) | High quality, generated from hardware description |

---

## 4. Comparison: SVD Download vs PDF Extraction

### 4.1 When to Use SVD Download

**Advantages:**
- Machine-readable, no LLM parsing errors
- Complete and structured (when available)
- Maintained by manufacturer or active community
- Directly consumable by svd2rust/chiptool
- Zero API cost (no Gemini calls)
- Fast — no multi-page PDF processing

**Best for:**
- Any chip with a known, high-quality SVD file
- Standard ARM Cortex-M parts (STM32, nRF, SAM, RP2040/RP2350)
- ESP32 family (with caveat about completeness)
- Automated pipelines where reliability is paramount

**Limitations:**
- Not all chips have SVDs (especially older or niche parts)
- SVDs can have errors (ST's SVDs are notorious for this — stm32-rs exists to patch them)
- SVDs may lag behind silicon revisions
- Non-ARM architectures (some RISC-V, 8051, etc.) may not have SVDs

### 4.2 When to Use PDF Extraction

**Advantages:**
- Works for ANY chip with a datasheet/reference manual
- Can capture information not in SVDs (register descriptions, usage notes, constraints)
- Can extract from vendor-specific formats (Chinese datasheets, unusual layouts)
- Handles "reference manual only" chips where no SVD exists

**Best for:**
- Chips without SVD files (many RISC-V, 8051, legacy parts)
- Analog/mixed-signal ICs with register-controlled features (codec ICs, sensor ICs like ADS1115, SHT40)
- Peripheral ICs controlled via I2C/SPI registers (not MCUs, but still have register maps)
- Verification/cross-checking of SVD data
- Extracting descriptive text and usage notes that SVDs don't capture

**Limitations:**
- LLM extraction is expensive (many API calls for large documents)
- Error-prone — bitfield details are easy to hallucinate
- Slow — hundreds of pages to process
- Inconsistent across vendors

### 4.3 Recommended Strategy: Hybrid Approach

```
For a given chip:
  1. Check if SVD exists (query known repos/URLs)
     -> If yes: download SVD, convert to JSON, done
     -> If partial: download SVD, extract missing parts from PDF
  2. If no SVD: extract register map from PDF (reference manual)
  3. For non-MCU ICs (sensors, codecs, etc.): always use PDF extraction
```

This suggests **two new commands** rather than one:

```bash
# Download SVD from known sources
datasheet svd download <part-number> --out extractions/registers/<part>.json

# Extract register map from PDF (fallback)
datasheet extract registers <pdf> -f --out extractions/registers/<part>.json

# Extract peripheral config (clock tree, pin mux, DMA)
datasheet extract peripheral-config <pdf> -f --out extractions/peripheral-config/<part>.json
```

---

## 5. Proposed Output JSON Schema

### 5.1 Design Goals

The schema should:
1. Be convertible to/from SVD XML (lossless round-trip where possible)
2. Map naturally to svd2rust / chiptool input formats
3. Capture information beyond SVD (descriptions, usage notes, constraints)
4. Be flat enough for Gemini's JSON schema depth limits (keep nesting under 5 levels)

### 5.2 Register Map Schema

```json
{
  "part_number": "RP2350",
  "datasheet_revision": "build-version: d126e9e-clean",
  "cpu": {
    "name": "Cortex-M33",
    "revision": "r0p4",
    "endian": "little",
    "mpu_present": true,
    "fpu_present": true,
    "nvic_priority_bits": 4,
    "address_width": 32
  },
  "peripherals": [
    {
      "name": "SIO",
      "description": "Single-cycle IO block",
      "base_address": "0xd0000000",
      "group_name": "SIO",
      "registers": [
        {
          "name": "CPUID",
          "offset": "0x000",
          "size": 32,
          "description": "Processor core identifier",
          "access": "read-only",
          "reset_value": "0x00000000",
          "fields": [
            {
              "name": "CPUID",
              "bit_range": "31:0",
              "bit_offset": 0,
              "bit_width": 32,
              "description": "Value is 0 when read from processor core 0, and 1 when read from processor core 1.",
              "access": "read-only",
              "reset_value": "0x0",
              "enumerated_values": []
            }
          ]
        },
        {
          "name": "GPIO_HI_IN",
          "offset": "0x008",
          "size": 32,
          "description": "Input value on GPIO32...47, QSPI IOs and USB pins",
          "access": "read-only",
          "reset_value": "0x00000000",
          "fields": [
            {
              "name": "QSPI_SD",
              "bit_range": "31:28",
              "bit_offset": 28,
              "bit_width": 4,
              "description": "Input value on QSPI SD0 (MOSI), SD1 (MISO), SD2 and SD3 pins",
              "access": "read-only",
              "reset_value": "0x0",
              "enumerated_values": []
            },
            {
              "name": "QSPI_CSN",
              "bit_range": "27:27",
              "bit_offset": 27,
              "bit_width": 1,
              "description": "Input value on QSPI CSn pin",
              "access": "read-only",
              "reset_value": "0x0",
              "enumerated_values": []
            },
            {
              "name": "GPIO",
              "bit_range": "15:0",
              "bit_offset": 0,
              "bit_width": 16,
              "description": "Input value on GPIO32...47",
              "access": "read-only",
              "reset_value": "0x0000",
              "enumerated_values": []
            }
          ]
        }
      ]
    }
  ]
}
```

### 5.3 Schema Design Decisions

1. **`bit_range` as string ("31:28")** — matches datasheet notation directly, easy to verify. Also provide computed `bit_offset` and `bit_width` integers for machine consumption.

2. **`access` uses SVD vocabulary** — `"read-only"`, `"write-only"`, `"read-write"`, `"writeOnce"`, `"read-writeOnce"`. This maps directly to SVD `<access>` element and to svd2rust access types. For datasheet abbreviations: `RO` -> `read-only`, `RW` -> `read-write`, `WO` -> `write-only`, `W1C` -> `read-write` (with note), `RC` -> `read-only` (with side-effect note).

3. **`enumerated_values` as array** — captures named constants when the datasheet defines them (e.g., `{value: 0, name: "DISABLED", description: "Feature disabled"}`). Often empty — many registers don't have named values in the datasheet.

4. **Flat peripheral list** — no nested peripheral groups. Grouping can be inferred from `group_name`. Keeps JSON nesting shallow for Gemini API limits.

5. **Hex strings for addresses and reset values** — preserves exact representation from datasheet. Parse to integers downstream.

### 5.4 Peripheral Configuration Schema

```json
{
  "part_number": "CH32V003",
  "clock_tree": {
    "oscillators": [
      {
        "name": "HSI",
        "type": "internal_rc",
        "frequency_hz": 24000000,
        "description": "Internal high-speed RC oscillator"
      },
      {
        "name": "HSE",
        "type": "external_crystal",
        "frequency_range_hz": [4000000, 25000000],
        "description": "External high-speed oscillator"
      },
      {
        "name": "LSI",
        "type": "internal_rc",
        "frequency_hz": 128000,
        "description": "Internal low-speed RC oscillator"
      }
    ],
    "plls": [
      {
        "name": "PLL",
        "source_mux": ["HSI", "HSE"],
        "source_register": "RCC_CFGR0.PLLSRC",
        "multiplier": 2,
        "output_name": "PLLCLK"
      }
    ],
    "system_clock": {
      "source_mux": ["HSI", "HSE", "PLLCLK"],
      "source_register": "RCC_CFGR0.SW",
      "max_frequency_hz": 48000000,
      "output_name": "SYSCLK"
    },
    "bus_prescalers": [
      {
        "name": "AHB_prescaler",
        "source": "SYSCLK",
        "register": "RCC_CFGR0.HPRE",
        "divisors": [1, 2, 4, 8, 16, 64, 128, 256],
        "output_name": "HCLK",
        "max_frequency_hz": 48000000
      }
    ],
    "peripheral_clocks": [
      {
        "peripheral": "ADC",
        "source": "HCLK",
        "prescaler_register": "RCC_CFGR0.ADCPRE",
        "divisors": [2, 4, 6, 8, 12, 16, 64, 96, 128]
      }
    ]
  },
  "pin_mux": [
    {
      "pin": "PA1",
      "functions": [
        {"function": "GPIO", "af_number": null, "default": true},
        {"function": "ADC_IN1", "af_number": null, "type": "analog"},
        {"function": "TIM1_CH2", "af_number": 0, "remap_register": "AFIO_PCFR1"},
        {"function": "TIM1_CH2", "af_number": 2, "remap_register": "AFIO_PCFR1"}
      ]
    }
  ],
  "dma_channels": [
    {
      "channel": 1,
      "peripheral_requests": [
        {"peripheral": "ADC", "request": "ADC", "direction": "peripheral_to_memory"},
        {"peripheral": "TIM2", "request": "TIM2_CH3", "direction": "peripheral_to_memory"}
      ]
    }
  ]
}
```

---

## 6. How This Feeds into Rust Firmware Generation

### 6.1 The svd2rust Pipeline

The standard Rust embedded workflow:

```
SVD XML  -->  svd2rust  -->  PAC crate (Peripheral Access Crate)
                              |
                              v
                         HAL crate (Hardware Abstraction Layer)
                              |
                              v
                         Application code
```

svd2rust generates type-safe Rust code where each register is a struct, each field has getter/setter methods, and access types are enforced at compile time. For example, a `read-only` register's generated struct will not have a `write()` method.

**Key svd2rust features:**
- Supports Cortex-M, MSP430, RISC-V, Xtensa targets via `--target` flag
- Generates interrupt enums from SVD interrupt definitions
- `--atomics` flag adds atomic set/clear/toggle operations
- Output is a complete Cargo crate

### 6.2 The chiptool Pipeline (Embassy)

```
YAML register defs  -->  chiptool  -->  metapac crate
                                         |
                                         v
                                    Embassy HAL
                                         |
                                         v
                                    Application code
```

Chiptool (used by Embassy) is a fork of svd2rust with key improvements:
- Uses YAML files instead of SVD XML as primary source format
- Supports merging/deduplicating registers across peripherals
- Generates "fieldset" structs that can be saved/loaded from variables
- Used for RP2040/RP2350, STM32, nRF PACs in the Embassy ecosystem

### 6.3 Converting Our JSON to SVD

For integration, we need a `json-to-svd` converter. The mapping is straightforward:

| Our JSON | SVD XML |
|----------|---------|
| `peripherals[].name` | `<peripheral><name>` |
| `peripherals[].base_address` | `<peripheral><baseAddress>` |
| `registers[].name` | `<register><name>` |
| `registers[].offset` | `<register><addressOffset>` |
| `registers[].access` | `<register><access>` |
| `fields[].name` | `<field><name>` |
| `fields[].bit_offset` | `<field><bitOffset>` |
| `fields[].bit_width` | `<field><bitWidth>` |
| `fields[].access` | `<field><access>` |
| `fields[].enumerated_values` | `<field><enumeratedValues>` |

This conversion could be a subcommand: `datasheet svd generate <json> --out <svd-file>`.

### 6.4 For Chips With Existing SVD Files

For chips like RP2350, ESP32-C6, STM32:
- Download the SVD directly
- Optionally convert to our JSON format for inspection/editing
- Feed into svd2rust/chiptool directly (no extraction needed)

For the ee-template firmware phase (Phase 10), the workflow would be:

```bash
# RP2350: use existing SVD
datasheet svd download RP2350 --source pico-sdk --out libs/RP2350.svd

# ESP32-C6: use Espressif SVD (may need supplementing)
datasheet svd download ESP32-C6 --source espressif --out libs/ESP32C6.svd

# CH32V003: use community SVD or ch32-data YAML
datasheet svd download CH32V003 --source ch32-rs --out libs/CH32V003.svd

# For I2C sensor (ADS1115) — no SVD, use PDF extraction
datasheet extract registers datasheets/ADS1115.pdf -f --out extractions/registers/ADS1115.json
```

---

## 7. Prompt Engineering for Register Map Extraction

### 7.1 Key Challenges

1. **Scale**: A single peripheral (e.g., UART) might have 15-30 registers with 5-10 fields each. Extracting a full MCU is hundreds of registers. Current extraction tasks produce ~100 lines of JSON. Register maps could produce 10,000+ lines.

2. **Precision requirements**: A single bit-offset error makes the generated code write to wrong bits. This is more critical than other extraction tasks where approximate values are acceptable.

3. **Table parsing**: Register bitfield tables are highly structured but can be visually complex (merged cells, multi-line descriptions, footnotes).

4. **Reserved fields**: Datasheets often list "Reserved" fields that must be preserved (they indicate "write as 0" or "do not modify" constraints).

### 7.2 Proposed Extraction Strategy

**Per-peripheral extraction** rather than whole-chip:

```bash
# Extract one peripheral at a time
datasheet extract registers <pdf> --peripheral UART -f --out extractions/registers/CHIP_UART.json
datasheet extract registers <pdf> --peripheral GPIO -f --out extractions/registers/CHIP_GPIO.json

# Or extract with page range hints
datasheet extract registers <pdf> --pages 54-82 -f --out extractions/registers/CHIP_SIO.json
```

This approach:
- Keeps each extraction within LLM context limits
- Allows targeted re-extraction if one peripheral has errors
- Produces manageable output sizes
- Aligns with how datasheets organize content (chapter per peripheral)

### 7.3 Prompt Structure

The prompt should:

1. **Demand exact copying** — register names, field names, and hex values must be copied character-for-character from the PDF. No normalization, no abbreviation.

2. **Use the two-pass pattern** — first identify the register summary table (offset + name + description), then fill in bitfield details per register. This matches how datasheets organize the content.

3. **Enumerate access type vocabulary** — provide explicit mapping table (RO -> read-only, etc.) since datasheets use inconsistent abbreviations.

4. **Require reserved fields** — explicitly instruct to include reserved/unused bit ranges.

5. **Include verification anchors** — ask for total register count, total field count, and checksum-like validations (e.g., "all fields in a 32-bit register should cover bits 31:0 with no gaps").

### 7.4 Anti-Hallucination Measures

Register extraction is high-risk for hallucination because:
- Bit positions are easy to get wrong (off-by-one, swapped MSB/LSB)
- Field names in similar registers blur together
- Reset values are frequently `0x0` which gives no verification signal

Mitigations:
- **Cross-reference register count**: "This peripheral has N registers starting at offset 0xNNN"
- **Verify bit coverage**: all fields in a register must tile bits 0-31 without overlap or gap
- **Verify offset monotonicity**: register offsets must be strictly increasing
- **Sample verification prompt**: "Before outputting, verify that register X at offset Y has exactly Z fields covering all 32 bits"

### 7.5 Handling the CH32V003 Case (Chinese Datasheets)

The CH32V003 datasheet is in Chinese. Observations:
- Gemini handles Chinese text well in extraction tasks
- The register tables themselves use English for register/field names (universal convention)
- Descriptions may be in Chinese — extract as-is, add optional `description_en` field
- The Reference Manual (separate PDF) contains the actual register definitions, not the pin-level datasheet

Recommendation: add a note in docs that for WCH chips, users should download the Reference Manual PDF, not just the datasheet.

---

## 8. Peripheral Configuration Extraction — Feasibility

### 8.1 Clock Trees

**Feasibility: Medium-High**

Clock tree diagrams are present in most MCU datasheets as block diagrams. The CH32V003 datasheet has a clear clock tree diagram on page 4 (Figure 1-3). The RP2350 has a detailed clocks chapter.

Challenges:
- Clock trees are often presented as **diagrams**, not tables. LLMs can interpret diagrams to some extent but accuracy varies.
- The data is highly interconnected (mux -> prescaler -> peripheral enable)
- Naming conventions vary wildly between vendors

The JSON schema (section 5.4) captures the essential data. A well-crafted prompt focusing on oscillator sources, PLL configuration, system clock mux, and bus prescalers should work for most MCUs.

### 8.2 Pin Muxing / Alternate Functions

**Feasibility: High**

Pin mux tables are already partially captured by the existing `pinout` extraction task (the `alternate_functions` array). The CH32V003 datasheet has a clear AF remapping table (Table 2-2, page 12).

What's needed beyond current pinout extraction:
- The AFIO/GPIO alternate function register bits that select each function
- Remap register names and bit values
- Conflict groups (which functions are mutually exclusive)

This could be an extension of the pinout extraction rather than a separate task.

### 8.3 DMA Channel Assignments

**Feasibility: Medium**

DMA channel-to-peripheral mapping tables exist in most MCU datasheets/reference manuals. They're typically simple lookup tables:

| DMA Channel | Peripheral Request | Direction |
|-------------|-------------------|-----------|
| CH1 | ADC | P->M |
| CH2 | SPI_TX | M->P |
| CH3 | SPI_RX | P->M |

This is straightforward tabular extraction. The main challenge is finding the table in the document (it's often buried deep in the DMA chapter).

### 8.4 Interrupt Vector Table

**Feasibility: High**

Interrupt tables are simple numbered lists:

| IRQ# | Name | Description |
|------|------|-------------|
| 0 | WWDG | Window Watchdog |
| 1 | PVD | PVD through EXTI |

These map directly to SVD `<interrupt>` elements and to svd2rust interrupt enums. Easy to extract, high value.

### 8.5 Recommended Priority

1. **Register map extraction** (from reference manual PDFs) — highest value, enables PAC generation
2. **SVD download** integration — for chips that have them, bypasses extraction entirely
3. **Interrupt vector table** — small, high-value, easy to extract
4. **DMA channel assignments** — medium effort, useful for HAL development
5. **Clock tree** — medium effort, complex output, but critical for initialization code
6. **Pin mux extensions** — extend existing pinout task rather than new task

---

## 9. Implementation Considerations for datasheet-cli

### 9.1 New Tasks

Add to `ExtractTask` enum:

```rust
enum ExtractTask {
    // ... existing tasks ...
    Registers,          // Register map extraction from PDF
    PeripheralConfig,   // Clock tree, DMA, interrupts
}
```

### 9.2 SVD Integration

New top-level subcommand (not an extraction task):

```bash
datasheet svd download <part> [--source <repo>] --out <file>
datasheet svd convert <svd-file> --to json --out <file>
datasheet svd convert <json-file> --to svd --out <file>
```

SVD sources to support:
- `pico-sdk` — RP2040, RP2350
- `espressif` — ESP32 family
- `cmsis-svd-data` — STM32, nRF, SAM, NXP, etc.
- `ch32-rs` — WCH CH32V/CH32X family
- `url` — arbitrary URL

### 9.3 PDF Splitting Considerations

Register map chapters in reference manuals can be 200-500 pages. The existing `pdf_split` module splits large PDFs into chunks. For register extraction:

- Split by peripheral chapter (ideal but requires detecting chapter boundaries)
- Alternatively, let the user specify `--pages` or `--peripheral` to scope extraction
- The merge strategy (`deep_merge` in extract.rs) works well for register data — each chunk produces a list of peripherals that can be concatenated

### 9.4 Verification Pipeline

After extraction, add a validation step:

```bash
datasheet validate registers <json-file>
```

Checks:
- All registers have monotonically increasing offsets
- All fields in each register tile the full register width without overlap
- Access types are from the allowed vocabulary
- Reset values are valid hex
- No duplicate register or field names within a peripheral

---

## 10. Open Questions

1. **Should register extraction target the datasheet or the reference manual?** For many MCUs these are separate PDFs. The reference manual is the right document but may not be what users have downloaded. Consider: should `datasheet mouser download` try to find the reference manual too?

2. **How to handle the "SVD exists but has errors" case?** ST's SVDs are infamous for missing fields, wrong access types, etc. The stm32-rs project maintains 100+ patch files. Should datasheet-cli apply known patches? Or should PDF extraction be used to generate a "diff" against the SVD to find discrepancies?

3. **Should peripheral config be one task or multiple?** Clock tree, pin mux, DMA, and interrupts are different enough that separate prompts may produce better results. But that means 4 separate Gemini API calls per chip.

4. **What about non-MCU register maps?** Sensor ICs (ADS1115, SHT40), flash chips (W25Q32JV), USB PHYs, etc. have I2C/SPI register maps that are much simpler (typically 8-bit registers, 10-50 registers total). These are a natural fit for PDF extraction and arguably more useful than MCU registers (since MCUs usually have SVDs). Should register extraction be optimized for this case first?

5. **Interaction with Phase 10 (firmware scaffold)?** The ee-template workflow generates firmware in Phase 10. If we can download SVD files in Phase 1 (part selection), the firmware phase can directly use svd2rust. What's the right place in the workflow to run register extraction?
