import FileProvider

class FileProviderExtension: NSFileProviderExtension {
    
    override func beginObservingDirectory(withIdentifier identifier: NSFileProviderItemIdentifier) {
        // Start watching directory for changes
        NSLog("[Ferro] Begin observing: %@", identifier.rawValue)
    }
    
    override func endObservingDirectory(withIdentifier identifier: NSFileProviderItemIdentifier) {
        // Stop watching directory
        NSLog("[Ferro] End observing: %@", identifier.rawValue)
    }
    
    override func changeHistory(for scope: NSFileProviderChangeScope, since requestIdentifier: NSFileProviderSyncAnchor) async throws -> NSFileProviderChangeSet {
        // Return changes since last sync anchor
        // For now, return empty change set
        return NSFileProviderChangeSet()
    }
    
    override func getSyncAnchor(scope: NSFileProviderSyncAnchor) async throws -> NSFileProviderSyncAnchor {
        // Return current sync anchor
        return NSFileProviderSyncAnchor()
    }
    
    override func item(for identifier: NSFileProviderItemIdentifier) async throws -> NSFileProviderItem {
        // Look up item by identifier
        // Query Ferro server via App Group shared container
        let item = try await FerroFileProviderManager.shared.getItem(identifier: identifier)
        return item
    }
    
    override func enumerateItems(for observer: NSFileProviderEnumerator, startingAt pageToken: NSFileProviderPage) async throws -> NSFileProviderEnumeratorResult {
        // Enumerate items in directory
        let items = try await FerroFileProviderManager.shared.enumerateItems(
            parentIdentifier: observer.itemIdentifier,
            pageToken: pageToken
        )
        return NSFileProviderEnumeratorResult(items: items, nextToken: nil)
    }
}
