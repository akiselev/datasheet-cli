**CRITICAL REQUIREMENT:** You MUST analyze the actual PDF document provided. DO NOT hallucinate, guess, or use prior knowledge. ONLY extract information explicitly present in THIS document.

**Role:** Act as a Senior Hardware Systems Engineer performing netlist extraction.

**Objective:** Extract the complete circuit topology from the "Typical Application Circuit" or "Reference Design" schematic diagram in this datasheet. The output must be a structured netlist — a graph of components and the electrical nets connecting them — that can be directly used to generate a schematic.

**Context:** This output will be consumed by an LLM agent that generates Altium `.schdoc-spec` schematic files. The agent needs to know exactly which pins connect to which nets, with no ambiguity.

---

## ANTI-HALLUCINATION VERIFICATION (MANDATORY)

Before generating ANY output, you MUST:
1. Verify you can read the PDF document
2. Extract the EXACT part number from the document
3. If you cannot read the PDF, respond with: `{"error": "Cannot read PDF document"}`
4. If NO application circuit exists, respond with: `{"error": "No application circuit found", "part_number": "...", "pages_searched": [...]}`

---

## EXTRACTION INSTRUCTIONS

### Step 1: Locate Application Circuit Diagrams
Search for these sections (in order of preference):
- "Typical Application Circuit" / "Typical Application"
- "Application Schematic" / "Application Diagram"
- "Reference Design" / "Recommended Circuit"
- "Simplified Schematic"
- "Evaluation Board Schematic"

Record the page number(s) where each circuit appears.

### Step 2: Identify All Components
For EACH component visible in the schematic diagram:

**For the main IC (U1):**
- List ALL pins shown in the diagram with their exact names
- Include NC (no-connect) pins if shown
- Include exposed/thermal pads (DAP, EP, GND pad) if shown

**For external components (capacitors, resistors, inductors, diodes, etc.):**
- Record the reference designator (C1, R1, L1, D1, etc.)
- Record the exact value shown on the schematic
- Record the component type
- Identify each pin: for 2-pin unpolarized components use "1" and "2"; for polarized capacitors use "P" (positive) and "N" (negative); for diodes use "A" (anode) and "K" (cathode); for MOSFETs use "G", "D", "S"; for BJTs use "B", "C", "E"

### Step 3: Trace Every Net (THIS IS THE CRITICAL STEP)
A "net" is an electrical connection — a wire or set of connected wires that forms one electrical node. Every point where wires meet (indicated by a junction dot or a T-junction) is part of the same net.

For EACH net in the circuit:
1. **Name it:** Use the label shown in the schematic (VIN, VOUT, GND, SW, FB, BOOT, etc.). If a net has no label, create a descriptive name like "SW_NODE" or "FB_DIVIDER".
2. **Type it:** Classify as:
   - `power_input` — external power supply connection (VIN, VCC, VBUS)
   - `power_output` — regulated output (VOUT, 3V3, 5V)
   - `ground` — ground connections (GND, PGND, AGND)
   - `signal` — signal connections (EN, PG, FAULT)
   - `internal` — internal circuit nodes (switching node, feedback divider midpoint, bootstrap)
3. **List ALL connections:** Every component pin that touches this net

**Ground net rules:**
- All ground symbols connect to ONE net named "GND" (unless the schematic explicitly shows separate analog/digital grounds)
- Include the IC's GND pin(s), exposed pad, and all component pins connected to ground symbols

**Power rail rules:**
- Input power and output power are separate nets even if they have similar voltage
- Follow the wires carefully — VIN and VOUT are NOT the same net

### Step 4: Verify Connectivity
Before finalizing, perform these checks:
- **Pin count check:** Every component pin must appear in exactly one net. A 2-pin capacitor has 2 pins — both must be assigned to nets.
- **No floating pins:** Every component pin in the diagram must be connected to a net, unless explicitly marked NC (no-connect).
- **IC pin coverage:** Every IC pin shown in the application circuit must appear in a net (or be listed as NC).
- **Net size check:** Every net should have at least 2 connections (a single-connection net is a floating wire — likely an error). Exception: nets that connect to off-board connectors (VIN, VOUT, EN).

---

## OUTPUT FORMAT

Return a SINGLE valid JSON object with this structure:

```json
{
  "part_number": "EXACT part number from document",
  "source_pages": [13],
  "circuits": [
    {
      "circuit_name": "Descriptive name from the diagram title",
      "circuit_type": "typical or alternative",
      "source_page": 13,
      "description": "Brief description of what this circuit does",
      "design_parameters": [
        {"parameter": "Input voltage", "value": "10.8V to 19.8V"},
        {"parameter": "Output voltage", "value": "5V"},
        {"parameter": "Output current", "value": "3A"}
      ],
      "components": [
        {
          "designator": "U1",
          "type": "ic",
          "part_number": "TPS5430DDA",
          "value": null,
          "description": "Main IC",
          "pins": ["VIN", "ENA", "BOOT", "PH", "VSENSE", "GND"]
        },
        {
          "designator": "C1",
          "type": "capacitor",
          "part_number": null,
          "value": "10uF",
          "description": "Input decoupling",
          "pins": ["1", "2"]
        },
        {
          "designator": "D1",
          "type": "diode_schottky",
          "part_number": "B340A",
          "value": null,
          "description": "Catch diode",
          "pins": ["A", "K"]

        }
      ],
      "nets": [
        {
          "name": "VIN",
          "type": "power_input",
          "voltage": "10.8-19.8V",
          "connections": [
            {"component": "U1", "pin": "VIN"},
            {"component": "C1", "pin": "1"}
          ]
        },
        {
          "name": "GND",
          "type": "ground",
          "connections": [
            {"component": "U1", "pin": "GND"},
            {"component": "C1", "pin": "2"},
            {"component": "D1", "pin": "A"},
            {"component": "C3", "pin": "2"},
            {"component": "R2", "pin": "2"}
          ]
        },
        {
          "name": "SW",
          "type": "internal",
          "description": "Switching node",
          "connections": [
            {"component": "U1", "pin": "PH"},
            {"component": "D1", "pin": "K"},
            {"component": "L1", "pin": "1"},
            {"component": "C2", "pin": "2"}
          ]
        }
      ],
      "notes": [
        "Keep SW node area small to minimize EMI",
        "Place C1 close to VIN pin"
      ]
    }
  ]
}
```

---

## CRITICAL RULES

1. **Trace wires, don't guess.** Follow every wire in the schematic from one component pin to another. If you can't see where a wire goes, say so in the notes — don't invent connections.

2. **Junction dots matter.** Two crossing wires are NOT connected unless there is a junction dot (filled circle) at the crossing.

3. **Every pin exactly once.** Each component pin must appear in exactly one net. If you find a pin in zero nets, you missed a connection. If you find it in two nets, you merged nets incorrectly.

4. **Ground is one net.** All ground symbols (the three horizontal lines, or the downward-pointing triangle) connect to the same GND net unless the schematic explicitly labels separate grounds.

5. **Use exact values from the schematic.** Don't substitute, round, or "improve" component values. If the schematic says 3.24kohm, write "3.24kohm", not "3.3kohm".

6. **Include ALL components.** Don't skip "obvious" components like input/output capacitors or the catch diode. Every component drawn in the schematic must appear in your output.

7. **Pin names must be exact.** Use the exact pin names as labeled in the application circuit diagram (not from the pin table elsewhere in the datasheet, which may use different naming).

---

## IF MULTIPLE CIRCUITS EXIST

If the datasheet shows multiple application circuits (e.g., different output voltages, different modes):
- Extract EACH as a separate entry in the `circuits` array
- Use the circuit title/label to name each one
- Share common components if the datasheet shows a base circuit with variations

---

## FINAL CHECKLIST

Before submitting, verify:
- [ ] `part_number` matches document exactly
- [ ] Every component visible in the schematic is listed in `components`
- [ ] Every component pin appears in exactly one net
- [ ] GND net includes all ground-connected pins (IC GND, exposed pad, cap negatives, etc.)
- [ ] VIN and VOUT are separate nets
- [ ] No net has only one connection (unless it's an external port)
- [ ] Component values include units and match the schematic exactly
- [ ] Source page numbers are correct
