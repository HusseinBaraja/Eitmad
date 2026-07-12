# Shell conformance tests

Own shared behavioral checks proving every shell uses typed contracts, respects Rust authority, renders Arabic-first states, and recovers from engine lifecycle changes.

Every shell must run the applicable shared scenarios for root RTL direction, logical layout, mirroring decisions, focus and keyboard order, mixed-direction isolation, cursor and selection behavior, copy/paste, truncation, tables, Arabic typography, text scaling, accessibility, complete localized messages, fallback failures, and locale-formatted values. Shell tests assert presentation only; they must not duplicate Rust validation, normalization, rounding, authorization, or document policy.

Platform-specific evidence records the native screen reader, input methods, font fallback, high-contrast behavior, print preview, and any known rendering difference. A platform exception requires an owner and must not weaken canonical data or security behavior.

Every desktop shell also proves that intentional stop suppresses restart, unexpected engine death uses bounded recovery, exhaustion requires explicit retry, stale process observations are ignored, and graceful shutdown precedes process-group termination. Platform-adapter tests may supply this evidence before a visual shell exists.
