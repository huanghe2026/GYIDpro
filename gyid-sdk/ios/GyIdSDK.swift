//
//  GyIdSDK.swift
//  GyID SDK - iOS Swift 封装
//
//  通过 C ABI 调用 Rust 原生库，提供类型安全的 Swift API。
//
//  使用方法:
//  ```swift
//  // 同步方式
//  let result = try GyIdSDK.shared.generateDefault()
//  print("GyID: \(result.id)")
//
//  // 异步方式
//  Task {
//      let result = try await GyIdSDK.shared.generate()
//      print("GyID: \(result.id)")
//  }
//  ```
//

import Foundation

/// GyID SDK 主接口
@MainActor
public final class GyIdSDK: Sendable {
    public static let shared = GyIdSDK()

    private init() {
        // 加载 Rust 动态库（通过 C ABI）
    }

    // ─────────────────────────────────────────────────────────────────────────
    // MARK: - 数据模型
    // ─────────────────────────────────────────────────────────────────────────

    /// GyID 生成结果
    public struct GyIdResult: Codable, Sendable {
        public let id: String
        public let hash: String
        public let createdAtMs: Int64
        public let linkedDevices: Int32
        public let platform: String
        public let sdkVersion: String

        enum CodingKeys: String, CodingKey {
            case id
            case hash
            case createdAtMs = "created_at_ms"
            case linkedDevices = "linked_devices"
            case platform
            case sdkVersion = "sdk_version"
        }
    }

    /// 生成选项
    public struct GenerateOptions: Codable, Sendable {
        public var geoLevel: String
        public var withGeo: Bool
        public var avatarPath: String?

        public init(
            geoLevel: String = "city",
            withGeo: Bool = true,
            avatarPath: String? = nil
        ) {
            self.geoLevel = geoLevel
            self.withGeo = withGeo
            self.avatarPath = avatarPath
        }

        enum CodingKeys: String, CodingKey {
            case geoLevel = "geo_level"
            case withGeo = "with_geo"
            case avatarPath = "avatar_path"
        }
    }

    /// 设备信息
    public struct DeviceInfo: Codable, Sendable {
        public let linkedGyid: String
        public let linkId: String
        public let linkedAtMs: Int64
        public let active: Bool

        enum CodingKeys: String, CodingKey {
            case linkedGyid = "linked_gyid"
            case linkId = "link_id"
            case linkedAtMs = "linked_at_ms"
            case active
        }
    }

    /// 设备列表
    public struct DeviceList: Codable, Sendable {
        public let masterGyid: String
        public let devices: [DeviceInfo]

        enum CodingKeys: String, CodingKey {
            case masterGyid = "master_gyid"
            case devices
        }
    }

    /// SDK 版本信息
    public struct SdkVersion: Codable, Sendable {
        public let version: String
        public let platform: String
        public let features: [String]
    }

    /// SDK 错误
    public enum GyIdError: Error, LocalizedError {
        case invalidFormat(String)
        case generationFailed(String)
        case exportFailed(String)
        case unknown(String)

        public var errorDescription: String? {
            switch self {
            case .invalidFormat(let msg): return "GyID 格式无效: \(msg)"
            case .generationFailed(let msg): return "生成失败: \(msg)"
            case .exportFailed(let msg): return "导出失败: \(msg)"
            case .unknown(let msg): return "未知错误: \(msg)"
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // MARK: - C ABI 绑定
    // ─────────────────────────────────────────────────────────────────────────

    // 注意: 这些函数需要通过 C ABI 暴露 Rust 函数
    // 在实际使用前需要:
    // 1. 使用 UniFFI 或手动创建 C 头文件
    // 2. 编译 Rust 库为 iOS 可用的静态库/框架

    @_silgen_name("gyid_generate_default")
    private static func _generateDefault() -> UnsafePointer<CChar>?

    @_silgen_name("gyid_generate")
    private static func _generate(_ options: UnsafePointer<CChar>?) -> UnsafePointer<CChar>?

    @_silgen_name("gyid_verify")
    private static func _verify(_ gyid: UnsafePointer<CChar>?) -> Bool

    @_silgen_name("gyid_get_devices")
    private static func _getDevices(_ masterGyid: UnsafePointer<CChar>?) -> UnsafePointer<CChar>?

    @_silgen_name("gyid_export_json")
    private static func _exportJson() -> UnsafePointer<CChar>?

    @_silgen_name("gyid_get_version")
    private static func _getVersion() -> UnsafePointer<CChar>?

    @_silgen_name("gyid_free_string")
    private static func _freeString(_ ptr: UnsafePointer<CChar>?)

    // ─────────────────────────────────────────────────────────────────────────
    // MARK: - 公开 API
    // ─────────────────────────────────────────────────────────────────────────

    /// 使用默认配置生成 GyID
    public func generateDefault() throws -> GyIdResult {
        guard let ptr = Self._generateDefault() else {
            throw GyIdError.generationFailed("Unknown error")
        }
        defer { Self._freeString(ptr) }

        let json = String(cString: ptr)
        return try decodeResult(json)
    }

    /// 使用自定义选项生成 GyID
    public func generate(options: GenerateOptions = GenerateOptions()) async throws -> GyIdResult {
        return try await withCheckedThrowingContinuation { continuation in
            Task.detached {
                do {
                    let encoder = JSONEncoder()
                    let data = try encoder.encode(options)
                    let jsonStr = String(data: data, encoding: .utf8) ?? "{}"

                    guard let ptr = Self._generate(jsonStr) else {
                        continuation.resume(throwing: GyIdError.generationFailed("Unknown error"))
                        return
                    }
                    defer { Self._freeString(ptr) }

                    let result = String(cString: ptr)
                    let gyidResult = try self.decodeResult(result)
                    continuation.resume(returning: gyidResult)
                } catch let error as GyIdError {
                    continuation.resume(throwing: error)
                } catch {
                    continuation.resume(throwing: GyIdError.unknown(error.localizedDescription))
                }
            }
        }
    }

    /// 验证 GyID 格式
    public func verify(_ gyid: String) -> Bool {
        return Self._verify(gyid)
    }

    /// 获取关联设备列表
    public func getDevices(masterGyid: String? = nil) throws -> DeviceList {
        guard let ptr = Self._getDevices(masterGyid) else {
            throw GyIdError.unknown("Failed to get devices")
        }
        defer { Self._freeString(ptr) }

        let json = String(cString: ptr)
        return try decodeDeviceList(json)
    }

    /// 导出 GyID 为 JSON
    public func exportJson() throws -> String {
        guard let ptr = Self._exportJson() else {
            throw GyIdError.exportFailed("Unknown error")
        }
        defer { Self._freeString(ptr) }

        return String(cString: ptr)
    }

    /// 获取 SDK 版本信息
    public func version() throws -> SdkVersion {
        guard let ptr = Self._getVersion() else {
            throw GyIdError.unknown("Failed to get version")
        }
        defer { Self._freeString(ptr) }

        let json = String(cString: ptr)
        return try decodeVersion(json)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // MARK: - 私有方法
    // ─────────────────────────────────────────────────────────────────────────

    private func decodeResult(_ json: String) throws -> GyIdResult {
        // 检查错误
        if let errorData = json.data(using: .utf8),
           let jsonObj = try? JSONSerialization.jsonObject(with: errorData) as? [String: Any],
           let error = jsonObj["error"] as? String {
            throw GyIdError.generationFailed(error)
        }

        let decoder = JSONDecoder()
        guard let data = json.data(using: .utf8) else {
            throw GyIdError.unknown("Invalid JSON encoding")
        }
        return try decoder.decode(GyIdResult.self, from: data)
    }

    private func decodeDeviceList(_ json: String) throws -> DeviceList {
        if let errorData = json.data(using: .utf8),
           let jsonObj = try? JSONSerialization.jsonObject(with: errorData) as? [String: Any],
           let error = jsonObj["error"] as? String {
            throw GyIdError.unknown(error)
        }

        let decoder = JSONDecoder()
        guard let data = json.data(using: .utf8) else {
            throw GyIdError.unknown("Invalid JSON encoding")
        }
        return try decoder.decode(DeviceList.self, from: data)
    }

    private func decodeVersion(_ json: String) throws -> SdkVersion {
        let decoder = JSONDecoder()
        guard let data = json.data(using: .utf8) else {
            throw GyIdError.unknown("Invalid JSON encoding")
        }
        return try decoder.decode(SdkVersion.self, from: data)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// MARK: - 便利扩展
// ─────────────────────────────────────────────────────────────────────────────

public extension GyIdSDK.GyIdResult {
    /// 验证此 GyID 是否有效
    func isValid() -> Bool {
        return GyIdSDK.shared.verify(id)
    }

    /// 转换为字典
    func toDictionary() -> [String: Any] {
        return [
            "id": id,
            "hash": hash,
            "createdAtMs": createdAtMs,
            "linkedDevices": linkedDevices,
            "platform": platform,
            "sdkVersion": sdkVersion
        ]
    }

    /// 导出为 JSON 字符串
    func toJsonString() throws -> String {
        let encoder = JSONEncoder()
        encoder.outputFormatting = .prettyPrinted
        let data = try encoder.encode(self)
        return String(data: data, encoding: .utf8) ?? ""
    }
}
