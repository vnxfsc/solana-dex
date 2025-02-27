use anchor_lang::prelude::*;
use crate::pumpfun;
use crate::raydium;
use crate::{DexError, TradeInstruction, DexType, MAX_PRICE_IMPACT};
use solana_program::pubkey::Pubkey;

// 常量定义
pub const MAX_BATCH_SIZE: usize = 5; // 最大批量交易指令数量

// 检查代币位置并选择正确的DEX
pub fn check_token_location(
    ctx: Context<CheckTokenLocationContext>,
    token_mint: Pubkey,
) -> Result<bool> {
    msg!("检查代币位置: {}", token_mint);
    
    // 记录检查开始时间
    let start_time = Clock::get()?.unix_timestamp;
    
    // 首先检查代币是否在Pump.fun上
    let on_pump = pumpfun::is_token_on_pump(
        ctx.accounts.pump_check_ctx.into(),
        token_mint,
    )?;
    
    // 如果代币在Pump.fun上，返回true
    if on_pump {
        msg!("代币在Pump.fun上");
        
        // 记录检查结束时间
        let end_time = Clock::get()?.unix_timestamp;
        let execution_time = end_time - start_time;
        msg!("代币位置检查执行时间: {}ms", execution_time);
        
        return Ok(true);
    }
    
    // 如果代币不在Pump.fun上，检查是否在Raydium上
    let on_raydium = raydium::is_token_on_raydium(
        ctx.accounts.raydium_check_ctx.into(),
        token_mint,
    )?;
    
    if on_raydium {
        msg!("代币在Raydium上");
        
        // 记录检查结束时间
        let end_time = Clock::get()?.unix_timestamp;
        let execution_time = end_time - start_time;
        msg!("代币位置检查执行时间: {}ms", execution_time);
        
        return Ok(false);
    }
    
    // 如果代币既不在Pump.fun上也不在Raydium上，返回错误
    msg!("代币既不在Pump.fun上也不在Raydium上");
    
    // 记录检查结束时间
    let end_time = Clock::get()?.unix_timestamp;
    let execution_time = end_time - start_time;
    msg!("代币位置检查执行时间: {}ms", execution_time);
    
    Err(DexError::TokenNotFound.into())
}

// 获取最优价格路由
pub fn get_best_price(
    ctx: Context<GetBestPrice>,
    amount_in: u64,
    is_buy: bool,  // true表示买入，false表示卖出
) -> Result<bool> {
    msg!("开始比较DEX价格: 输入金额 {}, 操作类型: {}", 
        amount_in, 
        if is_buy { "买入" } else { "卖出" }
    );
    msg!("代币Mint地址: {}", ctx.accounts.token_mint.key());
    
    // 记录价格查询开始时间
    let start_time = Clock::get()?.unix_timestamp;
    
    // 获取Pump.fun的价格
    let pump_price_result = pumpfun::get_price(
        ctx.accounts.pump_price_ctx.into(),
        amount_in,
        is_buy,
    );
    
    // 获取Raydium的价格
    let raydium_price_result = raydium::get_price(
        ctx.accounts.raydium_price_ctx.into(),
        amount_in,
        is_buy,
    );
    
    // 处理价格查询结果
    let (pump_price, raydium_price) = match (pump_price_result, raydium_price_result) {
        (Ok(pump), Ok(raydium)) => (pump, raydium),
        (Ok(pump), Err(_)) => {
            msg!("Raydium价格查询失败，使用Pump.fun");
            return Ok(true);
        },
        (Err(_), Ok(raydium)) => {
            msg!("Pump.fun价格查询失败，使用Raydium");
            return Ok(false);
        },
        (Err(_), Err(_)) => {
            msg!("两个DEX的价格查询都失败");
            return Err(DexError::PriceQueryFailed.into());
        }
    };
    
    // 计算考虑滑点和费用后的实际价格
    let pump_price_with_slippage = calculate_price_with_slippage(pump_price, is_buy);
    let raydium_price_with_slippage = calculate_price_with_slippage(raydium_price, is_buy);
    
    // 比较价格，选择最优的DEX
    // 如果是买入操作，选择价格较低的DEX（获得更多代币）
    // 如果是卖出操作，选择价格较高的DEX（获得更多SOL）
    let use_pump = if is_buy {
        // 买入时比较能获得的代币数量
        let pump_tokens_out = amount_in.checked_mul(1_000_000_000).unwrap_or(0).checked_div(pump_price_with_slippage).unwrap_or(0);
        let raydium_tokens_out = amount_in.checked_mul(1_000_000_000).unwrap_or(0).checked_div(raydium_price_with_slippage).unwrap_or(0);
        
        msg!("买入比较 - Pump.fun: {} 代币/SOL, Raydium: {} 代币/SOL", 
            pump_tokens_out, raydium_tokens_out);
        
        pump_tokens_out >= raydium_tokens_out
    } else {
        // 卖出时比较能获得的SOL数量
        let pump_sol_out = amount_in.checked_mul(pump_price_with_slippage).unwrap_or(0).checked_div(1_000_000_000).unwrap_or(0);
        let raydium_sol_out = amount_in.checked_mul(raydium_price_with_slippage).unwrap_or(0).checked_div(1_000_000_000).unwrap_or(0);
        
        msg!("卖出比较 - Pump.fun: {} SOL/代币, Raydium: {} SOL/代币", 
            pump_sol_out, raydium_sol_out);
        
        pump_sol_out >= raydium_sol_out
    };
    
    // 记录价格查询结束时间
    let end_time = Clock::get()?.unix_timestamp;
    let execution_time = end_time - start_time;
    
    if use_pump {
        msg!("选择Pump.fun进行交易，提供更好的价格");
    } else {
        msg!("选择Raydium进行交易，提供更好的价格");
    }
    
    msg!("价格比较执行时间: {}ms", execution_time);
    
    // 发出价格比较事件
    emit!(PriceCompared {
        token_mint: ctx.accounts.token_mint.key(),
        amount_in,
        is_buy,
        pump_price,
        raydium_price,
        pump_price_with_slippage,
        raydium_price_with_slippage,
        use_pump,
        execution_time,
        slot: Clock::get()?.slot,
    });
    
    // 返回true表示使用Pump.fun，false表示使用Raydium
    Ok(use_pump)
}

// 计算考虑滑点和费用后的价格
fn calculate_price_with_slippage(price: u64, is_buy: bool) -> u64 {
    // 买入时增加价格（减少获得的代币），卖出时减少价格（减少获得的SOL）
    if is_buy {
        price.checked_mul(102).unwrap_or(price).checked_div(100).unwrap_or(price) // 增加2%
    } else {
        price.checked_mul(98).unwrap_or(price).checked_div(100).unwrap_or(price) // 减少2%
    }
}

// 智能路由交易
pub fn smart_trade(
    ctx: Context<SmartTradeContext>,
    token_mint: Pubkey,
    amount_in: u64,
    min_amount_out: u64,
    is_buy: bool,  // true表示买入，false表示卖出
) -> Result<()> {
    msg!("开始智能交易路由，代币: {}", token_mint);
    
    // 检查输入金额是否大于0
    require!(amount_in > 0, DexError::InvalidArgument);
    
    // 检查最小输出金额是否大于0
    require!(min_amount_out > 0, DexError::InvalidArgument);
    
    // 记录交易开始时间
    let start_time = Clock::get()?.unix_timestamp;
    
    // 首先检查代币的位置
    let on_pump = check_token_location(
        ctx.accounts.check_location_ctx.into(),
        token_mint,
    )?;
    
    // 根据代币位置选择正确的DEX
    if on_pump {
        msg!("代币在Pump.fun上，使用Pump.fun进行交易");
        
        if is_buy {
            pumpfun::buy_token(ctx.accounts.pump_trade_ctx.into(), amount_in, min_amount_out)?;
        } else {
            pumpfun::sell_token(ctx.accounts.pump_trade_ctx.into(), amount_in, min_amount_out)?;
        }
    } else {
        msg!("代币在Raydium上，使用Raydium进行交易");
        
        if is_buy {
            raydium::buy_token(ctx.accounts.raydium_trade_ctx.into(), amount_in, min_amount_out)?;
        } else {
            raydium::sell_token(ctx.accounts.raydium_trade_ctx.into(), amount_in, min_amount_out)?;
        }
    }
    
    // 记录交易结束时间
    let end_time = Clock::get()?.unix_timestamp;
    let execution_time = end_time - start_time;
    
    msg!("智能交易完成，执行时间: {}ms", execution_time);
    
    // 发出智能交易事件
    emit!(SmartTradeExecuted {
        user: ctx.accounts.pump_trade_ctx.user.key(),
        token_mint,
        amount_in,
        min_amount_out,
        is_buy,
        dex_used: if on_pump { "Pump.fun".to_string() } else { "Raydium".to_string() },
        execution_time,
        slot: Clock::get()?.slot,
    });
    
    Ok(())
}

// 批量交易
pub fn batch_trade(
    ctx: Context<BatchTradeContext>,
    instructions: Vec<TradeInstruction>,
) -> Result<()> {
    // 检查指令是否为空
    require!(!instructions.is_empty(), DexError::EmptyBatchInstructions);
    
    // 检查指令数量是否超过限制
    require!(
        instructions.len() <= MAX_BATCH_SIZE,
        DexError::TooManyBatchInstructions
    );
    
    msg!("开始批量交易，指令数量: {}", instructions.len());
    
    // 记录批量交易开始时间
    let start_time = Clock::get()?.unix_timestamp;
    
    // 执行每个交易指令
    for (i, instruction) in instructions.iter().enumerate() {
        msg!("执行批量交易指令 {}/{}", i + 1, instructions.len());
        
        // 根据DEX类型选择正确的交易方式
        match instruction.dex_type {
            DexType::Auto => {
                // 使用智能路由
                smart_trade(
                    ctx.accounts.smart_trade_ctx.into(),
                    instruction.token_mint,
                    instruction.amount_in,
                    instruction.min_amount_out,
                    instruction.is_buy,
                )?;
            },
            DexType::PumpFun => {
                // 直接使用Pump.fun
                if instruction.is_buy {
                    pumpfun::buy_token(
                        ctx.accounts.pump_trade_ctx.into(),
                        instruction.amount_in,
                        instruction.min_amount_out,
                    )?;
                } else {
                    pumpfun::sell_token(
                        ctx.accounts.pump_trade_ctx.into(),
                        instruction.amount_in,
                        instruction.min_amount_out,
                    )?;
                }
            },
            DexType::Raydium => {
                // 直接使用Raydium
                if instruction.is_buy {
                    raydium::buy_token(
                        ctx.accounts.raydium_trade_ctx.into(),
                        instruction.amount_in,
                        instruction.min_amount_out,
                    )?;
                } else {
                    raydium::sell_token(
                        ctx.accounts.raydium_trade_ctx.into(),
                        instruction.amount_in,
                        instruction.min_amount_out,
                    )?;
                }
            },
        }
    }
    
    // 记录批量交易结束时间
    let end_time = Clock::get()?.unix_timestamp;
    let execution_time = end_time - start_time;
    
    msg!("批量交易完成，执行时间: {}ms", execution_time);
    
    // 发出批量交易事件
    emit!(BatchTradeExecuted {
        user: ctx.accounts.pump_trade_ctx.user.key(),
        instruction_count: instructions.len() as u8,
        execution_time,
        slot: Clock::get()?.slot,
    });
    
    Ok(())
}

// 检查代币位置所需的账户结构
#[derive(Accounts)]
pub struct CheckTokenLocationContext<'info> {
    pub pump_check_ctx: pumpfun::CheckTokenLocation<'info>,
    pub raydium_check_ctx: raydium::CheckTokenLocation<'info>,
    
    // DEX账户
    #[account(
        seeds = [b"dex_account".as_ref()],
        bump
    )]
    pub dex_account: Account<'info, crate::DexAccount>,
}

// 获取最优价格所需的账户结构
#[derive(Accounts)]
pub struct GetBestPrice<'info> {
    pub pump_price_ctx: pumpfun::GetPriceContext<'info>,
    pub raydium_price_ctx: raydium::GetPriceContext<'info>,
    
    // DEX账户
    #[account(
        seeds = [b"dex_account".as_ref()],
        bump
    )]
    pub dex_account: Account<'info, crate::DexAccount>,
    
    // 代币Mint地址
    pub token_mint: Account<'info, anchor_spl::token::Mint>,
}

// 智能交易所需的账户结构
#[derive(Accounts)]
pub struct SmartTradeContext<'info> {
    pub pump_trade_ctx: pumpfun::TradeToken<'info>,
    pub raydium_trade_ctx: raydium::TradeTokenRaydium<'info>,
    pub check_location_ctx: CheckTokenLocationContext<'info>,
    pub get_best_price_ctx: GetBestPrice<'info>,
    
    // DEX账户
    #[account(
        mut,
        seeds = [b"dex_account".as_ref()],
        bump
    )]
    pub dex_account: Account<'info, crate::DexAccount>,
}

// 批量交易所需的账户结构
#[derive(Accounts)]
pub struct BatchTradeContext<'info> {
    pub pump_trade_ctx: pumpfun::TradeToken<'info>,
    pub raydium_trade_ctx: raydium::TradeTokenRaydium<'info>,
    pub smart_trade_ctx: SmartTradeContext<'info>,
    
    // DEX账户
    #[account(
        mut,
        seeds = [b"dex_account".as_ref()],
        bump
    )]
    pub dex_account: Account<'info, crate::DexAccount>,
}

// 价格比较事件
#[event]
pub struct PriceCompared {
    pub token_mint: Pubkey,
    pub amount_in: u64,
    pub is_buy: bool,
    pub pump_price: u64,
    pub raydium_price: u64,
    pub pump_price_with_slippage: u64,
    pub raydium_price_with_slippage: u64,
    pub use_pump: bool,
    pub execution_time: i64,
    pub slot: u64,
}

// 智能交易事件
#[event]
pub struct SmartTradeExecuted {
    pub user: Pubkey,
    pub token_mint: Pubkey,
    pub amount_in: u64,
    pub min_amount_out: u64,
    pub is_buy: bool,
    pub dex_used: String,
    pub execution_time: i64,
    pub slot: u64,
}

// 批量交易事件
#[event]
pub struct BatchTradeExecuted {
    pub user: Pubkey,
    pub instruction_count: u8,
    pub execution_time: i64,
    pub slot: u64,
} 