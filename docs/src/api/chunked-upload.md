# Chunked Upload API

Ferro supports resumable chunked uploads for large files. Files are split into chunks, uploaded individually, and then reassembled on the server.

## Flow Overview

```
1. Init   -> POST /api/upload/init       -> returns upload_id, chunk_size
2. Chunk  -> PUT  /api/upload/:id/:index  -> upload each chunk (0, 1, 2, ...)
3. Done   -> POST /api/upload/:id/complete -> reassemble and store the file
```

## Init

Start a new chunked upload session:

```bash
curl -X POST http://localhost:8080/api/upload/init \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "path": "/videos/large-file.mp4",
    "total_size": 157286400,
    "chunk_size": 5242880
  }'
```

Response:

```json
{
  "upload_id": "ul_a1b2c3d4e5f6",
  "chunk_size": 5242880
}
```

### Request Fields

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `path` | `string` | Yes | -- | Target file path |
| `total_size` | `integer` | No | -- | Total file size in bytes (enables validation) |
| `chunk_size` | `integer` | No | `5242880` (5 MB) | Chunk size in bytes |

## Upload Chunks

Upload each chunk by its zero-based index:

```bash
# Chunk 0 (bytes 0 - 5242879)
dd if=large-file.mp4 bs=1M count=5 | \
  curl -X PUT http://localhost:8080/api/upload/ul_a1b2c3d4e5f6/0 \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/octet-stream" \
  --data-binary @-

# Chunk 1 (bytes 5242880 - 10485759)
dd if=large-file.mp4 bs=1M skip=5 count=5 | \
  curl -X PUT http://localhost:8080/api/upload/ul_a1b2c3d4e5f6/1 \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/octet-stream" \
  --data-binary @-

# Chunk 2 (remaining bytes)
dd if=large-file.mp4 bs=1M skip=10 | \
  curl -X PUT http://localhost:8080/api/upload/ul_a1b2c3d4e5f6/2 \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/octet-stream" \
  --data-binary @-
```

Responses:

| Status | Meaning |
|--------|---------|
| `200 OK` | Chunk accepted |
| `404 Not Found` | Invalid upload ID |
| `413 Payload Too Large` | Chunk exceeds configured chunk size |

## Complete Upload

After all chunks are uploaded, finalize the upload:

```bash
curl -X POST http://localhost:8080/api/upload/ul_a1b2c3d4e5f6/complete \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"path": "/videos/large-file.mp4"}'
```

Responses:

| Status | Meaning |
|--------|---------|
| `201 Created` | File assembled and stored |
| `400 Bad Request` | Missing chunks (gap in sequence) |
| `404 Not Found` | Invalid upload ID |
| `500 Internal Server Error` | Storage backend error |

The `path` in the complete request is optional -- if omitted, the path from the init request is used.

## Cancel Upload

Abort an upload session and free its memory:

```bash
curl -X DELETE http://localhost:8080/api/upload/ul_a1b2c3d4e5f6 \
  -H "Authorization: Bearer TOKEN"
```

## List Active Uploads

View all in-progress uploads:

```bash
curl http://localhost:8080/api/uploads \
  -H "Authorization: Bearer TOKEN"
```

```json
[
  {
    "upload_id": "ul_a1b2c3d4e5f6",
    "path": "/videos/large-file.mp4",
    "chunk_size": 5242880,
    "received": 2,
    "total_chunks": 30,
    "elapsed_secs": 15
  }
]
```

## Chunk Size

The default chunk size is 5 MB (5,242,880 bytes). You can customize it in the init request.

When `total_size` is provided, the server calculates the expected number of chunks:

```
total_chunks = ceil(total_size / chunk_size)
```

| Total Size | Chunk Size | Chunks |
|------------|------------|--------|
| 1 KB | 5 MB | 1 |
| 10 MB | 5 MB | 2 |
| 15 MB | 5 MB | 3 |
| 1 GB | 5 MB | 205 |

## Notes

- Upload state is held in memory. Server restarts will lose in-progress uploads.
- Chunks can be uploaded in any order (out-of-order).
- There is no timeout for in-progress uploads.
- The chunk data is held in memory until the upload is completed or cancelled.
