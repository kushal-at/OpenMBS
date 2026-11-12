# openmbs-solver

High-performance 2D structural frame analysis engine in pure Rust.

## Capabilities
- **Tapered Built-Up I-Sections**: Numerical integration using 5-point Gauss-Legendre quadrature along linearly varying web depths.
- **End Releases & Hinges**: Clean static condensation of moment releases at frame knees and pinned ridges.
- **Second-Order Geometric Stiffness ($P$-$\Delta$)**: Iterative equilibrium solution accounting for sway amplification and axial-flexural coupling.
- **Station Force Interpolation**: Continuous internal moment, shear, and axial distribution along members.

