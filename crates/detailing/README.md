# openmbs-detailing

Structural steel connections, 3D part occurrence modeling, BOMCHECK, and clash detection.

## Capabilities
- **AISC Design Guide 4**: Extended and flush 4-bolt and 8-bolt moment end-plate splices for frame knees and ridge joints.
- **AISC Design Guide 1**: Column base plate design, bearing checks, and anchor rod patterns with DM81 scope tracking.
- **3D Part Occurrences (`PartOccurrence`)**: Complete 3D part graph with physical bounding boxes, piece marks, assembly marks, hole schedules, and weights.
- **BOMCHECK (`run_bomcheck`)**: Reconciles parts against piece marks, lengths, weights, hole patterns, and erection grid coordinates.
- **3D Clash Detection (`run_clash_detection`)**: Performs spatial interference testing across primary frames, purlins, girts, openings, and bracing.

