---
id: economy/industries
title: Industries & Goods Catalogue
status: draft
owner: horia
depends_on: [economy/production]
research: []
updated: 2026-10-09
---

# Industries & Goods Catalogue

## Purpose
Define every industry in v1, the good or service it produces, who buys it and what it mainly needs. The list must **cover the whole economy**: broad where an industry matters less (e.g. textiles), separate where players will care about it on its own (cars are separate from aircraft; agriculture is split into several branches; electricity generation is split by technology). Each industry produces exactly one good or service, so industry and good share an id, **with one exception: electricity** (see [Electricity](#electricity-exception-to-one-good-per-industry) below).

Target granularity: similar to Power & Revolution's economic sectors, a bit finer for agriculture, energy and transport equipment. *(Compare against P&R 2026's actual sector list in research.)*

## Real-world basis
ISIC Rev.4 / NACE Rev.2 classification (Romania's statistics office INS uses CAEN Rev.2, the national NACE version), grouped. Input coefficients come from input-output tables (Eurostat FIGARO / OECD ICIO for Romania, INS supply-use tables). See the [research backlog](../../01-research/README.md).

Buyer codes: **H** households · **F** firms (intermediate) · **I** investment good · **G** government · **X** exportable · **M** importable.

## Catalogue (v1: 90 industries, 83 goods)

### Agriculture, forestry, fishing
| id | Industry / good | Main inputs | Buyers |
|---|---|---|---|
| `agri_cereals` | Cereals (wheat, maize, barley) | fertilisers, fuel, agri machinery, seeds, water | F (milling, feed, beverages), X, M |
| `agri_oilseeds` | Oilseeds & industrial crops (sunflower, rapeseed, soy, sugar beet) | fertilisers, fuel, machinery | F (food), X, M |
| `agri_fruit_veg` | Fruit, vegetables, potatoes | fertilisers, fuel, water, labour | H, F, X, M |
| `agri_vineyards` | Grapes & vineyards | chemicals, labour | F (beverages), H |
| `livestock_meat` | Cattle, pigs, poultry, sheep | feed (cereals, oilseeds), vet services, electricity | F (meat processing), X, M |
| `livestock_dairy` | Milk & eggs | feed, electricity | F, H |
| `forestry` | Forestry & logging | fuel, machinery | F (wood), X |
| `fishing` | Fishing & aquaculture | fuel, feed | F, H |

Rural households also produce food for their own use ([households](households.md)).

### Mining & extraction
| id | Industry / good | Main inputs | Buyers |
|---|---|---|---|
| `coal_mining` | Coal & lignite | electricity, machinery, explosives (chem) | F (power plants, steel), M |
| `crude_oil` | Crude oil extraction | machinery, electricity | F (refining), X, M |
| `natural_gas` | Natural gas extraction (onshore, offshore Black Sea) | machinery, electricity | F (power, fertilisers, chemicals), H (via distribution), X, M |
| `metal_ores` | Metal ores (iron, copper, gold, other) | electricity, fuel, machinery | F (metals), X, M |
| `uranium_fuel` | Uranium mining & nuclear fuel | electricity, chemicals | F (nuclear plants), M |
| `quarrying` | Stone, sand, gravel, salt, minerals | fuel, machinery | F (building materials, construction, chemicals) |

### Food, beverages, tobacco
| id | Industry / good | Main inputs | Buyers |
|---|---|---|---|
| `food_meat_dairy` | Meat & dairy processing | livestock, electricity, packaging | H, F (restaurants), X, M |
| `food_bakery_milling` | Flour, bread, pasta, bakery | cereals, electricity, gas | H, F |
| `food_other` | Sugar, vegetable oils, preserved fruit & veg, confectionery, other food | oilseeds, fruit & veg, packaging | H, F, X, M |
| `beverages` | Wine, beer, spirits, soft drinks, water | grapes, cereals, glass, packaging | H, F, X, M |
| `tobacco` | Tobacco products | imported leaf, paper | H, X, M |

### Light manufacturing
| id | Industry / good | Main inputs | Buyers |
|---|---|---|---|
| `textiles_apparel` | Textiles & clothing | fibres (imported, agri), chemicals, labour | H, X, M |
| `leather_footwear` | Leather goods & footwear | hides (livestock), chemicals | H, X, M |
| `wood_products` | Sawmills, panels, wood products | forestry, electricity | F (furniture, construction), X |
| `furniture` | Furniture | wood products, textiles, metal products | H, I, X, M |
| `paper_printing` | Paper, packaging, printing | wood, chemicals, electricity | F, H |

### Chemicals, energy products, materials
| id | Industry / good | Main inputs | Buyers |
|---|---|---|---|
| `refined_petroleum` | Petrol, diesel, jet fuel, other refined products | crude oil, electricity | H, F, X, M |
| `fertilisers` | Fertilisers | natural gas (key), chemicals | F (agriculture), X, M |
| `chemicals` | Basic & specialty chemicals, paints, detergents | oil, gas, electricity | F, H, X, M |
| `plastics_rubber` | Plastics & rubber products, tyres | chemicals, electricity | F (car parts, packaging, construction), X, M |
| `pharmaceuticals` | Medicines | chemicals, R&D | H, G (health), X, M |
| `building_materials` | Cement, glass, ceramics, bricks | quarrying, gas, electricity | F (construction) |
| `steel` | Iron & steel | metal ores, coal, electricity, gas | F (metal products, cars, machinery, construction), X, M |
| `nonferrous_metals` | Aluminium, copper and other metals | ores, electricity (very high) | F, X, M |
| `metal_products` | Fabricated metal products (structures, tools, containers) | steel, nonferrous metals | F, I, X |

### Engineering & electronics
| id | Industry / good | Main inputs | Buyers |
|---|---|---|---|
| `electronic_components` | Semiconductors & electronic components | chemicals, metals, electricity | F (electronics, cars), X, M |
| `computers_comms` | Computers, phones, telecom equipment | components | H, I, X, M |
| `consumer_electronics` | TVs, home appliances | components, metal products, plastics | H, X, M |
| `electrical_equipment` | Cables, motors, transformers, batteries | copper, steel, components | F, I (grid, plants), X, M |
| `industrial_machinery` | Industrial machinery & equipment | steel, metal products, electrical equipment | I, X, M |
| `agri_construction_machinery` | Tractors, harvesters, construction machinery | steel, engines, components | I, X, M |
| `medical_devices` | Medical devices & instruments | components, plastics | G, F (health), X, M |

### Transport equipment
| id | Industry / good | Main inputs | Buyers |
|---|---|---|---|
| `car_parts` | Vehicle parts & components | steel, plastics, components, electrical equipment | F (cars, trucks), X, M |
| `cars` | Passenger cars | car parts, steel, components, electricity | H, I (fleets), X, M |
| `trucks_buses` | Trucks, buses, trailers | car parts, steel | I, G, X, M |
| `rail_equipment` | Locomotives, rolling stock | steel, electrical equipment | I, G, X, M |
| `aircraft` | Aircraft, helicopters, aerospace parts | aluminium, components, high-skill labour | I, X, M |
| `shipbuilding` | Ships & boats | steel, machinery | I, X, M |
| `defence_equipment` | Weapons & military equipment | steel, components | G — *inactive until the military layer* |
| `other_manufacturing` | Toys, sports goods, jewellery, other | plastics, metals | H, X, M |

### Utilities
| id | Industry / good | Main inputs | Buyers |
|---|---|---|---|
| `gas_heat_supply` | Gas distribution & district heating | natural gas (domestic and imported) | H, F |
| `water_waste` | Water supply, sewage, waste management | electricity, construction | H, F, G |

### Electricity
Generation industries all produce the good `wholesale_electricity`; buyers are `power_supply` (F), `power_storage` (F) and the interconnectors (X, M). Named firms are listed where obvious (*verify all*); other capacity sits in cohorts.

| id | Industry / good | Main inputs | Buyers | Notes |
|---|---|---|---|---|
| `power_nuclear` | Nuclear generation → `wholesale_electricity` | `uranium_fuel`, industrial machinery, engineering services, high-skill labour | F, X | Named: Nuclearelectrica (Cernavodă). Baseload, ~90% capacity factor, near-zero marginal cost. |
| `power_hydro` | Hydro generation → `wholesale_electricity` | machinery, construction (maintenance), labour | F, X | Named: Hidroelectrica. Output depends on rainfall and drought ([environment](environment.md)). Small hydro in cohorts. |
| `power_coal` | Lignite and hard-coal generation → `wholesale_electricity` | `coal_mining`, machinery, ETS allowances | F, X | Named: Complexul Energetic Oltenia (lignite). Subject to the coal phase-out ([environment](environment.md)). |
| `power_gas` | Gas-fired generation, incl. CHP → `wholesale_electricity` | `natural_gas`, machinery, ETS allowances | F, X | Named: Romgaz (Iernut), OMV Petrom (Brazi). Usually the marginal plant. |
| `power_wind` | Onshore wind → `wholesale_electricity` | electrical equipment, machinery, construction (new capacity) | F, X | Cohorts. Offshore wind (Black Sea) later. |
| `power_solar` | Utility-scale solar → `wholesale_electricity` | electrical equipment (panels, inverters), construction | F, X | Cohorts. Rooftop prosumers are households and firms owning panels, not this industry ([energy](energy.md)). |
| `power_biomass` | Biomass, biogas and waste-to-energy → `wholesale_electricity` | `forestry`, `wood_products`, agricultural residues, `water_waste` | F, X | Cohorts. Biogenic CO₂ outside ETS; local air pollution counts. |
| `power_storage` | Batteries and pumped-hydro storage: buys and resells `wholesale_electricity` | `wholesale_electricity`, electrical equipment (batteries) | F | Cohorts; new capacity mostly by projects. Net consumer (round-trip losses). |
| `power_grid` | Transmission and distribution network services (regulated) → good `power_grid` | electrical equipment, `construction_civil`, `wholesale_electricity` (grid losses), labour | F (`power_supply`) | Named: Transelectrica (transmission); distribution companies (e.g. Electrica's distribution arm, Rețele Electrice, Distribuție Energie Oltenia — *verify*). Tariff set by the regulator. |
| `power_supply` | Retail electricity suppliers → good `electricity` | `wholesale_electricity`, `power_grid` network services, IT, admin support | H, F, G | Named: largest suppliers (*verify list*). Every other industry's electricity input and household energy bills buy this good. |

### Construction
| id | Industry / good | Main inputs | Buyers |
|---|---|---|---|
| `construction_buildings` | Residential & non-residential buildings | building materials, steel, wood, labour | I (housing, firms), G |
| `construction_civil` | Roads, rail, bridges, utility networks, plants | steel, building materials, machinery, fuel | I, G (infrastructure projects) |

### Trade, transport & logistics
| id | Industry / good | Main inputs | Buyers |
|---|---|---|---|
| `wholesale_trade` | Wholesale distribution (margin service) | transport, warehousing, real estate | F |
| `retail_trade` | Retail (margin service) | transport, real estate, electricity | H |
| `vehicle_trade_repair` | Car sales & repair | car parts, labour | H, F |
| `road_freight` | Road freight | fuel, trucks, roads | F, X |
| `rail_transport` | Rail freight & passengers | electricity, fuel, rolling stock, rail network | H, F |
| `air_transport` | Airlines | jet fuel, aircraft, airports | H, F, X |
| `water_transport` | Shipping & ports (incl. Danube, Black Sea) | fuel, ships, ports | F, X |
| `passenger_transport` | Urban & intercity bus, metro, taxis | fuel, electricity, vehicles | H |
| `post_logistics` | Postal, courier, warehousing | transport, real estate | H, F |

### Hospitality & leisure
| id | Industry / good | Main inputs | Buyers |
|---|---|---|---|
| `hotels_tourism` | Hotels, accommodation, tour services | real estate, food, energy | H, X (foreign tourists) |
| `restaurants` | Restaurants, bars, catering | food, beverages, energy | H, X |
| `recreation_culture` | Arts, entertainment, sport, gambling | real estate, media | H, X |

### Information & communication
| id | Industry / good | Main inputs | Buyers |
|---|---|---|---|
| `telecom_services` | Mobile, broadband, fixed telecom | equipment, electricity, broadband network | H, F |
| `it_software` | Software, IT services, outsourcing | computers, high-skill labour | F, H, I, X, M |
| `data_centres` | Data centres & cloud | electricity (very high), computers | F, X, M |
| `media_publishing` | Publishing, broadcasting, film, news | paper, telecom, IT | H, F |

### Finance & real estate
| id | Industry / good | Main inputs | Buyers |
|---|---|---|---|
| `banking_services` | Bank fees & financial services (interest margins are in [money-banking](money-banking.md)) | IT, real estate | H, F |
| `insurance_pensions` | Insurance & pension fund management | IT, real estate | H, F |
| `real_estate` | Rentals & real-estate services | buildings, maintenance | H, F |

### Business services
| id | Industry / good | Main inputs | Buyers |
|---|---|---|---|
| `professional_services` | Legal, accounting, consulting, advertising | IT, real estate | F, G, X |
| `engineering_rnd` | Architecture, engineering, scientific R&D | IT, high-skill labour | F, G, I, X |
| `admin_support` | Security, cleaning, staffing, call centres, travel agencies | labour, vehicles | F, H, X |

### Public & social services
| id | Industry / good | Main inputs | Buyers |
|---|---|---|---|
| `public_administration` | Government administration, tax office, justice, police | IT, real estate, vehicles | G |
| `defence_services` | Armed forces — *inactive until the military layer* | | G |
| `education_services` | Schools & universities (public and private) | real estate, IT, labour | G, H |
| `health_services` | Hospitals, clinics, doctors (public and private) | pharmaceuticals, medical devices, labour | G, H |
| `social_care` | Elderly care, care homes, social work | labour, real estate | G, H |
| `personal_services` | Hairdressers, repairs, laundry, domestic work | labour | H |

## Electricity: exception to "one good per industry"
- The eight generation industries (`power_nuclear`, `power_hydro`, `power_coal`, `power_gas`, `power_wind`, `power_solar`, `power_biomass`, `power_storage`) all sell the same homogeneous good, **`wholesale_electricity`**. It has no markup price: units are dispatched by **merit order** across technologies and the price is `wholesale_electricity_price` ([energy](energy.md)).
- `power_grid` sells network services as its own good `power_grid` at a regulated tariff.
- `power_supply` buys `wholesale_electricity` and network services and sells the retail good **`electricity`** to households, firms and government at `electricity_price`. Input coefficients $a_{electricity,j}$ of all other industries refer to this retail good.
- So industries and goods share ids everywhere except: eight generation industries → one good `wholesale_electricity`, and `power_supply` → good `electricity`. That gives 90 industries and 83 goods.
- Each generation industry has its own firm units, capacity, investment, subsidies, taxes, ownership and levers, so the player can expand solar specifically rather than "energy in general".

## Data per industry and per firm unit (data files, not code)
Each industry is made of **firm units**: 0–5 named firms plus up to three size-class cohort firms ([ADR-0016](../../03-architecture/decisions/0016-firm-representation.md)). Data that describe the technology or the market stay per industry; data that differ between firms are per firm unit, in the `firms` data file.

**Per industry**
- Input coefficients $a_{ij}$ for intermediate inputs (incl. energy).
- Depreciation rate, build time for new capacity.
- Import share of each input (Armington), trade elasticities.
- Starting markup, inventory norm, price stickiness (v1: one market price per industry).
- Household consumption category (for VAT categories and baskets).

**Per firm unit** (summing to the industry totals)
- Kind (named or cohort, with size class), firm count (1 for named firms), real name for named firms.
- Occupation mix per unit of output ([labor-market](labor-market.md)), productivity relative to the industry, capital intensity, export propensity.
- Capacity split across counties and regions as a vector (e.g. Dacia in Argeș, Ford Otosan in Dolj, IT in Bucharest and Cluj — *verify*).
- Ownership vector `ownership[f]` (state, domestic private, foreign) and `company` id for companies active in several industries (ADR-0016 Ownership).
- Generation units: their power plants (technology, capacity, region, age), see [energy](energy.md).
- Opening balance sheet: deposits, loans, capital, inventories, equity.

## Open questions
- [ ] Get P&R 2026's sector list (the owner's reference for granularity) and reconcile.
- [x] Split electricity generation by technology into separate industries (owner decision 2026-10-09): 8 generation industries plus `power_grid` and `power_supply`; see above and [energy](energy.md).
- [ ] Housing: households own dwellings; `real_estate` sells rental services (proposed).
- [ ] Named-firm list and cohort split per industry, from business demography and top lists ([ADR-0016](../../03-architecture/decisions/0016-firm-representation.md) threshold).
- [ ] Offshore wind as its own industry once Black Sea projects are concrete?
- [ ] Should large industrial consumers buy `wholesale_electricity` directly instead of through `power_supply` (v1: all through `power_supply`)?
