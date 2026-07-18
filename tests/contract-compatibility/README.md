# Contract compatibility tests

Own version negotiation, generated-binding validation, capability, schema evolution, error stability, and mixed-language serialization coverage.

Arabic-first coverage verifies Unicode preservation, stable localization message identifiers, typed parameters, canonical locale-independent values, explicit currency codes and units, and the absence of presentation-only bidi controls in stored or transported values. It also verifies safe compatibility behavior when a peer does not recognize a message identifier, parameter, locale capability, or document capability.

The canonical Rust fixture is `fixtures/protocol-v1.json` and contains active protocol `1.0` and `1.1` query samples. The C# runner compiles on .NET 8; the Swift runner compiles on macOS CI. Regenerate or check every output through `crates/contracts/codegen`; never edit generated bindings directly.
