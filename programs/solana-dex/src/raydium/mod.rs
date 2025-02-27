use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use solana_program::{
    program::invoke,
    pubkey::Pubkey,
    instruction::{Instruction, AccountMeta},
    system_instruction,
};
use std::str::FromStr;
use raydium_cpmm_cpi::{
    cpi,
    program::RaydiumCpmm,
    states::{AmmConfig, ObservationState, PoolState},
};
use crate::{MAX_PRICE_IMPACT, FEE_DENOMINATOR, DexError};

// Raydium CPMM程序ID
pub const RAYDIUM_CPMM_PROGRAM_ID: &str = "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C";

// 获取Raydium CPMM程序ID
pub fn get_raydium_cpmm_program_id() -> Pubkey {
    Pubkey::from_str(RAYDIUM_CPMM_PROGRAM_ID).unwrap()
}

// 在Raydium上购买代币
pub fn buy_token(
    ctx: Context<TradeTokenRaydium>,
    amount_in: u64,  // 输入的SOL数量
    min_amount_out: u64,  // 最小获得的代币数量（滑点控制）
) -> Result<()> {
    msg!("在Raydium上购买代币: {} SOL, 最小获得代币数量: {}", amount_in, min_amount_out);
    msg!("代币Mint地址: {}", ctx.accounts.token_mint.key());
    
    // 检查输入金额是否大于0
    require!(amount_in > 0, DexError::InvalidArgument);
    
    // 检查最小输出金额是否大于0
    require!(min_amount_out > 0, DexError::InvalidArgument);
    
    // 检查用户是否有足够的SOL
    let user_lamports = ctx.accounts.user.lamports();
    require!(user_lamports >= amount_in, DexError::InsufficientFunds);
    
    // 检查目标代币账户是否属于正确的代币类型
    require!(
        ctx.accounts.user_destination_token_account.mint == ctx.accounts.token_mint.key(),
        DexError::InvalidArgument
    );
    
    // 加载池状态（只加载一次）
    let pool_state = ctx.accounts.pool_state.load()?;
    
    // 计算价格影响
    let (price_impact, expected_amount_out) = calculate_price_impact(
        &pool_state,
        amount_in,
        true,
        ctx.accounts.token_mint.key()
    )?;
    
    // 检查价格影响是否过大
    require!(price_impact <= MAX_PRICE_IMPACT, DexError::ExcessivePriceImpact);
    
    // 记录预期输出金额和价格影响
    msg!("预期输出金额: {}, 价格影响: {}%", 
        expected_amount_out, 
        price_impact as f64 / 10_000.0
    );
    
    // 使用Raydium CPI进行交易
    msg!("使用Raydium CPI进行交易");
    
    // 构建CPI账户
    let cpi_accounts = cpi::accounts::Swap {
        payer: ctx.accounts.user.to_account_info(),
        authority: ctx.accounts.amm_authority.to_account_info(),
        amm_config: ctx.accounts.amm_config.to_account_info(),
        pool_state: ctx.accounts.pool_state.to_account_info(),
        input_token_account: ctx.accounts.user_source_token_account.to_account_info(),
        output_token_account: ctx.accounts.user_destination_token_account.to_account_info(),
        input_vault: ctx.accounts.input_vault.to_account_info(),
        output_vault: ctx.accounts.output_vault.to_account_info(),
        input_token_program: ctx.accounts.token_program.to_account_info(),
        output_token_program: ctx.accounts.token_program.to_account_info(),
        input_token_mint: ctx.accounts.input_token_mint.to_account_info(),
        output_token_mint: ctx.accounts.output_token_mint.to_account_info(),
        observation_state: ctx.accounts.observation_state.to_account_info(),
    };
    
    // 创建CPI上下文
    let cpi_program = ctx.accounts.raydium_program.to_account_info();
    let cpi_context = CpiContext::new(cpi_program, cpi_accounts);
    
    // 记录交易开始时间
    let start_time = Clock::get()?.unix_timestamp;
    
    // 执行交易
    msg!("执行Raydium swap_base_input交易");
    cpi::swap_base_input(cpi_context, amount_in, min_amount_out)?;
    
    // 记录交易结束时间
    let end_time = Clock::get()?.unix_timestamp;
    let execution_time = end_time - start_time;
    
    // 发出交易完成事件
    emit!(SwapExecuted {
        user: ctx.accounts.user.key(),
        dex: "Raydium".to_string(),
        token_in: ctx.accounts.input_token_mint.key(),
        token_out: ctx.accounts.output_token_mint.key(),
        amount_in,
        min_amount_out,
        actual_amount_out: expected_amount_out, // 实际值需要从交易后的账户余额中获取
        price_impact,
        execution_time,
        slot: Clock::get()?.slot,
    });
    
    msg!("交易完成，获得代币");
    Ok(())
}

// 在Raydium上卖出代币
pub fn sell_token(
    ctx: Context<TradeTokenRaydium>,
    amount_in: u64,  // 输入的代币数量
    min_amount_out: u64,  // 最小获得的SOL数量（滑点控制）
) -> Result<()> {
    msg!("在Raydium上卖出代币: {} 代币, 最小获得SOL数量: {}", amount_in, min_amount_out);
    msg!("代币Mint地址: {}", ctx.accounts.token_mint.key());
    
    // 检查输入金额是否大于0
    require!(amount_in > 0, DexError::InvalidArgument);
    
    // 检查最小输出金额是否大于0
    require!(min_amount_out > 0, DexError::InvalidArgument);
    
    // 检查用户代币账户是否有足够的代币
    require!(
        ctx.accounts.user_source_token_account.amount >= amount_in,
        DexError::InsufficientFunds
    );
    
    // 检查源代币账户是否属于正确的代币类型
    require!(
        ctx.accounts.user_source_token_account.mint == ctx.accounts.token_mint.key(),
        DexError::InvalidArgument
    );
    
    // 加载池状态（只加载一次）
    let pool_state = ctx.accounts.pool_state.load()?;
    
    // 计算价格影响
    let (price_impact, expected_amount_out) = calculate_price_impact(
        &pool_state,
        amount_in,
        false,
        ctx.accounts.token_mint.key()
    )?;
    
    // 检查价格影响是否过大
    require!(price_impact <= MAX_PRICE_IMPACT, DexError::ExcessivePriceImpact);
    
    // 记录预期输出金额和价格影响
    msg!("预期输出金额: {}, 价格影响: {}%", 
        expected_amount_out, 
        price_impact as f64 / 10_000.0
    );
    
    // 使用Raydium CPI进行交易
    msg!("使用Raydium CPI进行交易");
    
    // 构建CPI账户
    let cpi_accounts = cpi::accounts::Swap {
        payer: ctx.accounts.user.to_account_info(),
        authority: ctx.accounts.amm_authority.to_account_info(),
        amm_config: ctx.accounts.amm_config.to_account_info(),
        pool_state: ctx.accounts.pool_state.to_account_info(),
        input_token_account: ctx.accounts.user_source_token_account.to_account_info(),
        output_token_account: ctx.accounts.user_destination_token_account.to_account_info(),
        input_vault: ctx.accounts.input_vault.to_account_info(),
        output_vault: ctx.accounts.output_vault.to_account_info(),
        input_token_program: ctx.accounts.token_program.to_account_info(),
        output_token_program: ctx.accounts.token_program.to_account_info(),
        input_token_mint: ctx.accounts.input_token_mint.to_account_info(),
        output_token_mint: ctx.accounts.output_token_mint.to_account_info(),
        observation_state: ctx.accounts.observation_state.to_account_info(),
    };
    
    // 创建CPI上下文
    let cpi_program = ctx.accounts.raydium_program.to_account_info();
    let cpi_context = CpiContext::new(cpi_program, cpi_accounts);
    
    // 记录交易开始时间
    let start_time = Clock::get()?.unix_timestamp;
    
    // 执行交易
    msg!("执行Raydium swap_base_input交易");
    cpi::swap_base_input(cpi_context, amount_in, min_amount_out)?;
    
    // 记录交易结束时间
    let end_time = Clock::get()?.unix_timestamp;
    let execution_time = end_time - start_time;
    
    // 发出交易完成事件
    emit!(SwapExecuted {
        user: ctx.accounts.user.key(),
        dex: "Raydium".to_string(),
        token_in: ctx.accounts.input_token_mint.key(),
        token_out: ctx.accounts.output_token_mint.key(),
        amount_in,
        min_amount_out,
        actual_amount_out: expected_amount_out, // 实际值需要从交易后的账户余额中获取
        price_impact,
        execution_time,
        slot: Clock::get()?.slot,
    });
    
    msg!("交易完成，获得SOL");
    Ok(())
}

// 计算价格影响
fn calculate_price_impact(
    pool_state: &PoolState,
    amount_in: u64,
    is_buy: bool,
    token_mint: Pubkey,
) -> Result<(u64, u64)> {
    // 获取池子中的代币储备
    let token_0_vault_amount = pool_state.token_0_vault_amount;
    let token_1_vault_amount = pool_state.token_1_vault_amount;
    
    // 如果储备为0，返回错误
    require!(
        token_0_vault_amount > 0 && token_1_vault_amount > 0,
        DexError::InsufficientLiquidity
    );
    
    // 确定输入和输出代币
    let (input_amount, output_amount) = if token_mint == pool_state.token_0_mint {
        // 如果查询的是代币0
        if is_buy {
            // 买入代币0，使用代币1作为输入
            (token_1_vault_amount, token_0_vault_amount)
        } else {
            // 卖出代币0，使用代币0作为输入
            (token_0_vault_amount, token_1_vault_amount)
        }
    } else {
        // 如果查询的是代币1
        if is_buy {
            // 买入代币1，使用代币0作为输入
            (token_0_vault_amount, token_1_vault_amount)
        } else {
            // 卖出代币1，使用代币1作为输入
            (token_1_vault_amount, token_0_vault_amount)
        }
    };
    
    // 使用恒定乘积公式计算价格
    // 公式: k = input_amount * output_amount
    let k = input_amount
        .checked_mul(output_amount)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    
    // 计算交易后的新储备
    let new_input_amount = input_amount
        .checked_add(amount_in)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    
    let new_output_amount = k
        .checked_div(new_input_amount)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    
    // 计算输出金额
    let amount_out = output_amount
        .checked_sub(new_output_amount)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    
    // 考虑交易费用
    let fee_rate = pool_state.fee_rate;
    let fee_amount = amount_out
        .checked_mul(fee_rate as u64)
        .unwrap_or(0)
        .checked_div(FEE_DENOMINATOR)
        .unwrap_or(0);
    
    let amount_out_after_fee = amount_out.checked_sub(fee_amount).unwrap_or(amount_out);
    
    // 计算价格影响（以万分比表示）
    let ideal_price = output_amount
        .checked_mul(10_000)
        .unwrap_or(0)
        .checked_div(input_amount)
        .unwrap_or(0);
    
    let actual_price = amount_out_after_fee
        .checked_mul(10_000)
        .unwrap_or(0)
        .checked_div(amount_in)
        .unwrap_or(0);
    
    let price_impact = if ideal_price > actual_price {
        ideal_price
            .checked_sub(actual_price)
            .unwrap_or(0)
            .checked_mul(10_000)
            .unwrap_or(0)
            .checked_div(ideal_price)
            .unwrap_or(0)
    } else {
        0
    };
    
    Ok((price_impact, amount_out_after_fee))
}

// Raydium交易所需的账户结构
#[derive(Accounts)]
pub struct TradeTokenRaydium<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    
    // DEX账户
    #[account(
        mut,
        seeds = [b"dex_account".as_ref()],
        bump
    )]
    pub dex_account: Account<'info, crate::DexAccount>,
    
    // 代币Mint地址
    pub token_mint: Account<'info, token::Mint>,
    
    // Raydium程序
    pub raydium_program: Program<'info, RaydiumCpmm>,
    
    // Raydium AMM账户
    pub amm_authority: UncheckedAccount<'info>,
    pub amm_config: Box<Account<'info, AmmConfig>>,
    #[account(mut)]
    pub pool_state: AccountLoader<'info, PoolState>,
    
    // 代币账户
    #[account(mut)]
    pub input_vault: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub output_vault: Box<Account<'info, TokenAccount>>,
    
    // 用户账户
    #[account(mut)]
    pub user_source_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub user_destination_token_account: Account<'info, TokenAccount>,
    
    // 代币Mint
    pub input_token_mint: Box<Account<'info, token::Mint>>,
    pub output_token_mint: Box<Account<'info, token::Mint>>,
    
    // 观察状态
    #[account(mut)]
    pub observation_state: AccountLoader<'info, ObservationState>,
    
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

// 检查代币位置所需的账户结构
#[derive(Accounts)]
pub struct CheckTokenLocation<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    
    // DEX账户
    #[account(
        seeds = [b"dex_account".as_ref()],
        bump
    )]
    pub dex_account: Account<'info, crate::DexAccount>,
    
    // Raydium程序
    pub raydium_program: Program<'info, RaydiumCpmm>,
    
    // Raydium AMM账户
    pub amm_authority: UncheckedAccount<'info>,
    pub amm_config: Box<Account<'info, AmmConfig>>,
    #[account(mut)]
    pub pool_state: AccountLoader<'info, PoolState>,
    
    // 系统程序
    pub system_program: Program<'info, System>,
    
    // Token程序
    pub token_program: Program<'info, Token>,
}

// 获取价格所需的账户结构
#[derive(Accounts)]
pub struct GetPriceContext<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    
    // DEX账户
    #[account(
        seeds = [b"dex_account".as_ref()],
        bump
    )]
    pub dex_account: Account<'info, crate::DexAccount>,
    
    // 代币Mint地址
    pub token_mint: Account<'info, token::Mint>,
    
    // Raydium程序
    pub raydium_program: Program<'info, RaydiumCpmm>,
    
    // Raydium AMM账户
    pub amm_authority: UncheckedAccount<'info>,
    pub amm_config: Box<Account<'info, AmmConfig>>,
    #[account(mut)]
    pub pool_state: AccountLoader<'info, PoolState>,
    
    // 代币账户
    #[account(mut)]
    pub input_vault: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub output_vault: Box<Account<'info, TokenAccount>>,
    
    // 观察状态
    #[account(mut)]
    pub observation_state: AccountLoader<'info, ObservationState>,
    
    // 系统程序
    pub system_program: Program<'info, System>,
    
    // Token程序
    pub token_program: Program<'info, Token>,
}

// 检查代币是否在Raydium上
pub fn is_token_on_raydium(
    ctx: Context<CheckTokenLocation>,
    token_mint: Pubkey,
) -> Result<bool> {
    // 记录查询信息
    msg!("检查代币是否在Raydium上: {}", token_mint);
    
    // 从池状态中读取代币信息（只加载一次）
    let pool_state = ctx.accounts.pool_state.load()?;
    
    // 获取池子中的代币Mint
    let token_0_mint = pool_state.token_0_mint;
    let token_1_mint = pool_state.token_1_mint;
    
    msg!("池子代币0 Mint: {}", token_0_mint);
    msg!("池子代币1 Mint: {}", token_1_mint);
    msg!("查询代币Mint: {}", token_mint);
    
    // 检查代币是否与池子中的代币匹配
    if token_0_mint == token_mint || token_1_mint == token_mint {
        msg!("代币在Raydium池子中找到");
        
        // 检查池子流动性
        let token_0_vault_amount = pool_state.token_0_vault_amount;
        let token_1_vault_amount = pool_state.token_1_vault_amount;
        
        msg!("池子代币0数量: {}", token_0_vault_amount);
        msg!("池子代币1数量: {}", token_1_vault_amount);
        
        // 检查流动性是否足够
        if token_0_vault_amount > 0 && token_1_vault_amount > 0 {
            msg!("池子流动性充足，代币在Raydium上可用");
            return Ok(true);
        } else {
            msg!("池子流动性不足，代币在Raydium上不可用");
            return Ok(false);
        }
    } else {
        msg!("代币在Raydium池子中未找到");
        return Ok(false);
    }
}

// 获取Raydium上的价格
pub fn get_price(
    ctx: Context<GetPriceContext>,
    amount_in: u64,
    is_buy: bool,  // true表示买入，false表示卖出
) -> Result<u64> {
    // 检查输入金额是否大于0
    require!(amount_in > 0, DexError::InvalidArgument);
    
    // 记录查询信息
    msg!("查询Raydium上的价格: 输入金额 {}, 操作类型: {}", 
        amount_in, 
        if is_buy { "买入" } else { "卖出" }
    );
    msg!("代币Mint地址: {}", ctx.accounts.token_mint.key());
    
    // 从池状态中读取代币信息（只加载一次）
    let pool_state = ctx.accounts.pool_state.load()?;
    
    // 计算价格影响和预期输出金额
    let (price_impact, amount_out) = calculate_price_impact(
        &pool_state,
        amount_in,
        is_buy,
        ctx.accounts.token_mint.key()
    )?;
    
    // 计算价格比率
    let price_ratio = if amount_in > 0 {
        amount_out
            .checked_mul(1_000_000_000)
            .unwrap_or(0)
            .checked_div(amount_in)
            .unwrap_or(0)
    } else {
        0
    };
    
    msg!("Raydium上的最终价格: {}, 价格影响: {}%", 
        price_ratio, 
        price_impact as f64 / 10_000.0
    );
    
    Ok(price_ratio)
}

// 交易执行事件
#[event]
pub struct SwapExecuted {
    pub user: Pubkey,
    pub dex: String,
    pub token_in: Pubkey,
    pub token_out: Pubkey,
    pub amount_in: u64,
    pub min_amount_out: u64,
    pub actual_amount_out: u64,
    pub price_impact: u64,
    pub execution_time: i64,
    pub slot: u64,
} 