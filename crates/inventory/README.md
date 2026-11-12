# openmbs-inventory

Manufacturer database, material catalogs, costing rules, and integrity validation for OpenMBS.

## Capabilities
- **Database Catalog**: Web/flange plate dimensions, hot-rolled shapes, cold-formed Z/C purlins, structural bolts, and anchor rods.
- **Supply Scope**: DM81 anchor bolt supply toggles (`ByManufacturer` vs `ByOthers`).
- **Cost Rules (`resolve_cost_rates`)**: DB_USER-style conditional rate overrides for finishes, framing colors, and member weight tiers.
- **DBCHECK (`dbcheck`)**: Automated database auditor detecting out-of-order thicknesses, missing grades, and conflicting costing rules.

