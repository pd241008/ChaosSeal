# Citation

## Artifact Version

Artifact release: v3 (multi-tier strengthening generation)

Artifact branch: `feature/docs` (see git log for the exact commit)

## Associated Paper

Desai, P. and Sastry, A. "Dynamic Node Revocation and Chaos-Derived Key
Rotation in LEO Satellite Swarms: A Dual-Layer Cryptographic Protocol."
*IEEE Ad Hoc Networks* (Submitted, 2026).

DOI badge in `README.md` (Zenodo: 10.5281/zenodo.22005854).

## How to Cite This Artifact

If you reference this artifact in your own work, please cite the associated
paper and link to this repository:

```bibtex
@article{desai2026dynamic,
  title={Dynamic Node Revocation and Chaos-Derived Key Rotation in LEO
         Satellite Swarms: A Dual-Layer Cryptographic Protocol},
  author={Desai, Prathmesh and Sastry, Avinash},
  journal={IEEE Ad Hoc Networks (Submitted)},
  year={2026}
}
```

## Component Citation

Individual components may be cited separately:

- **Protocol core (Q32.32 fixed-point, wrapped coupling, Lyapunov/KS, AEAD, BEE)**:
  `core_v2/`
- **LEO network simulator (visibility, Gilbert-Elliott loss, baselines)**:
  `netsim_v2/`
- **Independent verification gates**: `scripts/validate_benettin.py`,
  `scripts/verify_metastability.py`, `scripts/verify_spectrum_convergence.py`
- **Canonical aggregation**: `analysis/v3_analysis.py`
- **Formal security and membership arguments**:
  `docs/end_to_end_security_and_membership.md`

## Versioning Policy

This artifact follows the paper's review cycle. Code and archived results
(`results_v3/`) will not change during the review period; reproduction
commands never overwrite the archive. Post-acceptance updates will be
released as new tags.

## Important Caveat When Citing

The pendulum-derived security numbers (λ₁, KS, epoch bounds) are
**design-stage and verifier-gated** — see the caveat block in `README.md`
and `docs/design_note_metastability.md`. The originally published
"840.6 s epoch bound" was retracted on 2026-09-06 and must not be cited;
see `PROVENANCE.md` for the retraction record.
