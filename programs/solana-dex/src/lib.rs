use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token};
use solana_program::{system_instruction, pubkey::Pubkey};

// 导入模块
pub mod pumpfun;
pub mod raydium;
pub mod router;
pub mod mev_protection;

// 常量定义
pub const MAX_PRICE_IMPACT: u64 = 50_000; // 5%的最大价格影响
pub const FEE_DENOMINATOR: u64 = 1_000_000;
pub const DEFAULT_FEE_RATE: u64 = 3_000; // 0.3%
pub const PROTOCOL_VERSION: u8 = 1;
pub const MIN_COMMITMENT_DELAY: u64 = 2; // 最小承诺延迟（区块数）
pub const MAX_COMMITMENT_DELAY: u64 = 100; // 最大承诺延迟（区块数）
pub const COMMITMENT_EXPIRY: u64 = 150; // 承诺过期时间（区块数）

declare_id!("FZ6RHhMSv5xoE8GjK5KJi2i7Gue1DW3APGe4an4CJjte");

#[program]
pub mod solana_dex {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let dex_account = &mut ctx.accounts.dex_account;
        dex_account.authority = ctx.accounts.authority.key();
        dex_account.bump = *ctx.bumps.get("dex_account").unwrap();
        dex_account.protocol_version = PROTOCOL_VERSION;
        dex_account.locked = false;
        dex_account.total_commitments = 0;
        dex_account.executed_commitments = 0;
        dex_account.expired_commitments = 0;
        msg!("DEX账户已初始化");
        Ok(())
    }

    // 在Pump.fun上购买代币
    pub fn buy_token_on_pump(
        ctx: Context<pumpfun::TradeToken>,
        amount_in: u64,
        min_amount_out: u64,
    ) -> Result<()> {
        // 检查重入锁
        let dex_account = &mut ctx.accounts.dex_account;
        require!(!dex_account.locked, DexError::ReentrancyDetected);
        
        // 设置锁定状态
        dex_account.locked = true;
        
        // 记录交易开始时间
        let start_time = Clock::get()?.unix_timestamp;
        
        // 执行交易
        let result = pumpfun::buy_token(ctx, amount_in, min_amount_out);
        
        // 解除锁定状态
        dex_account.locked = false;
        
        // 记录交易结束时间和性能指标
        let end_time = Clock::get()?.unix_timestamp;
        let execution_time = end_time - start_time;
        msg!("交易执行时间: {}ms", execution_time);
        
        result
    }

    // 在Pump.fun上卖出代币
    pub fn sell_token_on_pump(
        ctx: Context<pumpfun::TradeToken>,
        amount_in: u64,
        min_amount_out: u64,
    ) -> Result<()> {
        // 检查重入锁
        let dex_account = &mut ctx.accounts.dex_account;
        require!(!dex_account.locked, DexError::ReentrancyDetected);
        
        // 设置锁定状态
        dex_account.locked = true;
        
        // 记录交易开始时间
        let start_time = Clock::get()?.unix_timestamp;
        
        // 执行交易
        let result = pumpfun::sell_token(ctx, amount_in, min_amount_out);
        
        // 解除锁定状态
        dex_account.locked = false;
        
        // 记录交易结束时间和性能指标
        let end_time = Clock::get()?.unix_timestamp;
        let execution_time = end_time - start_time;
        msg!("交易执行时间: {}ms", execution_time);
        
        result
    }

    // 在Raydium上购买代币
    pub fn buy_token_on_raydium(
        ctx: Context<raydium::TradeTokenRaydium>,
        amount_in: u64,
        min_amount_out: u64,
    ) -> Result<()> {
        // 检查重入锁
        let dex_account = &mut ctx.accounts.dex_account;
        require!(!dex_account.locked, DexError::ReentrancyDetected);
        
        // 设置锁定状态
        dex_account.locked = true;
        
        // 记录交易开始时间
        let start_time = Clock::get()?.unix_timestamp;
        
        // 执行交易
        let result = raydium::buy_token(ctx, amount_in, min_amount_out);
        
        // 解除锁定状态
        dex_account.locked = false;
        
        // 记录交易结束时间和性能指标
        let end_time = Clock::get()?.unix_timestamp;
        let execution_time = end_time - start_time;
        msg!("交易执行时间: {}ms", execution_time);
        
        result
    }

    // 在Raydium上卖出代币
    pub fn sell_token_on_raydium(
        ctx: Context<raydium::TradeTokenRaydium>,
        amount_in: u64,
        min_amount_out: u64,
    ) -> Result<()> {
        // 检查重入锁
        let dex_account = &mut ctx.accounts.dex_account;
        require!(!dex_account.locked, DexError::ReentrancyDetected);
        
        // 设置锁定状态
        dex_account.locked = true;
        
        // 记录交易开始时间
        let start_time = Clock::get()?.unix_timestamp;
        
        // 执行交易
        let result = raydium::sell_token(ctx, amount_in, min_amount_out);
        
        // 解除锁定状态
        dex_account.locked = false;
        
        // 记录交易结束时间和性能指标
        let end_time = Clock::get()?.unix_timestamp;
        let execution_time = end_time - start_time;
        msg!("交易执行时间: {}ms", execution_time);
        
        result
    }

    // 检查代币位置
    pub fn check_token_location(
        ctx: Context<router::CheckTokenLocationContext>,
        token_mint: Pubkey,
    ) -> Result<bool> {
        router::check_token_location(ctx, token_mint)
    }

    // 获取最优价格路由
    pub fn get_best_price(
        ctx: Context<router::GetBestPrice>,
        amount_in: u64,
        is_buy: bool,
    ) -> Result<bool> {
        router::get_best_price(ctx, amount_in, is_buy)
    }

    // 智能路由：自动选择正确的DEX进行交易
    pub fn smart_trade(
        ctx: Context<router::SmartTradeContext>,
        token_mint: Pubkey,
        amount_in: u64,
        min_amount_out: u64,
        is_buy: bool,
    ) -> Result<()> {
        // 检查重入锁
        let dex_account = &mut ctx.accounts.dex_account;
        require!(!dex_account.locked, DexError::ReentrancyDetected);
        
        // 设置锁定状态
        dex_account.locked = true;
        
        // 记录交易开始时间
        let start_time = Clock::get()?.unix_timestamp;
        
        // 执行交易
        let result = router::smart_trade(ctx, token_mint, amount_in, min_amount_out, is_buy);
        
        // 解除锁定状态
        dex_account.locked = false;
        
        // 记录交易结束时间和性能指标
        let end_time = Clock::get()?.unix_timestamp;
        let execution_time = end_time - start_time;
        msg!("交易执行时间: {}ms", execution_time);
        
        result
    }
    
    // 批量交易：一次执行多个交易指令
    pub fn batch_trade(
        ctx: Context<router::BatchTradeContext>,
        instructions: Vec<TradeInstruction>,
    ) -> Result<()> {
        // 检查重入锁
        let dex_account = &mut ctx.accounts.dex_account;
        require!(!dex_account.locked, DexError::ReentrancyDetected);
        
        // 设置锁定状态
        dex_account.locked = true;
        
        // 记录交易开始时间
        let start_time = Clock::get()?.unix_timestamp;
        
        // 执行批量交易
        let result = router::batch_trade(ctx, instructions);
        
        // 解除锁定状态
        dex_account.locked = false;
        
        // 记录交易结束时间和性能指标
        let end_time = Clock::get()?.unix_timestamp;
        let execution_time = end_time - start_time;
        msg!("批量交易执行时间: {}ms", execution_time);
        
        result
    }

    // MEV保护：创建交易承诺
    pub fn create_trade_commitment(
        ctx: Context<mev_protection::CreateCommitment>,
        commitment_hash: [u8; 32],
        min_slot_delay: u64,
    ) -> Result<()> {
        mev_protection::create_commitment(ctx, commitment_hash, min_slot_delay)
    }

    // MEV保护：执行承诺交易
    pub fn execute_committed_trade(
        ctx: Context<mev_protection::ExecuteCommitment>,
        token_mint: Pubkey,
        amount_in: u64,
        min_amount_out: u64,
        is_buy: bool,
        dex_type: DexType,
        nonce: [u8; 32],
    ) -> Result<()> {
        mev_protection::execute_commitment(
            ctx,
            token_mint,
            amount_in,
            min_amount_out,
            is_buy,
            dex_type,
            nonce,
        )
    }

    // MEV保护：批量执行承诺交易
    pub fn batch_execute_committed_trades(
        ctx: Context<mev_protection::BatchExecuteCommitment>,
        params: Vec<mev_protection::CommitmentExecutionParams>,
    ) -> Result<()> {
        // 检查重入锁
        let dex_account = &mut ctx.accounts.dex_account;
        require!(!dex_account.locked, DexError::ReentrancyDetected);
        
        // 设置锁定状态
        dex_account.locked = true;
        
        // 记录交易开始时间
        let start_time = Clock::get()?.unix_timestamp;
        
        // 执行批量承诺交易
        let result = mev_protection::batch_execute_commitments(ctx, params);
        
        // 解除锁定状态
        dex_account.locked = false;
        
        // 记录交易结束时间和性能指标
        let end_time = Clock::get()?.unix_timestamp;
        let execution_time = end_time - start_time;
        msg!("批量承诺交易执行时间: {}ms", execution_time);
        
        result
    }

    // MEV保护：检查承诺是否过期
    pub fn check_commitment_expiry(
        ctx: Context<mev_protection::CheckExpiredCommitment>,
    ) -> Result<()> {
        mev_protection::check_expired_commitment(ctx)
    }

    // MEV保护：查询承诺统计
    pub fn get_commitment_statistics(
        ctx: Context<mev_protection::GetCommitmentStats>,
    ) -> Result<mev_protection::CommitmentStats> {
        mev_protection::get_commitment_stats(ctx)
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    
    #[account(
        init,
        payer = authority,
        space = 8 + 32 + 1 + 1 + 1 + 8 + 8 + 8,  // 8字节discriminator + 32字节pubkey + 1字节bump + 1字节version + 1字节locked + 8字节total_commitments + 8字节executed_commitments + 8字节expired_commitments
        seeds = [b"dex_account".as_ref()],
        bump
    )]
    pub dex_account: Account<'info, DexAccount>,
    
    pub system_program: Program<'info, System>,
}

#[account]
pub struct DexAccount {
    pub authority: Pubkey,
    pub bump: u8,
    pub protocol_version: u8,
    pub locked: bool,
    // 承诺交易统计
    pub total_commitments: u64,
    pub executed_commitments: u64,
    pub expired_commitments: u64,
}

// 交易指令结构
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct TradeInstruction {
    pub token_mint: Pubkey,
    pub amount_in: u64,
    pub min_amount_out: u64,
    pub is_buy: bool,
    pub dex_type: DexType,  // 0 = Auto, 1 = Pump.fun, 2 = Raydium
}

// DEX类型枚举
#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq)]
pub enum DexType {
    Auto,
    PumpFun,
    Raydium,
}

// 错误码定义
#[error_code]
pub enum DexError {
    #[msg("重入攻击检测")]
    ReentrancyDetected,
    
    #[msg("价格影响过大")]
    ExcessivePriceImpact,
    
    #[msg("K值减少")]
    KValueReduced,
    
    #[msg("交易超时")]
    TransactionTimeout,
    
    #[msg("无效的DEX类型")]
    InvalidDexType,
    
    #[msg("批量交易指令为空")]
    EmptyBatchInstructions,
    
    #[msg("批量交易指令过多")]
    TooManyBatchInstructions,
    
    #[msg("代币未找到")]
    TokenNotFound,
    
    #[msg("价格查询失败")]
    PriceQueryFailed,
    
    #[msg("无效参数")]
    InvalidArgument,
    
    #[msg("资金不足")]
    InsufficientFunds,
    
    #[msg("流动性不足")]
    InsufficientLiquidity,
    
    #[msg("算术溢出")]
    ArithmeticOverflow,
    
    #[msg("交易被拒绝")]
    TransactionRejected,
    
    #[msg("承诺哈希不匹配")]
    CommitmentHashMismatch,
    
    #[msg("承诺尚未成熟")]
    CommitmentNotMatured,
    
    #[msg("承诺已过期")]
    CommitmentExpired,
    
    #[msg("承诺已执行")]
    CommitmentAlreadyExecuted,
    
    #[msg("无效的区块延迟")]
    InvalidSlotDelay,
    
    #[msg("承诺尚未过期")]
    CommitmentNotExpired,
    
    #[msg("批量承诺执行失败")]
    BatchCommitmentExecutionFailed,
}
