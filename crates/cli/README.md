# openmbs-cli

Command-line interface for OpenMBS batch engineering, CI/CD automation, and reporting.

## CLI Commands
```sh
# Initialize new PEB project file (US Imperial or SI Metric)
openmbs peb-new warehouse.peb.json --us

# Run complete PEB pipeline and export deliverable bundle
openmbs peb-run warehouse.peb.json output/warehouse-bundle

# Generate a turnkey demonstration project and complete bundle
openmbs peb-demo output/demo-peb

# Print plain-text MBS Verify report to stdout
openmbs verify warehouse.peb.json

# Audit manufacturer database integrity with DBCHECK
openmbs dbcheck examples/database.json
```

