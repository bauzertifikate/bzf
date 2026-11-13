# bzf against the EPBD and the iSFP

An informative annex to the spec (CC BY 4.0). It says where each item the EPBD (Directive (EU) 2024/1275) and the
German iSFP ask for lies in a bzf project. Read on 2026-11-06:
- the directive's full text on [EUR-Lex](https://eur-lex.europa.eu/legal-content/EN/TXT/HTML/?uri=OJ:L_202401275);
- dena's [iSFP handbook](https://www.gebaeudeforum.de/fileadmin/gebaeudeforum/Downloads/iSFP-Publikation/iSFP-Handbuch-21-12.pdf) (10/2021, corrected 09/2025) and [short guide](https://www.gebaeudeforum.de/fileadmin/gebaeudeforum/Downloads/iSFP-Publikation/iSFP-Kurzanleitung-21-12.pdf);
- dena's [news sheet](https://www.gebaeudeforum.de/fileadmin/gebaeudeforum/Downloads/iSFP-Publikation/iSFP-Infosheet-Aktuelles.pdf) (as of 16.10.2026);
- BAFA's [Merkblatt iSFP](https://www.bafa.de/SharedDocs/Downloads/DE/Energie/ebw_merkblatt_isfp_2023.pdf?__blob=publicationFile&v=4);
- the [BEG EM Richtlinie of 17.07.2026](https://www.energiewechsel.de/KAENEF/Redaktion/DE/PDF-Anlagen/BEG/beg-richtlinien-juli-2026/20260720-beg-einzelmassnahmen.pdf).

The standalone dataset proposal is [`schema/bzf-epbd/0.1/dataset.json`](schema/bzf-epbd/0.1/dataset.json), with
examples in [`examples/bzf-epbd/`](examples/bzf-epbd/). This annex records the mapping rationale; it does not claim
that BZF is an official EU exchange format or that schema validation proves regulatory compliance.

What "where" means:

| Kind | Meaning |
|---|---|
| **stored** | A field of a bzf document: the fact as it is, entered or received |
| **derived** | Worked out from stored facts by the method that issues the document (DIN V 18599 for the iSFP, DIN/TS 18599 for the GModG certificate, BAFA's rating rules for the iSFP's colours). Never stored in the model. It reaches a reader as figures of an issue |
| **issued** | In the issued document (`Versendet/…`), whose files keep their owner's format (DIBt's, BAFA's), with `versand.json` naming the type, format, registration, inputs and headline figures |
| **outside** | Content of the issued document that is no fact about the building: legal text, advice, contacts |

## The directive asks for results, and for the inputs behind them

The directive names no data format (no XML, JSON, IFC or BIM) and no building identifier. It asks for
"machine-readable" certificates (Art. 19(4)), a passport "in a digital format suitable for printing" (Art. 12(4)), and
databases behind "an appropriate digital interface" (Art. 22(2)). The formats are left to implementing acts on
data-access interoperability (Art. 16(5)) and on the templates for the Building Stock Observatory (Art. 22(6)).

What makes the model matter is **Art. 20(8)**: every certificate is uploaded to the database "including all necessary
data required for the calculation". Together with Annex I (the calculation's inputs) and Annex VI (inputs checked
against product certificates and plans), the inputs travel with the result. In bzf they are the building document
with its systems, kept with the issue by hash (`versand.json` `inputs`).

## EPBD Annex V: the energy performance certificate

| Item (Annex V) | In bzf | Kind |
|---|---|---|
| Energy performance class; primary and final energy per m² and year; on-site renewable share; operational GHG; life-cycle GWP if available | `versand.json` `figures` (`class`, `primaryEnergy`, `finalEnergy`, `renewableShare`, `emissions`, `lifeCycleGwp`) | derived, issued |
| Primary and final consumption in kWh/MWh; renewable production, main carrier and renewable source; energy needs | `figures` (`energyNeed`, `renewableProduction`; the absolute energy is the per-m² figure on `area`); carrier and source from the producing `systems[]` | derived, issued |
| Can react to external signals (yes/no) | `systems[].signals`; as the issuer assessed it, `versand.json` `assessments.reactsToSignals` | stored, issued |
| Heat distribution can run at low temperature (yes/no) | `systems[].flowTemperature` of the distribution and emitters; as assessed, `assessments.lowTemperatureHeating` | stored, then derived; issued |
| One-stop shop contact | `versand.json` `oneStopShop` | issued |
| Optional: energy use, peak load, generator size, carrier and main element type per use (heating, cooling, hot water, ventilation, lighting) | `figures[].uses` (energy need, final and primary energy, peak load per use); `systems[]` (`kind`, `type`, `services`, `carrier`, `output`) | issued, stored |
| Optional: average U-value of opaque and of transparent elements; most common window type | constructions' layers and `openings[].uValue`, `glazing`, `frame` | derived from stored |
| Optional: overheating risk; PM2.5; GHG emission class; carbon removals by temporary storage | `assessments.overheatingRisk`; `figures` (`pm25`, `emissionClass`, `carbonRemovals`) | issued |
| Optional: feasibility of more efficient temperatures for heating, hot water, air-conditioning (Art. 19(8)) | `assessments` (`lowTemperatureHeating`, `lowTemperatureHotWater`, `lowTemperatureCooling`) | issued |
| Optional: the district heating network's local primary energy and emission factors | `systems[].factors` of the `districtHeatingStation` | stored |
| Optional: fixed indoor-quality sensors and controls | `systems[]` of kind `automation`, type `indoorQualitySensors` or `indoorQualityControls` | stored |
| Optional: EV charging points, number and type | `systems[]` of kind `chargingPoint` with `count`, `type`, `output` | stored |
| Optional: energy storage, type and size | `systems[]` of kind `battery` (`energy`) or `thermalStorage` (`volume`) | stored |
| Optional: remaining lifespan of heating and AC | `systems[].year`, `endOfLife` | stored |
| Optional: metered consumption | `consumption/<id>.json` | stored |
| Optional: district heating connection or potential | `systems[]` of kind `districtHeatingStation`; `roadmaps[].heatNetwork`; as a certificate states it, `assessments.heatNetwork` | stored, issued |
| Optional: renovation passport available; SRI assessed and its value; logbook available | an issue of type `renovationRoadmap`; an issue of type `sri` with `figures[].smartReadiness`; the logbook is where the project is kept | issued, outside |
| Not in the template, but in § 85 GEG and DIBt's registration | `address`, `year`, `use`, `category`, `identifiers` on the building; registration number in `versand.json` `registrations` | stored |
| The calculation's outputs (Annex I 2, 3): energy per use; total, non-renewable and renewable primary energy; life-cycle GWP per stage (Annex III) | `figures` (`uses`, `primaryEnergy`, `primaryEnergyTotal`, `primaryEnergyRenewable`, `lifeCycleGwpStages`) | issued |
| The calculation's inputs (Annex I 4, 5; Art. 20(8)): thermal capacity, insulation, thermal bridges, passive solar and shading, air-tightness, systems with their efficiencies | layers with `lambda`, `density`, `heatCapacity`, inhomogeneous layers by their `parts`, air layers with their ventilation (`air`); `thermalBridges`; `openings[].gValue`, `lightTransmittance`, `shading`; `airtightness`; `systems[]` with `efficiency`, `scop`, `seer`, `heatRecovery`, `airflow`, `specificFanPower`, `power`, `insulation`, `standingLoss`, `automationClass`; spaces' `designTemperature` and `internalGains` where they differ from the method's standard conditions | stored |
| Reference values: minimum requirements, MEPS, nearly zero-energy and zero-emission (Art. 19(1)) | `figures[].references`, as they apply to the building | issued |
| The certificate's recommendations (Art. 19(5) to (7)) | a roadmap of scheme `epc-recommendations`, each recommendation a step with its figures | stored, issued |

## EPBD Annex VIII and Art. 12: the renovation passport (in Germany, the iSFP)

| Item (Annex VIII) | In bzf | Kind |
|---|---|---|
| Current energy performance | `figures` with `state: "current"` | derived, issued |
| Graphic roadmap of the staged deep renovation | `roadmaps/<id>.json` `steps` | stored |
| National requirements (minimum requirements, MEPS, fossil phase-out, with dates) | the issued document | outside |
| Why the steps come in this order | `steps[].rationale` | stored |
| Per step: measures with technology and material options | `steps[].measures` with `options`, `changes` (a value only an energy program's own model holds, such as a hydraulic balance, as a change of the op `app`) | stored |
| Per step: primary and final energy savings (kWh and %), GHG reduction, bill savings with the price assumptions, class after the step | `figures` per step (`state`: the step's id); prices in `assumptions` | derived (prices stored), issued |
| District-heating potential | `heatNetwork` | stored |
| Renewable generation and self-consumption after renovation | `figures` with `state: "target"` (`renewableShare`, `selfConsumption`) | derived, issued |
| Circularity and whole-life GHG options; health, comfort, adaptation benefits | `measures[].options`, `product.epd` on materials and systems; benefits in the issued document | stored, outside |
| Funding sources with links | `measures[].funding` (`programme`, `url`) | stored |
| Advisory services, one-stop shops | `versand.json` `oneStopShop`; other advice in the issued document | issued, outside |
| May: timing of steps; time a step takes | `steps[].year`, `until`, `trigger`; `duration` | stored |
| May: costs per step, payback with and without subsidy | `costs/<id>.json` (commercial), each figure with its `source`; payback derived | stored, derived |
| May: technologies with their advantages and disadvantages | `measures[].description`, `options`, `advantages`, `disadvantages` | stored |
| May: lifetime and maintenance costs | `measures[].lifetime`; `costs/<id>.json` `maintenanceYearly` (commercial); `systems[].endOfLife` | stored |
| May: seismic safety, where relevant | the building's `seismic` (`zone`, `note`) | stored |
| May: past major renovations and envelope replacements | steps with `status: "done"`; `year` on every built item | stored |
| May: trades and local contractors | `measures[].trades` | stored |
| Issued after an on-site visit (Art. 12(3)) | `visit` | stored |
| Uploaded to the database and reachable from the logbook (Art. 12(7), (8)) | an issue of type `renovationRoadmap` with `registrations` | issued |

## The iSFP

The iSFP has two documents, "Mein Sanierungsfahrplan" and "Umsetzungshilfe für meine Maßnahmen", printed by BAFA's
Druckapplikation (version 2.7.x from 01.12.2026). The balancing software hands it project and balance data through an
interface BAFA gives only to software makers under a usage agreement. Its format is not published, so an issued iSFP's
data file is named only by `"format": "bafa-isfp"`.

| iSFP | In bzf | Kind |
|---|---|---|
| Address, Energieberater, BAFA-Beraternummer, Vorgangsnummer | building `address`; the Energieberater as a party (`identifiers`); the Vorgangsnummer as a `registrations` entry | stored (party data stays with its organisation) |
| "Ihr Haus heute": building data, past renovations, photos | the building; done steps; received files of `type: "photo"` | stored |
| Colour classes of the whole building (7, on primary energy) and of 8 components: walls, roof or top ceiling, windows and doors, floor (7 each on area-weighted U with BAFA's correction factors), heating and hot water (9, on e_g,p), distribution (7), ventilation (8) | the rated items and their constructions and systems; `measures[].components` groups them | derived (BAFA's rules), issued |
| 2 to 5 packages of up to 5 components, or all at once | `steps` with `measures` | stored |
| Timing: a year, a range, or a trigger | `year`, `until`, `trigger` | stored |
| Target values per measure: U-value, thickness, WLS; efficiency, JAZ (as `scop`), solar fraction, collector area, storage, heat recovery, PV kWp, battery kWh | `measures[].targets`; the new constructions and systems in `changes` | stored |
| Execution variants (up to 3 per component), short description, "Zu beachten" | `options`, `description`, `requirements` | stored |
| Per package: investment, Sowieso-Kosten, BEG funding in € | `costs/<id>.json` per measure (`investment`, `anywayCosts`, `funding`), each with where it comes from (`source`: an offer, the funding guideline); the package's sums derived | stored (commercial) |
| Primary energy, final energy, CO₂ and energy costs after each step, savings in % | `figures` per step | derived, issued |
| Energy costs calibrated to 36 months of consumption | `consumption/<id>.json`: periods with their calorific basis and value, wood type and hot-water share as billed, and by the meters' `services` whether the amounts hold hot water; prices and their source in `assumptions` | stored, then derived |
| Effizienzhaus level reached | `figures[].standard`; `target.standard` | derived, issued |
| Heat networks and municipal heat planning (since 2.6) | `heatNetwork` | stored |
| Energieausweis class and renewable figure (since 2.6) | the certificate's issue `figures` | issued |
| Technical documentation: U-value tables, envelope and systems per state, elevations | constructions, openings, systems; the state after each step from `changes` (after a change of the op `app`, only by the program whose change it is); drawings derived from the geometry | derived from stored |
| User-influence table (temperatures, presence, room use) | the balancing software's own document (reserved app folder) | not in the model |
| Datenblatt zur Qualitätssicherung (heated volume, thermal-bridge allowance, primary energy) | the method; the allowance is the balancing software's setting | derived |
| BEG check that a measure is the iSFP's (Richtlinie 9.2.4), iSFP bonus within 15 years | the issued iSFP with its `time`; `measures[].targets` to compare against | stored, issued |

## What the format leaves to others

- **The methods.** How DIN V 18599 or DIN/TS 18599 balance a building, and BAFA's rating rules and correction
  factors, are not in the format. What a method assumes for an item (a default by kind and year) is never stored in the model; a result record names
  what its method assumed (`result.json` `assumptions`).
- **The issued formats.** DIBt's certificate data (unpublished for 2027), BAFA's iSFP data (not public) and GAEB
  belong to their owners. bzf keeps their files and names them.
- **Advice and law.** National requirements, one-stop-shop contacts, benefit texts and the one-stop shop's own
  record are the issued documents' content.
- **A building identifier.** The directive defines none and Germany has none. `identifiers` holds the cadastre's
  (ALKIS) id as `de:alkis` and leaves room for a national one (`<country>:building-id`).
