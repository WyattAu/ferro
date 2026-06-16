package com.wyattau.ferro

import android.content.Context
import java.io.*
import java.net.ServerSocket
import java.net.Socket

/**
 * HTTP server that serves static files from Android assets
 * and proxies API requests to the Ferro server via ADB reverse.
 */
class AssetServer(private val context: Context, private val port: Int = 8888) {
    private var serverSocket: ServerSocket? = null
    private var running = false
    private var thread: Thread? = null
    private val backendUrl = "http://127.0.0.1:8080"

    fun start() {
        running = true
        thread = Thread {
            try {
                serverSocket = ServerSocket(port)
                while (running) {
                    val client = serverSocket?.accept() ?: break
                    Thread { handleRequest(client) }.start()
                }
            } catch (e: Exception) {
                if (running) e.printStackTrace()
            }
        }.apply { isDaemon = true; start() }
    }

    fun stop() {
        running = false
        serverSocket?.close()
    }

    private fun handleRequest(client: Socket) {
        try {
            val inputStream = client.getInputStream()
            val reader = BufferedReader(InputStreamReader(inputStream))
            val requestLine = reader.readLine() ?: return
            val parts = requestLine.split(" ")
            if (parts.size < 2) return

            val method = parts[0]
            val rawPath = parts[1]

            // Read headers
            val headers = mutableMapOf<String, String>()
            while (true) {
                val line = reader.readLine() ?: break
                if (line.isEmpty()) break
                val colon = line.indexOf(':')
                if (colon > 0) {
                    headers[line.substring(0, colon).trim().lowercase()] = line.substring(colon + 1).trim()
                }
            }

            // Read body if present
            val contentLength = headers["content-length"]?.toIntOrNull() ?: 0
            val body = if (contentLength > 0) {
                val bodyBytes = ByteArray(contentLength)
                var read = 0
                while (read < contentLength) {
                    val n = inputStream.read(bodyBytes, read, contentLength - read)
                    if (n < 0) break
                    read += n
                }
                bodyBytes
            } else null

            // API requests (WebDAV, REST) -> proxy to Ferro server
            val isApiRequest = method in listOf("PROPFIND", "PROPPATCH", "MKCOL", "PUT", "DELETE", "MOVE", "COPY", "OPTIONS")
                || rawPath.startsWith("/api/")
                || rawPath.startsWith("/dav/")
                || rawPath.startsWith("/healthz")

            if (isApiRequest) {
                proxyRequest(client, method, rawPath, headers, body)
            } else {
                // Static file from assets
                serveStaticFile(client, rawPath)
            }
        } catch (e: Exception) {
            e.printStackTrace()
        } finally {
            client.close()
        }
    }

    private fun proxyRequest(client: Socket, method: String, path: String, headers: Map<String, String>, body: ByteArray?) {
        try {
            // Use raw socket connection (HttpURLConnection doesn't work with ADB reverse)
            val backend = Socket("127.0.0.1", 8080)
            backend.soTimeout = 30000

            // Build raw HTTP request
            val request = StringBuilder()
            request.append("$method $path HTTP/1.1\r\n")
            request.append("Host: 127.0.0.1:8080\r\n")
            request.append("Connection: close\r\n")

            for ((key, value) in headers) {
                if (key !in listOf("host", "connection", "transfer-encoding")) {
                    request.append("$key: $value\r\n")
                }
            }

            if (body != null) {
                request.append("Content-Length: ${body.size}\r\n")
            }
            request.append("\r\n")

            val output = backend.getOutputStream()
            output.write(request.toString().toByteArray())
            if (body != null) {
                output.write(body)
            }
            output.flush()

            // Read response
            val inputStream = backend.getInputStream()
            val responseBytes = inputStream.readBytes()
            backend.close()

            // Parse status line from response
            val responseStr = String(responseBytes)
            val firstLineEnd = responseStr.indexOf("\r\n")
            val statusLine = if (firstLineEnd > 0) responseStr.substring(0, firstLineEnd) else "HTTP/1.1 502 Bad Gateway"
            val statusCode = statusLine.split(" ").getOrNull(1)?.toIntOrNull() ?: 502

            // Find body after headers
            val headerEnd = "\r\n\r\n".toByteArray()
            var bodyStart = -1
            for (i in 0..responseBytes.size - headerEnd.size) {
                if (responseBytes[i] == headerEnd[0] && responseBytes[i+1] == headerEnd[1] && responseBytes[i+2] == headerEnd[2] && responseBytes[i+3] == headerEnd[3]) {
                    bodyStart = i + 4
                    break
                }
            }
            val bodyBytes = if (bodyStart > 0) responseBytes.copyOfRange(bodyStart, responseBytes.size) else byteArrayOf()

            // Extract Content-Type from response headers
            var contentType = "application/octet-stream"
            val headerSection = responseStr.substring(0, firstLineEnd + 2)
            for (line in headerSection.split("\r\n")) {
                if (line.lowercase().startsWith("content-type:")) {
                    contentType = line.substringAfter(":").trim()
                    break
                }
            }

            // Send response to client
            val responseHeader = buildString {
                append("HTTP/1.1 $statusCode OK\r\n")
                append("Content-Type: $contentType\r\n")
                append("Content-Length: ${bodyBytes.size}\r\n")
                append("Access-Control-Allow-Origin: *\r\n")
                append("Access-Control-Allow-Methods: GET, POST, PUT, DELETE, PROPFIND, MKCOL, MOVE, COPY, OPTIONS\r\n")
                append("Access-Control-Allow-Headers: *\r\n")
                append("Connection: close\r\n")
                append("\r\n")
            }

            client.getOutputStream().write(responseHeader.toByteArray())
            client.getOutputStream().write(bodyBytes)
            client.getOutputStream().flush()
        } catch (e: Exception) {
            e.printStackTrace()
            sendResponse(client, 502, "text/plain", "Proxy Error: ${e.message}")
        }
    }

    private fun serveStaticFile(client: Socket, rawPath: String) {
        var path = rawPath.trimStart('/')
        if (path.isEmpty() || path == "/") path = "index.html"
        if (path.startsWith("ui/")) path = path.removePrefix("ui/")

        if (path.contains("..")) {
            sendResponse(client, 403, "text/plain", "Forbidden")
            return
        }

        val mimeType = getMimeType(path)
        val assetPath = "www/$path"

        try {
            val inputStream = context.assets.open(assetPath)
            val bytes = inputStream.readBytes()
            inputStream.close()
            sendResponse(client, 200, mimeType, null, bytes)
        } catch (e: FileNotFoundException) {
            sendResponse(client, 404, "text/plain", "Not Found: $path")
        }
    }

    private fun sendResponse(client: Socket, code: Int, contentType: String, body: String?, bytes: ByteArray? = null) {
        val statusText = when (code) { 200 -> "OK"; 403 -> "Forbidden"; 404 -> "Not Found"; 502 -> "Bad Gateway"; else -> "Error" }
        val responseBytes = bytes ?: body?.toByteArray() ?: byteArrayOf()

        val header = buildString {
            append("HTTP/1.1 $code $statusText\r\n")
            append("Content-Type: $contentType\r\n")
            append("Content-Length: ${responseBytes.size}\r\n")
            append("Access-Control-Allow-Origin: *\r\n")
            append("Access-Control-Allow-Methods: GET, POST, PUT, DELETE, PROPFIND, MKCOL, MOVE, COPY, OPTIONS\r\n")
            append("Access-Control-Allow-Headers: *\r\n")
            append("Cache-Control: no-cache\r\n")
            append("\r\n")
        }

        val output = client.getOutputStream()
        output.write(header.toByteArray())
        output.write(responseBytes)
        output.flush()
    }

    private fun getMimeType(path: String): String = when {
        path.endsWith(".html") -> "text/html"
        path.endsWith(".js") -> "application/javascript"
        path.endsWith(".wasm") -> "application/wasm"
        path.endsWith(".css") -> "text/css"
        path.endsWith(".json") -> "application/json"
        path.endsWith(".png") -> "image/png"
        path.endsWith(".jpg") -> "image/jpeg"
        path.endsWith(".svg") -> "image/svg+xml"
        path.endsWith(".woff2") -> "font/woff2"
        else -> "application/octet-stream"
    }
}
