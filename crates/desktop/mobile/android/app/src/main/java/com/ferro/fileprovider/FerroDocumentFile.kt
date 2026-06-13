package com.ferro.fileprovider

import android.os.ParcelFileDescriptor
import java.io.File

class FerroDocumentFile(
    val documentId: String,
    val displayName: String,
    val mimeType: String,
    val size: Long,
    val lastModified: Long,
    val isDirectory: Boolean
) {
    fun toUri(): String {
        return "content://${FerroDocumentsProvider.AUTHORITY}/document/$documentId"
    }
}
