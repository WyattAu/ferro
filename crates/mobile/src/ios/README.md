# iOS Platform Configuration

## Info.plist

The iOS app requires the following Info.plist entries for mobile features:

- **Camera**: `NSCameraUsageDescription` - "Ferro needs camera access to upload photos"
- **Photo Library**: `NSPhotoLibraryUsageDescription` - "Ferro needs photo library access to upload images"
- **Push Notifications**: `UIBackgroundModes` includes `remote-notification`
- **Biometric Auth**: `NSFaceIDUsageDescription` - "Ferro uses Face ID for secure authentication"
