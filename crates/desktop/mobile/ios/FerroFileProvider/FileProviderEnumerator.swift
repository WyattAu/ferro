import FileProvider

class FileProviderEnumerator: NSObject, NSFileProviderEnumerator {
    
    let parentIdentifier: NSFileProviderItemIdentifier
    let pageToken: NSFileProviderPage
    
    init(parentIdentifier: NSFileProviderItemIdentifier, pageToken: NSFileProviderPage) {
        self.parentIdentifier = parentIdentifier
        self.pageToken = pageToken
        super.init()
    }
    
    func invalidate() {
        // Cleanup
    }
    
    func enumerateItems(for observer: NSFileProviderEnumerator, startingAt page: NSFileProviderPage) async throws {
        let items = try await FerroFileProviderManager.shared.enumerateItems(
            parentIdentifier: parentIdentifier,
            pageToken: page
        )
        for item in items {
            observer.didEnumerate(items: [item])
        }
        observer.finishEnumerating(upTo: nil)
    }
    
    func enumerateChanges(for observer: NSFileProviderChangeObserver, from syncAnchor: NSFileProviderSyncAnchor) async throws {
        // Report changes since sync anchor
        observer.finishEnumeratingChanges(upTo: syncAnchor, moreComing: false)
    }
    
    func currentSyncAnchor(completionHandler: @escaping (NSFileProviderSyncAnchor?) -> Void) {
        completionHandler(nil)
    }
}
