import * as anchor from '@coral-xyz/anchor';
import { PublicKey, SystemProgram, LAMPORTS_PER_SOL } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '@solana/spl-token';

async function main() {
  console.log("开始交易示例...");

  // 设置连接和钱包
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  // 加载程序
  // 实际使用时应该使用正确的程序ID
  const programId = new PublicKey("FZ6RHhMSv5xoE8GjK5KJi2i7Gue1DW3APGe4an4CJjte");
  
  try {
    // 直接使用IDL对象，避免fetchIdl调用
    const program = new anchor.Program({} as any, programId, provider);

    console.log("钱包地址:", provider.wallet.publicKey.toString());

    // 查找DEX账户PDA
    const [dexAccount] = PublicKey.findProgramAddressSync(
      [Buffer.from("dex_account")],
      program.programId
    );

    // 初始化DEX账户（如果尚未初始化）
    try {
      console.log("尝试初始化DEX账户...");
      await program.methods
        .initialize()
        .accounts({
          authority: provider.wallet.publicKey,
          dexAccount: dexAccount,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
      console.log("DEX账户已初始化");
    } catch (e) {
      console.log("DEX账户可能已初始化，继续执行:", e);
    }

    // ==================== 买入代币示例 ====================

    // 1. 设置买入参数
    const tokenToBuy = new PublicKey("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"); // USDC Devnet
    const solAmountIn = 0.1 * LAMPORTS_PER_SOL; // 0.1 SOL
    const minTokenOut = 0; // 最小获得的代币数量（这里设为0，实际使用时应设置合理的滑点保护）

    console.log("\n准备买入代币:");
    console.log("代币地址:", tokenToBuy.toString());
    console.log("输入SOL金额:", solAmountIn / LAMPORTS_PER_SOL, "SOL");

    // 2. 获取用户的代币账户
    // 注意：在实际应用中，需要先创建或获取关联代币账户
    // 这里简化处理，假设已经有了代币账户
    const userTokenAccount = new PublicKey("填入你的代币账户地址");
    console.log("用户代币账户:", userTokenAccount.toString());

    // 3. 执行买入操作
    try {
      console.log("执行买入操作...");
      const tx = await program.methods
        .smartTrade(
          tokenToBuy,
          new anchor.BN(solAmountIn),
          new anchor.BN(minTokenOut),
          true // true表示买入
        )
        .accounts({
          user: provider.wallet.publicKey,
          dexAccount: dexAccount,
          tokenMint: tokenToBuy,
          userSol: provider.wallet.publicKey,
          userToken: userTokenAccount,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          // 这里需要添加其他必要的账户，根据实际情况
        })
        .rpc();
      console.log("买入交易已确认:", tx);
      
      // 这里可以添加查询代币余额的代码
      console.log("买入后代币余额: [需要查询]");
    } catch (e) {
      console.log("买入交易失败:", e);
      console.log("注意: 在实际环境中需要提供完整的账户结构");
    }

    // ==================== 卖出代币示例 ====================

    // 1. 设置卖出参数
    const tokenToSell = tokenToBuy; // 使用相同的代币进行卖出示例
    const tokenAmountIn = 10 * 10**6; // 假设卖出10个USDC (USDC有6位小数)
    const minSolOut = 0; // 最小获得的SOL数量（这里设为0，实际使用时应设置合理的滑点保护）

    console.log("\n准备卖出代币:");
    console.log("代币地址:", tokenToSell.toString());
    console.log("输入代币金额:", tokenAmountIn / 10**6, "USDC");

    // 2. 执行卖出操作
    try {
      console.log("执行卖出操作...");
      const tx = await program.methods
        .smartTrade(
          tokenToSell,
          new anchor.BN(tokenAmountIn),
          new anchor.BN(minSolOut),
          false // false表示卖出
        )
        .accounts({
          user: provider.wallet.publicKey,
          dexAccount: dexAccount,
          tokenMint: tokenToSell,
          userSol: provider.wallet.publicKey,
          userToken: userTokenAccount,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          // 这里需要添加其他必要的账户，根据实际情况
        })
        .rpc();
      console.log("卖出交易已确认:", tx);
      
      // 这里可以添加查询SOL余额的代码
      console.log("卖出后SOL余额: [需要查询]");
    } catch (e) {
      console.log("卖出交易失败:", e);
      console.log("注意: 在实际环境中需要提供完整的账户结构");
    }

    // ==================== 直接在特定DEX上交易 ====================

    // 在Raydium上买入代币
    console.log("\n在Raydium上买入代币:");
    try {
      const tx = await program.methods
        .raydiumBuy(
          new anchor.BN(solAmountIn),
          new anchor.BN(minTokenOut)
        )
        .accounts({
          user: provider.wallet.publicKey,
          dexAccount: dexAccount,
          tokenMint: tokenToBuy,
          userSol: provider.wallet.publicKey,
          userToken: userTokenAccount,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          // 这里需要添加Raydium特定的账户，如池状态、AMM账户等
        })
        .rpc();
      console.log("Raydium买入交易已确认:", tx);
    } catch (e) {
      console.log("Raydium买入交易失败:", e);
    }

    // 在PumpFun上卖出代币
    console.log("\n在Pump.fun上卖出代币:");
    try {
      const tx = await program.methods
        .pumpSell(
          new anchor.BN(tokenAmountIn),
          new anchor.BN(minSolOut)
        )
        .accounts({
          user: provider.wallet.publicKey,
          dexAccount: dexAccount,
          tokenMint: tokenToSell,
          userSol: provider.wallet.publicKey,
          userToken: userTokenAccount,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          // 这里需要添加Pump.fun特定的账户
        })
        .rpc();
      console.log("Pump.fun卖出交易已确认:", tx);
    } catch (e) {
      console.log("Pump.fun卖出交易失败:", e);
    }

    console.log("\n交易示例完成!");
  } catch (error) {
    console.error("程序初始化失败:", error);
  }
}

main().then(
  () => process.exit(0),
  (err) => {
    console.error(err);
    process.exit(1);
  }
); 