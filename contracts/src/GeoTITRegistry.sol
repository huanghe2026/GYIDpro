// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

/// @title GeoTITRegistry — GyID / TRIP 的链上「存在性」登记簿（GYIP-0003 §5.3）
///
/// @notice 设计原则（与 GYIP-0003 §3 的「链上只锚存在性」一致）：
///
/// 1. **不发币、不锚轨迹**：只登记 DID 公钥、epoch 的 Merkle 根、handle 绑定。
///    不写 cell、不写照片、不写任何原始位置。
/// 2. **原始 GPS 永不出端**：链上数据全部是可公开的聚合值（根 + 计数）。
/// 3. **平台代付 gas**：写入函数全部由 `verifier` 地址门控（Verifier / 平台
///    中继），普通用户不需要持有 ETH；`owner` 可轮换该地址。
/// 4. **可升级性最小化**：无代理、无自毁；只有 `owner` / `verifier` 两个
///    可轮换地址。
///
/// 语义要点：
/// - `register` 把「Ed25519 公钥 → DID 后缀」写成**一旦写入不可更改**的绑定
///   （首次登记后重复调用直接 revert），防止身份根被置换；
/// - `anchorEpoch` 要求 epoch 序号**从 0 开始严格连续**，重复提交即 revert
///   （GYIP-0003：「同 epoch 重复提交即告警」）；
/// - `claimHandle` 要求链下已证明 n ≥ 100 且 T ≥ 20（信任分门槛），且 handle
///   **只能声明一次**，避免抢注与反复改名。
///
/// 注意：链上记录只是**索引与存在性证明**，不是真相来源。DID 与公钥的映射、
/// 证书的有效性一律由端上与 RP 依据 `trip-core` 的确定性编码自行验证。
contract GeoTITRegistry {
    /* ------------------------------------------------------------------ */
    /* 类型                                                                */
    /* ------------------------------------------------------------------ */

    struct Identity {
        bool registered;
        uint64 registeredAt;
        /// 已锚定的 epoch 数量，同时也是「下一个待锚定 epoch 序号」。
        uint32 epochCount;
        /// 最近一次锚定的 epoch 序号（`epochCount == 0` 时无意义）。
        uint64 lastEpoch;
        /// 最近一次锚定的 unique cell 数。
        uint32 lastUniqueCells;
        /// handle 声明时间（0 = 未声明）。
        uint64 handleClaimedAt;
        /// geoyuan.com 展示名（空 = 未声明）。
        string handle;
    }

    /* ------------------------------------------------------------------ */
    /* 常量                                                                */
    /* ------------------------------------------------------------------ */

    /// handle 声明的最小面包屑数（GYIP-0003 §10）。
    uint64 public constant HANDLE_MIN_BREADCRUMBS = 100;
    /// handle 声明的最小信任分 ×100（即 T ≥ 20.00）。
    uint64 public constant HANDLE_MIN_TRUST_X100 = 2000;

    /* ------------------------------------------------------------------ */
    /* 存储                                                                */
    /* ------------------------------------------------------------------ */

    address public owner;
    /// 唯一有权写链的地址（Verifier / 平台中继）；可轮换。
    address public verifier;

    mapping(bytes32 pubkey => Identity) private _identities;
    /// keccak256(abi.encode(pubkey, uint256(epochNo))) => Merkle root
    mapping(bytes32 epochKey => bytes32 merkleRoot) public epochRoots;
    /// keccak256(bytes(name)) => pubkey（bytes32(0) = 未占用）
    mapping(bytes32 nameKey => bytes32 pubkey) public handleIndex;

    /* ------------------------------------------------------------------ */
    /* 事件与错误                                                          */
    /* ------------------------------------------------------------------ */

    event OwnershipTransferred(address indexed from, address indexed to);
    event VerifierChanged(address indexed from, address indexed to);
    event DIDRegistered(bytes32 indexed pubkey, string didSuffix, uint64 timestamp);
    event EpochAnchored(
        bytes32 indexed pubkey,
        uint64 indexed epochNo,
        bytes32 merkleRoot,
        uint32 uniqueCells,
        uint64 timestamp
    );
    event HandleClaimed(
        bytes32 indexed pubkey,
        string name,
        uint64 breadcrumbs,
        uint64 trustX100,
        uint64 timestamp
    );

    error NotOwner();
    error NotVerifier();
    error ZeroAddress();
    error ZeroPubkey();
    error EmptyDIDSuffix();
    error AlreadyRegistered(bytes32 pubkey);
    error NotRegistered(bytes32 pubkey);
    error ZeroMerkleRoot();
    error EpochOutOfOrder(uint64 expected, uint64 got);
    error DuplicateEpoch(uint64 epochNo);
    error InsufficientTrajectory(uint64 breadcrumbs, uint64 trustX100);
    error EmptyHandle();
    error HandleAlreadyClaimed(bytes32 pubkey);
    error HandleTaken(bytes32 pubkey);

    /* ------------------------------------------------------------------ */
    /* 构造与治理                                                          */
    /* ------------------------------------------------------------------ */

    /// @param verifier_ 初始 Verifier / 中继地址（不可为零地址）。
    constructor(address verifier_) {
        if (verifier_ == address(0)) revert ZeroAddress();
        owner = msg.sender;
        verifier = verifier_;
        emit OwnershipTransferred(address(0), msg.sender);
        emit VerifierChanged(address(0), verifier_);
    }

    modifier onlyOwner() {
        if (msg.sender != owner) revert NotOwner();
        _;
    }

    modifier onlyVerifier() {
        if (msg.sender != verifier) revert NotVerifier();
        _;
    }

    /// @notice 轮换 owner（最小化治理；无两步确认，便于小团队运维）。
    function transferOwnership(address to) external onlyOwner {
        if (to == address(0)) revert ZeroAddress();
        emit OwnershipTransferred(owner, to);
        owner = to;
    }

    /// @notice 轮换 Verifier / 中继地址（换机器、换运营方时用）。
    function setVerifier(address to) external onlyOwner {
        if (to == address(0)) revert ZeroAddress();
        emit VerifierChanged(verifier, to);
        verifier = to;
    }

    /* ------------------------------------------------------------------ */
    /* 写入（仅 Verifier）                                                 */
    /* ------------------------------------------------------------------ */

    /// @notice 首次登记「Ed25519 公钥 → DID 后缀」绑定；已登记则 revert。
    /// @param pubkey     32 字节 Ed25519 公钥（= TIT / PoH 字段 0）
    /// @param didSuffix  DID 的 multibase 后缀（`z…`），便于浏览器直接拼展示
    function register(bytes32 pubkey, string calldata didSuffix) external onlyVerifier {
        if (pubkey == bytes32(0)) revert ZeroPubkey();
        if (bytes(didSuffix).length == 0) revert EmptyDIDSuffix();

        Identity storage id = _identities[pubkey];
        if (id.registered) revert AlreadyRegistered(pubkey);

        id.registered = true;
        id.registeredAt = uint64(block.timestamp);
        emit DIDRegistered(pubkey, didSuffix, id.registeredAt);
    }

    /// @notice 锚定一个 epoch 的 Merkle 根（序号从 0 起严格连续）。
    /// @param pubkey       身份公钥（须已 `register`）
    /// @param epochNo      epoch 序号，必须等于当前 `epochCount`
    /// @param merkleRoot   该 epoch 面包屑块哈希的 Merkle 根（非零）
    /// @param uniqueCells  该 epoch 内的 unique H3 cell 数
    function anchorEpoch(
        bytes32 pubkey,
        uint64 epochNo,
        bytes32 merkleRoot,
        uint32 uniqueCells
    ) external onlyVerifier {
        Identity storage id = _identities[pubkey];
        if (!id.registered) revert NotRegistered(pubkey);
        if (merkleRoot == bytes32(0)) revert ZeroMerkleRoot();
        if (epochNo != id.epochCount) revert EpochOutOfOrder(id.epochCount, epochNo);

        bytes32 key = epochKey(pubkey, epochNo);
        if (epochRoots[key] != bytes32(0)) revert DuplicateEpoch(epochNo);
        epochRoots[key] = merkleRoot;

        id.epochCount += 1;
        id.lastEpoch = epochNo;
        id.lastUniqueCells = uniqueCells;

        emit EpochAnchored(pubkey, epochNo, merkleRoot, uniqueCells, uint64(block.timestamp));
    }

    /// @notice 声明 handle（geoyuan.com 展示名）；需链下已满足 n≥100 且 T≥20。
    /// @dev 只能声明一次；改成别的名字需要新的身份根（保持「不可撤销」语义）。
    function claimHandle(
        bytes32 pubkey,
        string calldata name,
        uint64 breadcrumbs,
        uint64 trustX100
    ) external onlyVerifier {
        Identity storage id = _identities[pubkey];
        if (!id.registered) revert NotRegistered(pubkey);
        if (bytes(name).length == 0) revert EmptyHandle();
        if (breadcrumbs < HANDLE_MIN_BREADCRUMBS || trustX100 < HANDLE_MIN_TRUST_X100) {
            revert InsufficientTrajectory(breadcrumbs, trustX100);
        }
        if (bytes(id.handle).length != 0) revert HandleAlreadyClaimed(pubkey);

        bytes32 nameKey = keccak256(bytes(name));
        bytes32 holder = handleIndex[nameKey];
        if (holder != bytes32(0) && holder != pubkey) revert HandleTaken(holder);

        handleIndex[nameKey] = pubkey;
        id.handle = name;
        id.handleClaimedAt = uint64(block.timestamp);

        emit HandleClaimed(pubkey, name, breadcrumbs, trustX100, id.handleClaimedAt);
    }

    /* ------------------------------------------------------------------ */
    /* 读取                                                                */
    /* ------------------------------------------------------------------ */

    /// @notice epoch 的存储键：`keccak256(abi.encode(pubkey, uint256(epochNo)))`。
    /// @dev 用 `abi.encode`（而非 `encodePacked`），便于 Rust/TS 侧按标准
    ///      ABI 32 字节字对齐复现，避免打包编码陷阱。
    function epochKey(bytes32 pubkey, uint64 epochNo) public pure returns (bytes32) {
        return keccak256(abi.encode(pubkey, uint256(epochNo)));
    }

    function epochRoot(bytes32 pubkey, uint64 epochNo) external view returns (bytes32) {
        return epochRoots[epochKey(pubkey, epochNo)];
    }

    function isRegistered(bytes32 pubkey) external view returns (bool) {
        return _identities[pubkey].registered;
    }

    /// @notice 标量字段聚合读取（不含动态 `handle`，便于链下低开销解码）。
    function identityOf(bytes32 pubkey)
        external
        view
        returns (
            bool registered,
            uint64 registeredAt,
            uint32 epochCount,
            uint64 lastEpoch,
            uint32 lastUniqueCells,
            uint64 handleClaimedAt
        )
    {
        Identity storage id = _identities[pubkey];
        return (
            id.registered,
            id.registeredAt,
            id.epochCount,
            id.lastEpoch,
            id.lastUniqueCells,
            id.handleClaimedAt
        );
    }

    function handleOf(bytes32 pubkey) external view returns (string memory) {
        return _identities[pubkey].handle;
    }

    function ownerOfHandle(string calldata name) external view returns (bytes32) {
        return handleIndex[keccak256(bytes(name))];
    }
}
