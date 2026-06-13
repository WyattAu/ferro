package com.ferro.fileprovider

import android.content.ContentProvider
import android.content.ContentValues
import android.content.UriMatcher
import android.database.Cursor
import android.database.MatrixCursor
import android.os.CancellationSignal
import android.os.ParcelFileDescriptor
import android.provider.DocumentsContract
import android.provider.DocumentsContract.Document
import android.provider.DocumentsContract.Root
import android.webkit.MimeTypeMap
import java.io.File
import java.io.FileInputStream
import java.io.FileOutputStream
import java.net.HttpURLConnection
import java.net.URL
import java.util.Base64

class FerroDocumentsProvider : ContentProvider() {
    
    companion object {
        const val AUTHORITY = "com.ferro.fileprovider.documents"
        const val ROOT_ID = "ferro_root"
        const val PREFS_NAME = "ferro_file_provider"
        const val KEY_SERVER_URL = "server_url"
        const val KEY_AUTH_TOKEN = "auth_token"
    }
    
    private val uriMatcher = UriMatcher(UriMatcher.NO_MATCH).apply {
        addURI(AUTHORITY, "root", 1)
        addURI(AUTHORITY, "root/*", 2)
        addURI(AUTHORITY, "document/*", 3)
        addURI(AUTHORITY, "document/*/*", 4)
    }
    
    override fun onCreate(): Boolean = true
    
    override fun queryRoots(projection: Array<out String>?): Cursor {
        val cursor = MatrixCursor(projection ?: arrayOf(
            Root.COLUMN_ROOT_ID,
            Root.COLUMN_TITLE,
            Root.COLUMN_ICON,
            Root.COLUMN_FLAGS,
            Root.COLUMN_DOCUMENT_ID
        ))
        
        cursor.newRow().apply {
            add(Root.COLUMN_ROOT_ID, ROOT_ID)
            add(Root.COLUMN_TITLE, "Ferro")
            add(Root.COLUMN_ICON, android.R.drawable.ic_menu_upload)
            add(Root.COLUMN_FLAGS, Root.FLAG_SUPPORTS_RECENTS or Root.FLAG_SUPPORTS_SEARCH)
            add(Root.COLUMN_DOCUMENT_ID, ROOT_ID)
        }
        
        return cursor
    }
    
    override fun queryDocument(documentId: String, projection: Array<out String>?): Cursor {
        val cursor = MatrixCursor(projection ?: arrayOf(
            Document.COLUMN_DOCUMENT_ID,
            Document.COLUMN_DISPLAY_NAME,
            Document.COLUMN_MIME_TYPE,
            Document.COLUMN_SIZE,
            Document.COLUMN_LAST_MODIFIED,
            Document.COLUMN_FLAGS
        ))
        
        if (documentId == ROOT_ID) {
            cursor.newRow().apply {
                add(Document.COLUMN_DOCUMENT_ID, ROOT_ID)
                add(Document.COLUMN_DISPLAY_NAME, "Ferro")
                add(Document.COLUMN_MIME_TYPE, DocumentsContract.Document.MIME_TYPE_DIR)
                add(Document.COLUMN_SIZE, 0)
                add(Document.COLUMN_LAST_MODIFIED, System.currentTimeMillis())
                add(Document.COLUMN_FLAGS, 0)
            }
        } else {
            // Query Ferro server for document info
            val conn = getServerConnection() ?: return cursor
            val path = documentIdToPath(documentId)
            
            // Make HEAD request to get document metadata
            val url = URL("${conn.serverUrl}$path")
            val httpConn = url.openConnection() as HttpURLConnection
            httpConn.requestMethod = "HEAD"
            httpConn.setRequestProperty("Authorization", "Bearer ${conn.authToken}")
            
            if (httpConn.responseCode == 200) {
                val fileName = path.substringAfterLast('/')
                val contentType = httpConn.contentType ?: "application/octet-stream"
                val contentLength = httpConn.contentLength.toLong()
                val lastModified = httpConn.getHeaderField("Last-Modified")?.let {
                    java.text.SimpleDateFormat("EEE, dd MMM yyyy HH:mm:ss z", java.util.Locale.US)
                        .parse(it)?.time ?: System.currentTimeMillis()
                } ?: System.currentTimeMillis()
                
                val flags = if (httpConn.getHeaderField("Content-Type") == "httpd/unix-directory") {
                    Document.FLAG_DIR_SUPPORTS_CREATE
                } else {
                    0
                }
                
                cursor.newRow().apply {
                    add(Document.COLUMN_DOCUMENT_ID, documentId)
                    add(Document.COLUMN_DISPLAY_NAME, fileName)
                    add(Document.COLUMN_MIME_TYPE, contentType)
                    add(Document.COLUMN_SIZE, contentLength)
                    add(Document.COLUMN_LAST_MODIFIED, lastModified)
                    add(Document.COLUMN_FLAGS, flags)
                }
            }
        }
        
        return cursor
    }
    
    override fun queryChildDocuments(
        parentDocumentId: String,
        projection: Array<out String>?,
        sortOrder: String?
    ): Cursor {
        val cursor = MatrixCursor(projection ?: arrayOf(
            Document.COLUMN_DOCUMENT_ID,
            Document.COLUMN_DISPLAY_NAME,
            Document.COLUMN_MIME_TYPE,
            Document.COLUMN_SIZE,
            Document.COLUMN_LAST_MODIFIED,
            Document.COLUMN_FLAGS
        ))
        
        val conn = getServerConnection() ?: return cursor
        val path = documentIdToPath(parentDocumentId)
        
        // Make PROPFIND request to list directory
        val url = URL("${conn.serverUrl}$path")
        val httpConn = url.openConnection() as HttpURLConnection
        httpConn.requestMethod = "PROPFIND"
        httpConn.setRequestProperty("Depth", "1")
        httpConn.setRequestProperty("Authorization", "Bearer ${conn.authToken}")
        httpConn.setRequestProperty("Content-Type", "application/xml")
        
        if (httpConn.responseCode == 207) {
            val inputStream = httpConn.inputStream
            val response = inputStream.bufferedReader().readText()
            inputStream.close()
            
            // Parse PROPFIND response and add items
            val items = parsePropfindResponse(response, path)
            for (item in items) {
                cursor.newRow().apply {
                    add(Document.COLUMN_DOCUMENT_ID, item.documentId)
                    add(Document.COLUMN_DISPLAY_NAME, item.displayName)
                    add(Document.COLUMN_MIME_TYPE, item.mimeType)
                    add(Document.COLUMN_SIZE, item.size)
                    add(Document.COLUMN_LAST_MODIFIED, item.lastModified)
                    add(Document.COLUMN_FLAGS, item.flags)
                }
            }
        }
        
        return cursor
    }
    
    override fun openDocument(
        documentId: String,
        mode: String,
        signal: CancellationSignal?
    ): ParcelFileDescriptor {
        val conn = getServerConnection() 
            ?: throw IllegalArgumentException("Not authenticated")
        
        val path = documentIdToPath(documentId)
        val url = URL("${conn.serverUrl}$path")
        
        // Create a pipe for streaming
        val pipe = ParcelFileDescriptor.createPipe()
        val readSide = pipe.readFd
        
        Thread {
            try {
                val httpConn = url.openConnection() as HttpURLConnection
                httpConn.requestMethod = "GET"
                httpConn.setRequestProperty("Authorization", "Bearer ${conn.authToken}")
                
                val inputStream = httpConn.inputStream
                val outputStream = FileOutputStream(ParcelFileDescriptor.FileDescriptor(readSide))
                
                val buffer = ByteArray(8192)
                var bytesRead: Int
                while (inputStream.read(buffer).also { bytesRead = it } != -1) {
                    outputStream.write(buffer, 0, bytesRead)
                }
                
                outputStream.close()
                inputStream.close()
            } catch (e: Exception) {
                e.printStackTrace()
            }
        }.start()
        
        return ParcelFileDescriptor.ParcelFileDescriptor(readSide)
    }
    
    override fun createDocument(
        parentDocumentId: String,
        mimeType: String,
        displayName: String
    ): String {
        val conn = getServerConnection() 
            ?: throw IllegalArgumentException("Not authenticated")
        
        val parentPath = documentIdToPath(parentDocumentId)
        val newPath = "$parentPath/$displayName"
        val url = URL("${conn.serverUrl}$newPath")
        
        if (mimeType == DocumentsContract.Document.MIME_TYPE_DIR) {
            // MKCOL for directory
            val httpConn = url.openConnection() as HttpURLConnection
            httpConn.requestMethod = "MKCOL"
            httpConn.setRequestProperty("Authorization", "Bearer ${conn.authToken}")
            httpConn.responseCode
        } else {
            // PUT for file
            val httpConn = url.openConnection() as HttpURLConnection
            httpConn.requestMethod = "PUT"
            httpConn.setRequestProperty("Authorization", "Bearer ${conn.authToken}")
            httpConn.setRequestProperty("Content-Type", mimeType)
            httpConn.doOutput = true
            httpConn.outputStream.close()
            httpConn.responseCode
        }
        
        return pathToDocumentId(newPath)
    }
    
    override fun deleteDocument(documentId: String): Int {
        val conn = getServerConnection() 
            ?: throw IllegalArgumentException("Not authenticated")
        
        val path = documentIdToPath(documentId)
        val url = URL("${conn.serverUrl}$path")
        
        val httpConn = url.openConnection() as HttpURLConnection
        httpConn.requestMethod = "DELETE"
        httpConn.setRequestProperty("Authorization", "Bearer ${conn.authToken}")
        
        return if (httpConn.responseCode in 200..299) 1 else 0
    }
    
    override fun renameDocument(documentId: String, displayName: String): String {
        val conn = getServerConnection() 
            ?: throw IllegalArgumentException("Not authenticated")
        
        val oldPath = documentIdToPath(documentId)
        val parentPath = oldPath.substringBeforeLast('/')
        val newPath = "$parentPath/$displayName"
        
        val url = URL("${conn.serverUrl}$oldPath")
        val httpConn = url.openConnection() as HttpURLConnection
        httpConn.requestMethod = "MOVE"
        httpConn.setRequestProperty("Authorization", "Bearer ${conn.authToken}")
        httpConn.setRequestProperty("Destination", "${conn.serverUrl}$newPath")
        httpConn.responseCode
        
        return pathToDocumentId(newPath)
    }
    
    override fun copyDocument(sourceDocumentId: String, targetParentDocumentId: String): String {
        val conn = getServerConnection() 
            ?: throw IllegalArgumentException("Not authenticated")
        
        val sourcePath = documentIdToPath(sourceDocumentId)
        val targetParentPath = documentIdToPath(targetParentDocumentId)
        val fileName = sourcePath.substringAfterLast('/')
        val targetPath = "$targetParentPath/$fileName"
        
        val url = URL("${conn.serverUrl}$sourcePath")
        val httpConn = url.openConnection() as HttpURLConnection
        httpConn.requestMethod = "COPY"
        httpConn.setRequestProperty("Authorization", "Bearer ${conn.authToken}")
        httpConn.setRequestProperty("Destination", "${conn.serverUrl}$targetPath")
        httpConn.responseCode
        
        return pathToDocumentId(targetPath)
    }
    
    // Helper methods
    private fun getServerConnection(): ServerConnection? {
        val prefs = context?.getSharedPreferences(PREFS_NAME, 0) ?: return null
        val serverUrl = prefs.getString(KEY_SERVER_URL, null) ?: return null
        val authToken = prefs.getString(KEY_AUTH_TOKEN, null) ?: return null
        return ServerConnection(serverUrl, authToken)
    }
    
    private fun documentIdToPath(documentId: String): String {
        return if (documentId == ROOT_ID) "/" else "/$documentId"
    }
    
    private fun pathToDocumentId(path: String): String {
        return path.removePrefix("/")
    }
    
    private fun parsePropfindResponse(response: String, parentPath: String): List<DocumentItem> {
        // Simple XML parsing for PROPFIND response
        val items = mutableListOf<DocumentItem>()
        // Real implementation would use XmlPullParser
        return items
    }
    
    // Required but not used
    override fun update(documentId: String, values: ContentValues?, selection: String?, selectionArgs: Array<out String>?) = 0
    override fun query(uri: android.net.Uri, projection: Array<out String>?, selection: String?, selectionArgs: Array<out String>?, sortOrder: String?): Cursor? = null
    override fun getType(uri: android.net.Uri): String? = null
    override fun insert(uri: android.net.Uri, values: ContentValues?): android.net.Uri? = null
    override fun delete(uri: android.net.Uri, selection: String?, selectionArgs: Array<out String>?) = 0
}

data class ServerConnection(val serverUrl: String, val authToken: String)
data class DocumentItem(
    val documentId: String,
    val displayName: String,
    val mimeType: String,
    val size: Long,
    val lastModified: Long,
    val flags: Int
)
