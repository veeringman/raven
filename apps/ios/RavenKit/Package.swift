// swift-tools-version: 6.2
import PackageDescription

let package = Package(
    name: "RavenKit",
    platforms: [
        .iOS(.v26),
    ],
    products: [
        .library(name: "RavenKit", targets: ["RavenKit"]),
    ],
    targets: [
        .binaryTarget(
            name: "raven_ffiFFI",
            path: "RavenFFI.xcframework"
        ),
        .target(
            name: "RavenKit",
            dependencies: ["raven_ffiFFI"],
            path: "Sources/RavenKit"
        ),
    ]
)
