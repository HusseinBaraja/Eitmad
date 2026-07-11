# End-to-end tests

Own a small set of critical Arabic-first user workflows across shell, engine, storage, audit, sync, and server boundaries. Prefer focused lower-level tests for detailed behavior.

Each critical workflow includes an Arabic-only record, a mixed Arabic/Latin record, keyboard-only operation, a denial or validation failure, and recovery without lost input. Searchable workflows include an original-form match and a broader normalized match. Document-producing workflows render and inspect representative multi-page Arabic output, including font embedding, page breaks, repeated headers, totals, signatures, and mixed-direction identifiers.
