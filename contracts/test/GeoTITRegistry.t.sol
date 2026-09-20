// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {GeoTITRegistry} from "../src/GeoTITRegistry.sol";

/// @dev 用最少的中间人换取「另一个 msg.sender」，从而在**不依赖 forge-std**
///      （无 vm.prank）的前提下测试访问控制。
contract Caller {
    function exec(address target, bytes calldata data)
        external
        payable
        returns (bool ok, bytes memory ret)
    {
        (ok, ret) = target.call{value: msg.value}(data);
    }
}

/// @title GeoTITRegistry 行为测试
///
/// 设计为**零外部依赖**（不用 forge-std），因此：
/// - `forge test` 可直接运行（`assert` 失败即测试失败）；
/// - `node tools/test-inproc.mjs` 也能把同一个合约部署到内存 EVM 上，
///   依次调用每个 `test_*` 函数（revert = 失败），做到「一套源码两种跑法」。
///
/// 约定：测试合约自身即链上 `verifier`（构造时传入 `address(this)`）。
contract GeoTITRegistryTest {
    GeoTITRegistry internal reg;
    Caller internal thirdParty;

    bytes32 internal constant PK = bytes32(uint256(0x1111));
    bytes32 internal constant PK2 = bytes32(uint256(0x2222));
    bytes32 internal constant ROOT0 = keccak256("epoch-0-root");
    bytes32 internal constant ROOT1 = keccak256("epoch-1-root");

    /// @dev 每个用例前重置状态（JS runner 也会先调用它）。
    function setUp() public {
        reg = new GeoTITRegistry(address(this));
        thirdParty = new Caller();
    }

    /* ---------------------------- 登记 ---------------------------- */

    function test_register_creates_identity() public {
        assert(!reg.isRegistered(PK));
        reg.register(PK, "zF25s3Dd");

        assert(reg.isRegistered(PK));
        assert(!reg.isRegistered(PK2));
        assert(bytes(reg.handleOf(PK)).length == 0);

        (
            bool registered,
            uint64 registeredAt,
            uint32 epochCount,
            uint64 lastEpoch,
            uint32 lastUniqueCells,
            uint64 handleClaimedAt
        ) = reg.identityOf(PK);
        assert(registered);
        // 精确断言：登记的正是当前区块时间（不依赖环境把 timestamp 设成多少）
        assert(registeredAt == uint64(block.timestamp));
        assert(epochCount == 0);
        assert(lastEpoch == 0);
        assert(lastUniqueCells == 0);
        assert(handleClaimedAt == 0);
    }

    function test_register_rejects_duplicate() public {
        reg.register(PK, "zAAA");
        (bool ok,) = address(reg).call(
            abi.encodeWithSelector(GeoTITRegistry.register.selector, PK, "zBBB")
        );
        assert(!ok); // 身份根一旦写入不可更改
    }

    function test_register_rejects_bad_input() public {
        (bool zeroPubkey,) = address(reg).call(
            abi.encodeWithSelector(GeoTITRegistry.register.selector, bytes32(0), "zAAA")
        );
        assert(!zeroPubkey);

        (bool emptySuffix,) = address(reg).call(
            abi.encodeWithSelector(GeoTITRegistry.register.selector, PK, "")
        );
        assert(!emptySuffix);
    }

    /* ---------------------------- 锚定 ---------------------------- */

    function test_anchor_epoch_sequential() public {
        reg.register(PK, "zAAA");
        reg.anchorEpoch(PK, 0, ROOT0, 37);
        reg.anchorEpoch(PK, 1, ROOT1, 41);

        assert(reg.epochRoot(PK, 0) == ROOT0);
        assert(reg.epochRoot(PK, 1) == ROOT1);
        assert(reg.epochRoot(PK, 2) == bytes32(0));

        (, , uint32 epochCount, uint64 lastEpoch, uint32 lastUniqueCells,) =
            reg.identityOf(PK);
        assert(epochCount == 2);
        assert(lastEpoch == 1);
        assert(lastUniqueCells == 41);
    }

    function test_anchor_epoch_out_of_order_reverts() public {
        reg.register(PK, "zAAA");
        reg.anchorEpoch(PK, 0, ROOT0, 10);

        // 跳过 1 直接锚 2 → 必须失败
        (bool skip,) = address(reg).call(
            abi.encodeWithSelector(
                GeoTITRegistry.anchorEpoch.selector,
                PK,
                uint64(2),
                ROOT1,
                uint32(10)
            )
        );
        assert(!skip);

        // 重复锚 0 → 必须失败
        (bool dup,) = address(reg).call(
            abi.encodeWithSelector(
                GeoTITRegistry.anchorEpoch.selector,
                PK,
                uint64(0),
                ROOT1,
                uint32(10)
            )
        );
        assert(!dup);
    }

    function test_anchor_epoch_requires_registration() public {
        (bool ok,) = address(reg).call(
            abi.encodeWithSelector(
                GeoTITRegistry.anchorEpoch.selector,
                PK,
                uint64(0),
                ROOT0,
                uint32(1)
            )
        );
        assert(!ok);
    }

    function test_anchor_epoch_requires_nonzero_root() public {
        reg.register(PK, "zAAA");
        (bool ok,) = address(reg).call(
            abi.encodeWithSelector(
                GeoTITRegistry.anchorEpoch.selector,
                PK,
                uint64(0),
                bytes32(0),
                uint32(1)
            )
        );
        assert(!ok);
    }

    /* ---------------------------- handle ---------------------------- */

    function test_claim_handle_enforces_threshold() public {
        reg.register(PK, "zAAA");

        // n=99 < 100 → 拒绝
        (bool lowCount,) = address(reg).call(
            abi.encodeWithSelector(
                GeoTITRegistry.claimHandle.selector,
                PK,
                "alice",
                uint64(99),
                uint64(2000)
            )
        );
        assert(!lowCount);

        // T=19.99 < 20.00 → 拒绝
        (bool lowTrust,) = address(reg).call(
            abi.encodeWithSelector(
                GeoTITRegistry.claimHandle.selector,
                PK,
                "alice",
                uint64(100),
                uint64(1999)
            )
        );
        assert(!lowTrust);

        // 恰好达标 → 通过
        reg.claimHandle(PK, "alice", 100, 2000);
        assert(
            keccak256(bytes(reg.handleOf(PK))) == keccak256(bytes("alice"))
        );
        assert(reg.ownerOfHandle("alice") == PK);
        (,,,,, uint64 claimedAt) = reg.identityOf(PK);
        assert(claimedAt == uint64(block.timestamp));
        assert(reg.ownerOfHandle("alice") == PK);
    }

    function test_claim_handle_only_once_and_unique() public {
        reg.register(PK, "zAAA");
        reg.register(PK2, "zBBB");
        reg.claimHandle(PK, "alice", 120, 3000);

        // 同一身份再次声明 → 拒绝
        (bool again,) = address(reg).call(
            abi.encodeWithSelector(
                GeoTITRegistry.claimHandle.selector,
                PK,
                "alice2",
                uint64(120),
                uint64(3000)
            )
        );
        assert(!again);

        // 另一身份抢注同名 → 拒绝
        (bool stolen,) = address(reg).call(
            abi.encodeWithSelector(
                GeoTITRegistry.claimHandle.selector,
                PK2,
                "alice",
                uint64(500),
                uint64(9000)
            )
        );
        assert(!stolen);
    }

    /* ---------------------------- 权限 ---------------------------- */

    function test_only_verifier_can_write() public {
        bytes32 pk = PK;
        bytes memory regData =
            abi.encodeWithSelector(GeoTITRegistry.register.selector, pk, "zAAA");
        (bool okRegister,) = thirdParty.exec(address(reg), regData);
        assert(!okRegister);

        // 先由 verifier 登记，再由第三方尝试锚定/声明
        reg.register(PK, "zAAA");

        (bool okAnchor,) = thirdParty.exec(
            address(reg),
            abi.encodeWithSelector(
                GeoTITRegistry.anchorEpoch.selector,
                pk,
                uint64(0),
                ROOT0,
                uint32(1)
            )
        );
        assert(!okAnchor);

        (bool okClaim,) = thirdParty.exec(
            address(reg),
            abi.encodeWithSelector(
                GeoTITRegistry.claimHandle.selector,
                pk,
                "alice",
                uint64(200),
                uint64(5000)
            )
        );
        assert(!okClaim);
    }

    function test_owner_rotates_verifier() public {
        // 第三方不能改治理参数
        (bool okTransfer,) = thirdParty.exec(
            address(reg),
            abi.encodeWithSelector(GeoTITRegistry.transferOwnership.selector, address(0x1))
        );
        assert(!okTransfer);

        (bool okSet,) = thirdParty.exec(
            address(reg),
            abi.encodeWithSelector(GeoTITRegistry.setVerifier.selector, address(0x2))
        );
        assert(!okSet);

        // owner（本测试合约）可以
        reg.setVerifier(address(0xCAFE));
        assert(reg.verifier() == address(0xCAFE));

        // 轮换后原 verifier 失去写权限
        (bool afterRotate,) = address(reg).call(
            abi.encodeWithSelector(GeoTITRegistry.register.selector, PK, "zAAA")
        );
        assert(!afterRotate);

        // 恢复，供后续断言使用
        reg.setVerifier(address(this));
        reg.register(PK, "zAAA");
        assert(reg.isRegistered(PK));
    }

    function test_constructor_rejects_zero_verifier() public {
        bool deployed;
        try new GeoTITRegistry(address(0)) returns (GeoTITRegistry created) {
            created;
            deployed = true;
        } catch {
            deployed = false;
        }
        assert(!deployed);

        // 合法 Verifier 可正常部署，且 owner = 部署者
        GeoTITRegistry ok = new GeoTITRegistry(address(0xBEEF));
        assert(ok.verifier() == address(0xBEEF));
        assert(ok.owner() == address(this));
    }
}
