# Contributing to BZF

BZF is maintained by Bauzertifikate and welcomes independent implementations, issues, pull requests and forks.
The JSON Schemas in this repository are authoritative; no private application source is needed to contribute.

For a schema change:

1. Explain the building information or exchange use case and include a fictional example.
2. Identify whether it changes a stable or draft contract. Follow [VERSIONING.md](VERSIONING.md).
3. Specify field meaning and units, preserve unknown fields, and add positive and negative validation fixtures.
4. Update the documentation and changelog, then run `./check.sh`.

Do not include customer data, credentials, proprietary plans or copied standards tables in contributions.
Contributions use the existing licence for the relevant part of the repository, as listed in [LICENSE](LICENSE).
Submit only material you have the right to contribute under those terms.

Maintainers review and merge changes into the official specification. A fork may evolve independently; clearly
identify incompatible changes and avoid presenting a modified fork as an official BZF release.

The tests validate document structure and selected references. They do not certify building calculations,
regulatory compliance or a complete storage implementation.
