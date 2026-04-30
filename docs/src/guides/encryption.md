# Encryption

Ferro supports end-to-end file encryption using the [age](https://age-encryption.org/) format. Files are encrypted with X25519 (key exchange) and ChaCha20-Poly1305 (symmetric encryption), then stored in ASCII-armored format.

## E2E Encryption Overview

```
Original file -> age encrypt (passphrase) -> ASCII armored .age file -> stored on server
Stored .age file -> age decrypt (passphrase) -> original file -> returned to user
```

- Encryption is performed on the server using the `age` crate
- Passphrase-based encryption uses scrypt key derivation
- Encrypted files are identified by the `-----BEGIN AGE ENCRYPTED FILE-----` header
- The server does not store your passphrase

## Encrypting a File

### Via REST API

```bash
curl -X POST http://localhost:8080/api/files/encrypt \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"path": "/documents/secret.txt", "passphrase": "my-secure-password"}'
```

Response:

```json
{
  "path": "/documents/secret.txt",
  "size": 543,
  "encrypted": true
}
```

The original file content is replaced with the age-encrypted version in-place.

### What happens

1. The server reads the file at the given path
2. Encrypts the content using age with the provided passphrase
3. Stores the encrypted content back to the same path
4. The encrypted content starts with `-----BEGIN AGE ENCRYPTED FILE-----`

## Decrypting a File

### Via REST API

```bash
curl -X POST http://localhost:8080/api/files/decrypt \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"path": "/documents/secret.txt", "passphrase": "my-secure-password"}'
```

Response:

```json
{
  "path": "/documents/secret.txt",
  "size": 42,
  "encrypted": false
}
```

### What happens

1. The server reads the file at the given path
2. Checks if the content is age-encrypted (starts with the age header)
3. Decrypts the content using age with the provided passphrase
4. Stores the decrypted content back to the same path

### Wrong passphrase

If the passphrase is incorrect, the API returns a 400 error:

```json
{
  "error": "DECRYPT_FAILED",
  "message": "Decryption failed: wrong passphrase? ..."
}
```

## Key Management

Ferro uses passphrase-based encryption via the age format. There is no server-side key management -- the passphrase is the only secret required to decrypt files.

### Best practices

- Use strong, unique passphrases (20+ characters)
- Do not store passphrases in configuration files
- Consider using a password manager
- Different passphrases for different sensitivity levels
- Test decryption after encryption to verify the passphrase

### Security properties

| Property | Value |
|----------|-------|
| Key exchange | X25519 |
| Symmetric encryption | ChaCha20-Poly1305 |
| Key derivation | scrypt (passphrase-based) |
| Header format | ASCII-armored (`-----BEGIN AGE ENCRYPTED FILE-----`) |

## Using the age CLI Directly

You can also encrypt files before uploading using the `age` CLI tool:

```bash
# Encrypt locally
age -p -o secret.txt.age secret.txt

# Upload encrypted file
curl -X PUT http://localhost:8080/documents/secret.txt.age \
  -H "Authorization: Bearer TOKEN" \
  --data-binary @secret.txt.age

# Download and decrypt
curl http://localhost:8080/documents/secret.txt.age \
  -H "Authorization: Bearer TOKEN" -o secret.txt.age
age -d -o secret.txt secret.txt.age
```

This approach keeps the passphrase entirely on your machine.

## See Also

- [REST API - Encryption endpoints](../api/rest.md#encrypt-a-file)
- [Security](../security.md)
