# openmbs-units

Dual-unit engineering conversion and formatting engine for OpenMBS.

## Overview
- Supports both **US Customary (Imperial)** and **SI Metric** systems.
- Utilizes exact NIST conversion constants (e.g. `0.3048 m/ft`, `4.4482216152605 N/lbf`).
- Typed engineering wrappers: `Length`, `Force`, `Pressure`, `Moment`, `Mass`, and `Speed`.
- Architectural fractional string formatting (`format_ft_in`) for feet, inches, and sixteenths.

