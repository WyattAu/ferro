import FileProvider

class FerroFileItem: NSObject, NSFileProviderItem {
    
    let itemIdentifier: NSFileProviderItemIdentifier
    let parentItemIdentifier: NSFileProviderItemIdentifier
    let filename: String
    let contentType: String
    let fileSize: Int64
    let isDirectory: Bool
    
    init(identifier: NSFileProviderItemIdentifier, filename: String, contentType: String, fileSize: Int64, isDirectory: Bool) {
        self.itemIdentifier = identifier
        self.filename = filename
        self.contentType = contentType
        self.fileSize = fileSize
        self.isDirectory = isDirectory
        
        // Derive parent identifier from path
        let pathComponents = identifier.rawValue.split(separator: "/")
        if pathComponents.count > 1 {
            let parentPath = pathComponents.dropLast().joined(separator: "/")
            self.parentItemIdentifier = NSFileProviderItemIdentifier(parentPath)
        } else {
            self.parentItemIdentifier = .rootContainer
        }
        
        super.init()
    }
    
    var itemCapabilities: NSFileProviderItemCapabilities {
        if isDirectory {
            return [.allowsContentEnumerating, .allowsReading]
        } else {
            return [.allowsReading, .allowsWriting, .allowsRenaming, .allowsDeleting]
        }
    }
    
    var typeIdentifier: String {
        return contentType
    }
    
    var fileSystemFlags: NSFileProviderFileSystemFlags {
        return []
    }
    
    var isExcludedFromSync: Bool {
        return false
    }
}
