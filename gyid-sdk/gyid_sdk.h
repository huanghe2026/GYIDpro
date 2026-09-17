/**
 * GyID SDK - C ABI 头文件
 * 
 * 提供跨语言 FFI 接口，支持:
 * - iOS Swift/Objective-C
 * - Android JNI
 * - 其他 C 兼容语言
 * 
 * 使用方式:
 * 1. 编译 Rust 库为 C 兼容格式
 * 2. 链接 libgyid_sdk.a
 * 3. 包含此头文件
 */

#ifndef GYID_SDK_H
#define GYID_SDK_H

#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ─────────────────────────────────────────────────────────────────────────────
// 版本信息
// ─────────────────────────────────────────────────────────────────────────────

#define GYID_SDK_VERSION_MAJOR 0
#define GYID_SDK_VERSION_MINOR 1
#define GYID_SDK_VERSION_PATCH 0

// ─────────────────────────────────────────────────────────────────────────────
// 数据结构
// ─────────────────────────────────────────────────────────────────────────────

/**
 * GyID 生成结果
 */
typedef struct {
    const char* id;           // GyID 字符串
    const char* hash;         // 原始哈希
    int64_t created_at_ms;   // 创建时间戳（毫秒）
    int32_t linked_devices;  // 关联设备数量
    const char* platform;    // 平台名称
    const char* sdk_version; // SDK 版本
} GyIdResult;

/**
 * 生成选项
 */
typedef struct {
    const char* geo_level;    // "city" | "district" | "exact"
    bool with_geo;            // 是否采集地理位置
    const char* avatar_path;  // 头像路径（可选）
} GenerateOptions;

/**
 * 设备信息
 */
typedef struct {
    const char* linked_gyid;   // 关联的 GyID
    const char* link_id;      // 关联 ID
    int64_t linked_at_ms;     // 关联时间
    bool active;              // 是否激活
} DeviceInfo;

/**
 * 设备列表
 */
typedef struct {
    const char* master_gyid;  // 主 GyID
    DeviceInfo* devices;      // 设备数组
    int32_t device_count;     // 设备数量
} DeviceList;

/**
 * SDK 版本信息
 */
typedef struct {
    const char* version;       // 版本号
    const char* platform;      // 平台
    const char** features;     // 特性列表
    int32_t feature_count;     // 特性数量
} SdkVersion;

/**
 * SDK 错误码
 */
typedef enum {
    GYID_OK = 0,
    GYID_ERROR_INVALID_PARAM = 1,
    GYID_ERROR_GENERATION_FAILED = 2,
    GYID_ERROR_STORAGE_FAILED = 3,
    GYID_ERROR_VERIFICATION_FAILED = 4,
    GYID_ERROR_UNKNOWN = 99
} GyIdErrorCode;

// ─────────────────────────────────────────────────────────────────────────────
// API 函数声明
// ─────────────────────────────────────────────────────────────────────────────

/**
 * 初始化 SDK（可选，某些平台需要）
 * 返回: GYID_OK 或错误码
 */
int gyid_init(void);

/**
 * 使用默认配置生成 GyID
 * 返回: JSON 字符串（调用方需用 gyid_free_string 释放）
 */
const char* gyid_generate_default(void);

/**
 * 使用自定义选项生成 GyID
 * options: JSON 格式的 GenerateOptions
 * 返回: JSON 字符串（调用方需用 gyid_free_string 释放）
 */
const char* gyid_generate(const char* options_json);

/**
 * 验证 GyID 格式
 * gyid: GyID 字符串
 * 返回: true 表示格式正确
 */
bool gyid_verify(const char* gyid);

/**
 * 获取关联设备列表
 * master_gyid: 主 GyID（可选，传 NULL 表示使用本地存储）
 * 返回: JSON 字符串（调用方需用 gyid_free_string 释放）
 */
const char* gyid_get_devices(const char* master_gyid);

/**
 * 导出 GyID 为 JSON
 * 返回: JSON 字符串（调用方需用 gyid_free_string 释放）
 */
const char* gyid_export_json(void);

/**
 * 获取 SDK 版本信息
 * 返回: JSON 字符串（调用方需用 gyid_free_string 释放）
 */
const char* gyid_get_version(void);

/**
 * 释放字符串内存
 * ptr: gyid_* 函数返回的字符串指针
 */
void gyid_free_string(const char* ptr);

/**
 * 获取最后的错误信息
 * 返回: 错误信息字符串（无需释放）
 */
const char* gyid_get_last_error(void);

// ─────────────────────────────────────────────────────────────────────────────
// 同步阻塞 API（Android JNI 风格）
// ─────────────────────────────────────────────────────────────────────────────

/**
 * 同步生成 GyID（阻塞调用）
 * 返回: JSON 字符串
 */
const char* gyid_generate_sync(const char* options_json);

/**
 * 同步获取设备列表
 * 返回: JSON 字符串
 */
const char* gyid_get_devices_sync(const char* master_gyid);

#ifdef __cplusplus
}
#endif

#endif // GYID_SDK_H
