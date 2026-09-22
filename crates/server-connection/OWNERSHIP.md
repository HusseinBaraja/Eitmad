# Direct server connection ownership

This Rust crate owns the desktop engine's authenticated TLS/WebSocket connection to the server, device proof, bounded network I/O, and access/refresh token lifecycle. It implements the transport driver in `eitmad-sync`. The control plane remains the authentication authority and `eitmad-secret-storage` remains the credential persistence authority. Discovery and relay connection drivers are outside this crate's current route.
