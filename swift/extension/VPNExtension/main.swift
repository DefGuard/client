import NetworkExtension

autoreleasepool {
    NEProvider.startSystemExtensionMode()
}

// A system extension is a standalone executable: keep the process alive so the registered
// provider can service requests. Without this the process exits immediately after registering.
dispatchMain()
