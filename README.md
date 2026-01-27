# datasheet-cli

A command-line tool that extracts structured data from PDF datasheets using LLMs. Point it at a datasheet, get back JSON you can use in your tooling.

```bash
# Extract pinout data from a microcontroller datasheet
datasheet extract pinout STM32F407.pdf -f

# Get PCB footprint dimensions
datasheet extract footprint TPS62840.pdf --out footprint.json

# Search Mouser for a part and download its datasheet
datasheet mouser download LM5164
```

## Why?

Every electronics engineer has done this: you're designing a PCB, you need the pinout for a new chip, and you spend 20 minutes scrolling through a 500-page PDF to find Table 12 buried on page 47. Then you manually transcribe it into your symbol editor, probably making a typo that you won't catch until after the boards are fabbed.

Datasheets are the lifeblood of hardware design, but they're trapped in PDFs that were designed for humans to read, not machines to parse. This tool uses Gemini's vision capabilities to extract the data you actually need:

- **Pinout & configuration** - Every pin with electrical type, alternate functions, and groupings
- **Footprint geometry** - Package dimensions, pad sizes, and land pattern recommendations
- **Electrical specs** - Absolute max ratings, operating conditions, thermal data
- **Power requirements** - Voltage rails, sequencing rules, decoupling capacitors
- **High-speed constraints** - Impedance targets, length matching, termination requirements
- **And more** - DRC rules, boot configuration, reference design BOM

The output is structured JSON that you can pipe into your CAD tools, symbol generators, design rule checkers, or documentation pipelines.

## Installation

### From source (requires Rust 1.85+)

```bash
git clone https://github.com/akiselev/datasheet-cli
cd datasheet-cli
cargo install --path .
```

### Binary releases

Coming soon.

## Quick Start

1. Get a [Google AI Studio API key](https://aistudio.google.com/apikey) (free tier works)

2. Set your API key:
   ```bash
   export GOOGLE_API_KEY="your-api-key"
   # or
   export GEMINI_API_KEY="your-api-key"
   ```

3. Extract data:
   ```bash
   datasheet extract pinout ~/datasheets/STM32F407VG.pdf -f
   ```

That's it. The tool handles uploading the PDF to Gemini, manages caching (PDFs are cached for 48 hours to avoid re-uploads), and returns structured JSON.

## Extraction Tasks

### `pinout` - Pin Configuration

Extracts complete pin tables for schematic symbol generation.

```bash
datasheet extract pinout LM5164.pdf -f
```

```json
{
  "part_details": {
    "part_number": "LM5164",
    "datasheet_revision": "SNVSBJ4B - March 2023"
  },
  "packages": [{
    "package_name": "SOT-23-6",
    "total_pin_count": 6,
    "pins": [
      {
        "pin_number": "1",
        "pin_name": "SW",
        "electrical_type": "Power Output",
        "functional_group": "Power",
        "description": "Switching node. Connect to inductor.",
        "alternate_functions": []
      }
    ]
  }]
}
```

### `footprint` - Package Dimensions

Extracts mechanical data for PCB footprint generation.

```bash
datasheet extract footprint TPS62840.pdf -f
```

```json
{
  "packages": [{
    "package_code": "YKE",
    "package_name": "DSBGA-4",
    "component_dimensions": {
      "body_description": "1.2mm x 0.8mm x 0.5mm (L x W x H), tolerance +/-0.05mm",
      "pin_pitch_description": "0.4mm vertical pitch, 0.5mm horizontal pitch",
      "pin_1_orientation": "Top-left corner identified by A1 marking"
    },
    "land_pattern_geometry": {
      "pad_shape_description": "Rectangular pads with 0.05mm corner radius",
      "pad_dimensions_mm": "0.25mm x 0.25mm nominal",
      "solder_mask_instructions": "NSMD preferred. Mask opening = pad size."
    }
  }]
}
```

### `characteristics` - Electrical Specifications

Extracts parametric data for simulation and design verification.

```bash
datasheet extract characteristics TPS62840.pdf -f
```

```json
{
  "absolute_maximum_ratings": [
    {
      "parameter": "Input Voltage (VIN)",
      "symbol": "VIN",
      "limit_min": "-0.3V",
      "limit_max": "6.5V"
    }
  ],
  "electrical_specifications": [
    {
      "parameter_name": "Quiescent Current",
      "symbol": "Iq",
      "typ_value": "60nA",
      "max_value": "120nA",
      "test_conditions": "VIN=3.6V, No Load, Enable=High"
    }
  ],
  "thermal_data": []
}
```

### `power` - Power Supply Requirements

Extracts power sequencing and decoupling requirements.

```bash
datasheet extract power ATSAM4S.pdf -f
```

### `high-speed` - Routing Constraints

Extracts impedance, length matching, and termination requirements for USB, Ethernet, DDR, etc.

```bash
datasheet extract high-speed STM32H7.pdf -f
```

### `custom` - Your Own Prompts

Use your own extraction prompt and JSON schema:

```bash
datasheet extract custom datasheet.pdf \
  --prompt "Extract the I2C address configuration options" \
  --schema schema.json
```

### All Tasks

| Task | Description |
|------|-------------|
| `pinout` | Pin configuration for schematic symbols |
| `footprint` | Package dimensions for PCB footprints |
| `characteristics` | Electrical and thermal specifications |
| `power` | Power rails, sequencing, decoupling |
| `high-speed` | High-speed interface routing constraints |
| `drc-rules` | PCB design rule constraints |
| `boot-config` | Boot mode and configuration pins |
| `layout-constraints` | Component placement rules |
| `reference-design` | Reference schematic BOM |
| `feature-matrix` | Part variant comparison |
| `custom` | User-defined extraction |

## Distributor Integration

### Mouser

Search parts and download datasheets directly:

```bash
# Search by keyword
datasheet mouser search "STM32F4"

# Get detailed part info
datasheet mouser part 511-STM32F407VGT6

# Download datasheet
datasheet mouser download 511-STM32F407VGT6 --dir ./datasheets
```

Requires: `MOUSER_API_KEY` ([Get one here](https://www.mouser.com/apihub/))

### DigiKey

```bash
# Search parts
datasheet digikey search "LM5164"

# Get part details
datasheet digikey part LM5164DDAR

# Download datasheet
datasheet digikey download LM5164DDAR
```

Requires: `DIGIKEY_CLIENT_ID` and `DIGIKEY_CLIENT_SECRET` ([Register here](https://developer.digikey.com/))

## Pipeline Examples

### Generate KiCad symbols

```bash
datasheet extract pinout STM32F407.pdf | python generate_kicad_symbol.py > STM32F407.kicad_sym
```

### Batch process a directory

```bash
for pdf in datasheets/*.pdf; do
  base=$(basename "$pdf" .pdf)
  datasheet extract footprint "$pdf" --out "footprints/${base}.json"
done
```

### Compare power requirements across parts

```bash
for part in TPS62840 TPS62842 TPS62844; do
  datasheet mouser download "$part"
  datasheet extract characteristics "${part}.pdf" --out "${part}_specs.json"
done
jq -s '.' *_specs.json > comparison.json
```

## Options

```
datasheet extract <TASK> <PDF> [OPTIONS]

Options:
  --model <MODEL>       Gemini model (default: gemini-3-pro-preview)
  --out <FILE>          Output file (default: stdout)
  -f, --formatted       Pretty-print JSON
  --prompt <TEXT|FILE>  Custom prompt (for 'custom' task)
  --schema <JSON|FILE>  Custom JSON schema (for 'custom' task)
  --no-cache            Disable PDF caching (re-upload each time)
  --api-key <KEY>       API key (default: $GOOGLE_API_KEY or $GEMINI_API_KEY)
```

## Caching

PDFs are uploaded to Gemini's File API and cached locally for 48 hours. This means:
- First extraction of a new PDF: uploads the file (~1-10 seconds depending on size)
- Subsequent extractions of the same PDF: uses cached reference (instant)

Cache location: `~/.cache/datasheet-cli/` (Linux) or platform equivalent.

To force re-upload: `--no-cache`

## Accuracy

The prompts are designed with anti-hallucination measures:
- Every extraction includes verification data (part number, revision) that you can check
- Missing data is explicitly marked as `null` or `"not specified"` rather than guessed
- Prompts instruct the model to return errors if key sections aren't found

That said, this is an LLM - always verify critical dimensions before sending boards to fab.

## Model Selection

The default model is `gemini-3-pro-preview`. You can override with `--model`:

```bash
datasheet extract pinout datasheet.pdf --model gemini-2.0-flash-exp
```

For large datasheets (500+ pages), the newer Gemini models with expanded context windows work best.

## Cost

Gemini has a generous free tier. For typical usage:
- ~1000 datasheet extractions/month on free tier
- ~$0.001-0.01 per extraction on paid tier

The exact cost depends on PDF size and model used.

## Limitations

- Only works with Gemini (no OpenAI/Anthropic support currently)
- PDFs must be readable (not scanned images without OCR)
- Some older/unusual datasheet formats may not extract cleanly
- Maximum PDF size depends on Gemini's limits (~100MB)

## Contributing

Contributions welcome. Areas that could use help:

- Additional extraction prompts for specific use cases
- Integration with other CAD tools (Altium, Eagle, OrCAD)
- Support for other LLM providers
- Better error handling and validation

## License

GPL-3.0-only. See [LICENSE](LICENSE) for details.

## See Also

- [kicad-footprint-generator](https://github.com/pointhi/kicad-footprint-generator) - Generate KiCad footprints from parameters
- [KicadModTree](https://gitlab.com/kicad/libraries/kicad-footprint-generator) - Python library for KiCad footprint scripts
