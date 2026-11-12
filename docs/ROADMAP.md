# Roadmap toward MBS functional coverage

The target is a comprehensive independent Rust metal-building system. Version 0.1 supplies a usable preliminary vertical slice. It does not close the original 170-feature research inventory or the 16 production workflows. `FEATURE_STATUS.csv` is a conservative per-feature ledger; research evidence is separate from implementation status.

1. **Verified domain and analysis**: durable object identities, unequal bays and multiple buildings, code-profile metadata, independent solver benchmarks, tapered members, releases, second-order analysis, load zones and combination separation, asynchronous computation and cancellation.
2. **Approved design scope**: choose the first jurisdiction/edition and manufacturer conventions; implement hot-rolled/welded member stability, interaction, shear and serviceability; validate each governing clause and diagnostic. Add cold-formed systems only with their own validation.
3. **Connections and detailed parts**: reviewed base/haunch/ridge families, bolts/welds/plates, explicit hole geometry, fit checks, bracing load paths, opening frames, inventory constraints and primary/secondary reconciliation.
4. **Fabrication and commercial outputs**: detailed assemblies, panel layouts/trim, complete BOM, packing/shipping, shop/erection drawings, supplier rates, proposal rules, PDF layout, target-CAD DXF round trips, IFC/Tekla adapters and separately scoped CNC.
5. **QA and controlled issuance**: signed review records, reproducible reference corpus, manufacturing acceptance, backups/recovery, project migration, release manifests and supervised pilot projects.
6. **Expanded product families**: mini-storage, combined buildings, lean-tos, canopies, hips, cranes, mezzanines and additional jurisdictions. Each requires independent validation rather than extrapolating the gable prototype.

## Definition of parity

A capability becomes implemented only when its input, processing, output, error handling and documented constraints work. A capability becomes validated only after its acceptance tests and appropriate independent specialist review pass. Menu visibility, research descriptions and passing unrelated tests do not count. Vendor file compatibility is a separate project requiring authorized fixtures and explicit format specifications.

The existing research contains known gaps: inaccessible client manuals/enhancements, incomplete video review, no licensed runtime tests and unknown proprietary engineering details. Those gaps cannot be filled by assuming vendor behavior. Original research documents are preserved as historical research rather than silently rewritten as current product claims.
