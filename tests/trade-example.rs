use anchor_client::{
    solana_sdk::{
        commitment_config::CommitmentConfig,
        pubkey::Pubkey,
        signature::{Keypair, Signer},
        system_program,
    },
    Client, Program,
};
use std::{rc::Rc, str::FromStr, time::Duration};
use anyhow::Result;

// 常量定义
const PROGRAM_ID: &str = "FZ6RHhMSv5xoE8GjK5KJi2i7Gue1DW3APGe4an4CJjte";
const RPC_URL: &str = "https://api.devnet.solana.com";
const TOKEN_PROGRAM_ID: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

// 主函数
#[tokio::main]
async fn main() -> Result<()> {
    println!("开始交易示例...");
    
    // 设置客户端
    let payer = Keypair::new();
    let program_id = Pubkey::from_str(PROGRAM_ID)?;
    
    // 创建Anchor客户端
    let client = Client::new_with_options(
        cluster_from_url(RPC_URL),
        Rc::new(payer),
        CommitmentConfig::confirmed(),
    );
    let program = client.program(program_id);
    
    println!("钱包地址: {}", program.payer());
    
    // 请求空投SOL用于测试
    request_airdrop(&program, 1_000_000_000).await?; // 1 SOL
    println!("已空投 1 SOL 到钱包");
    
    // 查找DEX账户PDA
    let seeds = &[b"dex_account".as_ref()];
    let (dex_account, _) = Pubkey::find_program_address(seeds, &program.id());
    
    // 初始化DEX账户
    match initialize_dex(&program, dex_account).await {
        Ok(_) => println!("DEX账户已初始化"),
        Err(e) => println!("DEX账户可能已初始化，继续执行: {}", e),
    }
    
    // ==================== 买入代币示例 ====================
    
    // 1. 设置买入参数
    let token_to_buy = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v")?; // USDC Devnet
    let sol_amount_in = 100_000_000; // 0.1 SOL (lamports)
    let min_token_out = 0; // 最小获得的代币数量（这里设为0，实际使用时应设置合理的滑点保护）
    
    println!("\n准备买入代币:");
    println!("代币地址: {}", token_to_buy);
    println!("输入SOL金额: {} lamports (0.1 SOL)", sol_amount_in);
    
    // 2. 获取用户的代币账户
    // 注意：在实际应用中，需要先创建或获取关联代币账户
    // 这里简化处理，假设已经有了代币账户
    let user_token_account = get_or_create_token_account(&program, token_to_buy).await?;
    println!("用户代币账户: {}", user_token_account);
    
    // 3. 执行买入操作
    match buy_token(&program, dex_account, token_to_buy, sol_amount_in, min_token_out, user_token_account).await {
        Ok(signature) => {
            println!("买入交易已确认: {}", signature);
            
            // 这里可以添加查询代币余额的代码
            println!("买入后代币余额: [需要查询]");
        },
        Err(e) => {
            println!("买入交易失败: {}", e);
            println!("注意: 在实际环境中需要提供完整的账户结构");
        }
    }
    
    // ==================== 卖出代币示例 ====================
    
    // 1. 设置卖出参数
    let token_to_sell = token_to_buy; // 使用相同的代币进行卖出示例
    let token_amount_in = 10_000_000; // 假设卖出10个USDC (USDC有6位小数)
    let min_sol_out = 0; // 最小获得的SOL数量（这里设为0，实际使用时应设置合理的滑点保护）
    
    println!("\n准备卖出代币:");
    println!("代币地址: {}", token_to_sell);
    println!("输入代币金额: {} (10 USDC)", token_amount_in);
    
    // 2. 执行卖出操作
    match sell_token(&program, dex_account, token_to_sell, token_amount_in, min_sol_out, user_token_account).await {
        Ok(signature) => {
            println!("卖出交易已确认: {}", signature);
            
            // 这里可以添加查询SOL余额的代码
            println!("卖出后SOL余额: [需要查询]");
        },
        Err(e) => {
            println!("卖出交易失败: {}", e);
            println!("注意: 在实际环境中需要提供完整的账户结构");
        }
    }
    
    // ==================== 直接在特定DEX上交易 ====================
    
    // 在Raydium上买入代币
    println!("\n在Raydium上买入代币:");
    match buy_token_on_raydium(&program, dex_account, token_to_buy, sol_amount_in, min_token_out, user_token_account).await {
        Ok(signature) => println!("Raydium买入交易已确认: {}", signature),
        Err(e) => println!("Raydium买入交易失败: {}", e),
    }
    
    // 在PumpFun上卖出代币
    println!("\n在Pump.fun上卖出代币:");
    match sell_token_on_pump(&program, dex_account, token_to_sell, token_amount_in, min_sol_out, user_token_account).await {
        Ok(signature) => println!("Pump.fun卖出交易已确认: {}", signature),
        Err(e) => println!("Pump.fun卖出交易失败: {}", e),
    }
    
    println!("\n交易示例完成!");
    Ok(())
}

// 从URL获取集群
fn cluster_from_url(url: &str) -> anchor_client::Cluster {
    anchor_client::Cluster::Custom(url.to_string(), url.to_string())
}

// 请求空投SOL
async fn request_airdrop(program: &Program, lamports: u64) -> Result<()> {
    let signature = program
        .rpc()
        .request_airdrop(&program.payer(), lamports)?;
    program.rpc().poll_for_signature(&signature)?;
    Ok(())
}

// 初始化DEX账户
async fn initialize_dex(program: &Program, dex_account: Pubkey) -> Result<()> {
    let tx = program
        .request()
        .accounts(initialize_accounts(program, dex_account))
        .args(initialize_args())
        .send()?;
    Ok(())
}

// 获取或创建代币账户（简化版，实际应用中需要更复杂的逻辑）
async fn get_or_create_token_account(program: &Program, token_mint: Pubkey) -> Result<Pubkey> {
    // 这里简化处理，返回一个新的公钥
    // 实际应用中应该使用SPL Token程序创建关联代币账户
    Ok(Pubkey::new_unique())
}

// 买入代币（使用智能路由）
async fn buy_token(
    program: &Program,
    dex_account: Pubkey,
    token_mint: Pubkey,
    sol_amount_in: u64,
    min_token_out: u64,
    user_token_account: Pubkey,
) -> Result<String> {
    let tx = program
        .request()
        .accounts(smart_trade_accounts(
            program,
            dex_account,
            token_mint,
            user_token_account,
        ))
        .args(smart_trade_args(
            token_mint,
            sol_amount_in,
            min_token_out,
            true, // true表示买入
        ))
        .send()?;
    Ok(tx.to_string())
}

// 卖出代币（使用智能路由）
async fn sell_token(
    program: &Program,
    dex_account: Pubkey,
    token_mint: Pubkey,
    token_amount_in: u64,
    min_sol_out: u64,
    user_token_account: Pubkey,
) -> Result<String> {
    let tx = program
        .request()
        .accounts(smart_trade_accounts(
            program,
            dex_account,
            token_mint,
            user_token_account,
        ))
        .args(smart_trade_args(
            token_mint,
            token_amount_in,
            min_sol_out,
            false, // false表示卖出
        ))
        .send()?;
    Ok(tx.to_string())
}

// 在Raydium上买入代币
async fn buy_token_on_raydium(
    program: &Program,
    dex_account: Pubkey,
    token_mint: Pubkey,
    sol_amount_in: u64,
    min_token_out: u64,
    user_token_account: Pubkey,
) -> Result<String> {
    let tx = program
        .request()
        .accounts(raydium_trade_accounts(
            program,
            dex_account,
            token_mint,
            user_token_account,
        ))
        .args(raydium_buy_args(
            sol_amount_in,
            min_token_out,
        ))
        .send()?;
    Ok(tx.to_string())
}

// 在PumpFun上卖出代币
async fn sell_token_on_pump(
    program: &Program,
    dex_account: Pubkey,
    token_mint: Pubkey,
    token_amount_in: u64,
    min_sol_out: u64,
    user_token_account: Pubkey,
) -> Result<String> {
    let tx = program
        .request()
        .accounts(pump_trade_accounts(
            program,
            dex_account,
            token_mint,
            user_token_account,
        ))
        .args(pump_sell_args(
            token_amount_in,
            min_sol_out,
        ))
        .send()?;
    Ok(tx.to_string())
}

// 以下是各种指令的账户和参数结构
// 注意：这些结构在实际使用时需要根据程序的IDL生成
// 这里仅作为示例，实际使用时应该使用anchor-client生成的类型

// 初始化账户结构
fn initialize_accounts(program: &Program, dex_account: Pubkey) -> Vec<(&'static str, Pubkey)> {
    vec![
        ("authority", program.payer()),
        ("dexAccount", dex_account),
        ("systemProgram", system_program::ID),
    ]
}

// 初始化参数
fn initialize_args() -> Vec<(&'static str, &'static str)> {
    vec![]
}

// 智能交易账户结构
fn smart_trade_accounts(
    program: &Program,
    dex_account: Pubkey,
    token_mint: Pubkey,
    user_token_account: Pubkey,
) -> Vec<(&'static str, Pubkey)> {
    vec![
        ("user", program.payer()),
        ("dexAccount", dex_account),
        ("tokenMint", token_mint),
        ("userSol", program.payer()),
        ("userToken", user_token_account),
        ("systemProgram", system_program::ID),
        ("tokenProgram", Pubkey::from_str(TOKEN_PROGRAM_ID).unwrap()),
        // 这里需要添加其他必要的账户，根据实际情况
    ]
}

// 智能交易参数
fn smart_trade_args(
    token_mint: Pubkey,
    amount_in: u64,
    min_amount_out: u64,
    is_buy: bool,
) -> Vec<(&'static str, anchor_client::anchor_lang::ToAccountMetas)> {
    vec![
        ("tokenMint", token_mint),
        ("amountIn", amount_in),
        ("minAmountOut", min_amount_out),
        ("isBuy", is_buy),
    ]
}

// Raydium交易账户结构
fn raydium_trade_accounts(
    program: &Program,
    dex_account: Pubkey,
    token_mint: Pubkey,
    user_token_account: Pubkey,
) -> Vec<(&'static str, Pubkey)> {
    vec![
        ("user", program.payer()),
        ("dexAccount", dex_account),
        ("tokenMint", token_mint),
        ("userSol", program.payer()),
        ("userToken", user_token_account),
        ("systemProgram", system_program::ID),
        ("tokenProgram", Pubkey::from_str(TOKEN_PROGRAM_ID).unwrap()),
        // 这里需要添加Raydium特定的账户，如池状态、AMM账户等
    ]
}

// Raydium买入参数
fn raydium_buy_args(
    amount_in: u64,
    min_amount_out: u64,
) -> Vec<(&'static str, anchor_client::anchor_lang::ToAccountMetas)> {
    vec![
        ("amountIn", amount_in),
        ("minAmountOut", min_amount_out),
    ]
}

// PumpFun交易账户结构
fn pump_trade_accounts(
    program: &Program,
    dex_account: Pubkey,
    token_mint: Pubkey,
    user_token_account: Pubkey,
) -> Vec<(&'static str, Pubkey)> {
    vec![
        ("user", program.payer()),
        ("dexAccount", dex_account),
        ("tokenMint", token_mint),
        ("userSol", program.payer()),
        ("userToken", user_token_account),
        ("systemProgram", system_program::ID),
        ("tokenProgram", Pubkey::from_str(TOKEN_PROGRAM_ID).unwrap()),
        // 这里需要添加Pump.fun特定的账户
    ]
}

// PumpFun卖出参数
fn pump_sell_args(
    amount_in: u64,
    min_amount_out: u64,
) -> Vec<(&'static str, anchor_client::anchor_lang::ToAccountMetas)> {
    vec![
        ("amountIn", amount_in),
        ("minAmountOut", min_amount_out),
    ]
} 