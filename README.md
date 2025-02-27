# Solana DEX 智能交易合约

这是一个基于Solana区块链的智能合约项目，实现了与Pump.fun和Raydium去中心化交易所(DEX)的自动交易功能。该合约允许用户在这两个DEX上执行代币的买入和卖出操作，并提供智能路由功能，自动选择价格更优的DEX进行交易。

## 功能特点

- **多DEX支持**：支持在Pump.fun和Raydium上进行交易
- **买入功能**：用户可以使用SOL购买特定代币
- **卖出功能**：用户可以卖出代币获取SOL
- **智能路由**：自动选择价格更优的DEX进行交易
- **滑点控制**：设置滑点参数，确保交易在预期的价格范围内执行

## 技术架构

- **开发框架**：使用Anchor框架进行Solana智能合约开发
- **编程语言**：Rust (合约)，TypeScript (客户端和测试)
- **区块链**：Solana

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
  .buyTokenOnPump(
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
  .buyTokenOnRaydium(
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
    new BN(amountIn), // 输入数量
    new BN(minAmountOut), // 最小输出数量
    true // true表示买入，false表示卖出
  )
  .accounts({
    // 账户参数
  })
  .rpc();
```

## 注意事项

- **安全性**：在使用合约进行交易前，请确保了解相关风险
- **滑点设置**：根据市场波动情况，合理设置滑点参数
- **交易规则**：遵守Pump.fun和Raydium的交易规则

## 贡献指南

1. Fork 项目
2. 创建功能分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'Add some amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 创建Pull Request

## 许可证

MIT

## 联系方式

如有任何问题或建议，请通过以下方式联系我们：

- 项目仓库：[GitHub](https://github.com/yourusername/solana-dex)
- 电子邮件：your.email@example.com 