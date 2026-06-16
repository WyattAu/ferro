package com.wyattau.ferro

import android.content.Context
import java.io.*
import java.net.HttpURLConnection
import java.net.ServerSocket
import java.net.Socket
import java.net.URL

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
            val url = URL("$backendUrl$path")
            val conn = url.openConnection() as HttpURLConnection
            conn.requestMethod = method
            conn.doOutput = body != null
            conn.doInput = true
            conn.connectTimeout = 5000
            conn.readTimeout = 30000

            // Forward relevant headers
            for ((key, value) in headers) {
                if (key !in listOf("host", "connection", "transfer-encoding")) {
                    conn.setRequestProperty(key, value)
                }
            }
            conn.setRequestProperty("Host", "127.0.0.1:8080")
            conn.setRequestProperty("Connection", "close")

            if (body != null) {
                conn.outputStream.write(body)
                conn.outputStream.flush()
            }

            val responseCode = conn.responseCode
            val responseBytes = try {
                conn.inputStream.readBytes()
            } catch (e: Exception) {
                conn.errorStream?.readBytes() ?: byteArrayOf()
            }

            val contentType = conn.contentType ?: "application/octet-stream"

            val header = buildString {
                append("HTTP/1.1 $responseCode OK\r\n")
                append("Content-Type: $contentType\r\n")
                append("Content-Length: ${responseBytes.size}\r\n")
                append("Access-Control-Allow-Origin: *\r\n")
                append("Access-Control-Allow-Methods: GET, POST, PUT, DELETE, PROPFIND, MKCOL, MOVE, COPY, OPTIONS\r\n")
                append("Access-Control-Allow-Headers: *\r\n")
                append("Connection: close\r\n")
                // Forward DAV headers
                for ((key, value) in conn.headerFields) {
                    if (key != null && key.lowercase().startsWith("dav") || key.lowercase().startsWith("allow")) {
                        append("$key: $value\r\n")
                    }
                }
                append("\r\n")
            }

            client.getOutputStream().write(header.toByteArray())
            client.getOutputStream().write(responseBytes)
            client.getOutputStream().flush()
        } catch (e: Exception) {
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
