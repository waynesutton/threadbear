`swift-note.hex` is a synthetic note sealed by Swift's CryptoKit
`AES.GCM.seal` with the Swift app's `NoteCrypto` wire format: byte 1,
then `SealedBox.combined` (nonce, JSON ciphertext, authentication tag).
Its 32-byte AES key is 32 repetitions of byte 7. It contains no real note.
The core test opens it to guard compatibility with existing Swift notes.
