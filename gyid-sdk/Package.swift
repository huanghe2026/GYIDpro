// swift-tools-version: 5.9
// GyID SDK - Swift Package Manager 清单
//
// 将 GyIdSDK.xcframework 添加为二进制依赖

import PackageDescription

let package = Package(
    name: "GyIdSDK",
    platforms: [
        .iOS(.v14),
        .macOS(.v11),
    ],
    products: [
        .library(
            name: "GyIdSDK",
            targets: ["GyIdSDKSwift"]
        ),
    ],
    targets: [
        // XCFramework 二进制（由 build-ios.sh 构建）
        .binaryTarget(
            name: "GyIdSDKCore",
            path: "target/ios-xcframework/GyIdSDK.xcframework"
        ),
        // Swift 封装层
        .target(
            name: "GyIdSDKSwift",
            dependencies: ["GyIdSDKCore"],
            path: "ios",
            sources: ["GyIdSDK.swift"]
        ),
    ]
)
