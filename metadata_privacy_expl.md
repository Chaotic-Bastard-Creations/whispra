Instead of permanent user identifiers like phone numbers and usernames, clients should negotiate short lived rotatin Session IDs
using their cryptographic handhsakes.

The servers view:
When a message hits axum Message::Text loop, the server
only sees this {"session_id": "xyz789", "payload": "aes-gcm-256 encrypted message"}
the server delivers it to whoever owns that temporary mailbox token right now
without ever knowing the real identity of the sendr or the recipient
