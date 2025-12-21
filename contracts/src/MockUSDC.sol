// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";

/// @title MockUSDC
/// @notice 테스트용 USDC 토큰 - 테스트넷에서 자유롭게 민팅 가능
/// @dev 실제 USDC와 동일하게 6 decimals 사용
contract MockUSDC is ERC20, Ownable {
    /// @notice 한 번에 민팅 가능한 최대량 (안전장치)
    uint256 public constant MAX_MINT_AMOUNT = 1_000_000 * 10 ** 6; // 100만 USDC

    /// @notice 사용자별 마지막 민팅 시간 (스팸 방지)
    mapping(address => uint256) public lastMintTime;

    /// @notice 민팅 쿨다운 시간
    uint256 public constant MINT_COOLDOWN = 1 hours;

    /// @notice 민팅 이벤트
    event Minted(address indexed to, uint256 amount);

    constructor() ERC20("Mock USDC", "mUSDC") Ownable(msg.sender) {
        // 배포자에게 초기 물량 지급 (테스트용)
        _mint(msg.sender, 10_000_000 * 10 ** 6); // 1000만 mUSDC
    }

    /// @notice USDC는 6 decimals 사용
    function decimals() public pure override returns (uint8) {
        return 6;
    }

    /// @notice 누구나 테스트 토큰 민팅 가능 (쿨다운 있음)
    /// @param amount 민팅할 양 (6 decimals 기준)
    function mint(uint256 amount) external {
        require(amount > 0, "Amount must be > 0");
        require(amount <= MAX_MINT_AMOUNT, "Exceeds max mint amount");
        require(
            block.timestamp >= lastMintTime[msg.sender] + MINT_COOLDOWN,
            "Mint cooldown not passed"
        );

        lastMintTime[msg.sender] = block.timestamp;
        _mint(msg.sender, amount);

        emit Minted(msg.sender, amount);
    }

    /// @notice 특정 주소에 민팅 (테스트 편의용)
    /// @param to 받을 주소
    /// @param amount 민팅할 양
    function mintTo(address to, uint256 amount) external {
        require(to != address(0), "Cannot mint to zero address");
        require(amount > 0, "Amount must be > 0");
        require(amount <= MAX_MINT_AMOUNT, "Exceeds max mint amount");
        require(
            block.timestamp >= lastMintTime[msg.sender] + MINT_COOLDOWN,
            "Mint cooldown not passed"
        );

        lastMintTime[msg.sender] = block.timestamp;
        _mint(to, amount);

        emit Minted(to, amount);
    }

    /// @notice Owner는 무제한 민팅 가능 (테스트 시나리오용)
    /// @param to 받을 주소
    /// @param amount 민팅할 양
    function ownerMint(address to, uint256 amount) external onlyOwner {
        require(to != address(0), "Cannot mint to zero address");
        _mint(to, amount);
        emit Minted(to, amount);
    }

    /// @notice 테스트용 토큰 소각
    /// @param amount 소각할 양
    function burn(uint256 amount) external {
        _burn(msg.sender, amount);
    }
}
