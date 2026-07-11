# Control plane ownership

**Owner:** Identity and fleet-control maintainers.

This plane owns organization and device enrollment, service identity, tenant placement, capability/policy distribution, and fleet control metadata.

It does not own business records, synchronization payload durability, update artifacts, or unrestricted administration. Changes require tenant-isolation, authorization, audit, availability, and recovery tests.

