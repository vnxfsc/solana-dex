# Solana DEX 智能交易合约

这是一个基于Solana区块链的智能合约项目，实现了与Pump.fun和Raydium去中心化交易所(DEX)的自动交易功能。该合约允许用户在这两个DEX上执行代币的买入和卖出操作，并提供智能路由功能，自动选择价格更优的DEX进行交易。此外，项目还实现了MEV保护功能，有效防止交易被抢先交易攻击。

## 功能特点

- **多DEX支持**：支持在Pump.fun和Raydium上进行交易
- **买入功能**：用户可以使用SOL购买特定代币
- **卖出功能**：用户可以卖出代币获取SOL
- **智能路由**：自动选择价格更优的DEX进行交易
- **滑点控制**：设置滑点参数，确保交易在预期的价格范围内执行
- **MEV保护**：通过承诺-揭示模式防止抢先交易攻击
- **批量交易**：支持批量创建和执行交易承诺，提高交易效率
- **高频交易优化**：针对高频交易场景进行了多项性能优化

## 技术架构

- **开发框架**：使用Anchor框架进行Solana智能合约开发
- **编程语言**：Rust (合约)，TypeScript (客户端和测试)
- **区块链**：Solana
- **交互协议**：通过CPI调用与Raydium和Pump.fun协议交互

## 安装和设置

### 前提条件

- 安装 [Rust](https://www.rust-lang.org/tools/install)
- 安装 [Solana CLI](https://docs.solana.com/cli/install-solana-cli-tools)
- 安装 [Anchor](https://project-serum.github.io/anchor/getting-started/installation.html)
- 安装 [Node.js](https://nodejs.org/) 和 [Yarn](https://yarnpkg.com/)

### 安装依赖

```bash
# 安装Rust依赖
cargo build

# 安装Node.js依赖
yarn install
```

## 构建和测试

### 构建合约

```bash
anchor build
```

### 运行测试

```bash
anchor test
```

## 部署

### 部署到本地网络

```bash
anchor localnet
```

### 部署到开发网络

```bash
anchor deploy --provider.cluster devnet
```

### 部署到主网

```bash
anchor deploy --provider.cluster mainnet
```

## 使用指南

### 初始化DEX账户

```typescript
await program.methods
  .initialize()
  .accounts({
    authority: wallet.publicKey,
    dexAccount: dexAccount,
    systemProgram: SystemProgram.programId,
  })
  .rpc();
```

### 在Pump.fun上购买代币

```typescript
await program.methods
  .pumpBuy(
    new BN(amountIn), // SOL数量
    new BN(minAmountOut) // 最小获得的代币数量
  )
  .accounts({
    // 账户参数
  })
  .rpc();
```

### 在Raydium上购买代币

```typescript
await program.methods
  .raydiumBuy(
    new BN(amountIn), // SOL数量
    new BN(minAmountOut) // 最小获得的代币数量
  )
  .accounts({
    // 账户参数
  })
  .rpc();
```

### 使用智能路由进行交易

```typescript
await program.methods
  .smartTrade(
    tokenMint,
    new BN(amountIn), // 输入数量
    new BN(minAmountOut), // 最小输出数量
    isBuy // true表示买入，false表示卖出
  )
  .accounts({
    // 账户参数
  })
  .rpc();
```

### 使用MEV保护功能

#### 创建交易承诺

```typescript
const nonce = generateRandomNonce();
const commitmentHash = calculateCommitmentHash(
  tokenMint, 
  amountIn, 
  minAmountOut, 
  isBuy, 
  nonce
);

await program.methods
  .createTradeCommitment(
    commitmentHash,
    new BN(slotDelay)
  )
  .accounts({
    // 账户参数
  })
  .rpc();
```

#### 执行交易承诺

```typescript
await program.methods
  .executeCommittedTrade(
    tokenMint,
    new BN(amountIn),
    new BN(minAmountOut),
    isBuy,
    nonce
  )
  .accounts({
    // 账户参数
  })
  .rpc();
```

#### 批量执行交易承诺

```typescript
await program.methods
  .batchExecuteCommitments(
    commitmentIds
  )
  .accounts({
    // 账户参数
  })
  .rpc();
```

## 性能优化

本项目针对高频交易场景进行了多项优化：

- **减少账户加载时间**：优化了池状态加载逻辑
- **批量交易功能**：支持在一个交易中执行多个操作
- **安全算术运算**：使用安全的算术运算防止溢出错误
- **减少存储操作**：最小化链上存储操作，降低Gas成本
- **预计算常量**：避免重复计算，提高执行效率

## 安全增强

- **重入攻击保护**：实现了重入锁机制
- **价格影响检查**：防止市场操纵和滑点攻击
- **MEV保护**：通过承诺-揭示模式防止抢先交易
- **详细错误处理**：提供更具体的错误类型和消息

## 注意事项

- **安全性**：在使用合约进行交易前，请确保了解相关风险
- **滑点设置**：根据市场波动情况，合理设置滑点参数
- **交易规则**：遵守Pump.fun和Raydium的交易规则
- **MEV保护**：使用承诺-揭示模式时，需要等待足够的区块确认

## 贡献指南

1. Fork 项目
2. 创建功能分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'Add some amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 创建Pull Request

