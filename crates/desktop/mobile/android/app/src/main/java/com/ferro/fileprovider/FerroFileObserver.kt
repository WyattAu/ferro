package com.ferro.fileprovider

import android.os.FileObserver
import java.io.File

class FerroFileObserver(
    path: String,
    private val listener: OnFileChangeListener?
) : FileObserver(path, ALL_EVENTS) {
    
    interface OnFileChangeListener {
        fun onFileCreated(path: String)
        fun onFileModified(path: String)
        fun onFileDeleted(path: String)
        fun onFileMoved(from: String, to: String)
    }
    
    override fun onEvent(event: Int, path: String?) {
        path ?: return
        
        val fullPath = "$path/$path"
        
        when (event and ALL_EVENTS) {
            CREATE -> listener?.onFileCreated(fullPath)
            MODIFY -> listener?.onFileModified(fullPath)
            DELETE -> listener?.onFileDeleted(fullPath)
            MOVED_FROM -> {
                // Handle move - would need to track the destination
                listener?.onFileDeleted(fullPath)
            }
            MOVED_TO -> {
                listener?.onFileCreated(fullPath)
            }
        }
    }
    
    fun startWatching() {
        startWatching()
    }
    
    fun stopWatching() {
        stopWatching()
    }
}
