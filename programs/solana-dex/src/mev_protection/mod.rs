use anchor_lang::prelude::*;
use solana_program::{
    keccak::hash,
    pubkey::Pubkey,
};
use crate::{
    DexError, DexType, MIN_COMMITMENT_DELAY, MAX_COMMITMENT_DELAY, COMMITMENT_EXPIRY,
    router, raydium, pumpfun
};

// 交易承诺账户
#[account]
pub struct TradeCommitment {
    // 用户公钥
    pub user: Pubkey,
    // 承诺哈希
    pub commitment_hash: [u8; 32],
    // 最早执行区块
    pub min_slot: u64,
    // 过期区块
    pub expiry_slot: u64,
    // 是否已执行
    pub executed: bool,
    // 创建时间戳
    pub created_at: i64,
    // 创建区块
    pub created_slot: u64,
}

// 创建交易承诺的上下文
#[derive(Accounts)]
pub struct CreateCommitment<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    
    // DEX账户
    #[account(
        seeds = [b"dex_account".as_ref()],
        bump
    )]
    pub dex_account: Account<'info, crate::DexAccount>,
    
    // 交易承诺账户
    #[account(
        init,
        payer = user,
        space = 8 + 32 + 32 + 8 + 8 + 1 + 8 + 8, // 8字节discriminator + 32字节pubkey + 32字节哈希 + 8字节min_slot + 8字节expiry_slot + 1字节executed + 8字节timestamp + 8字节slot
        seeds = [b"commitment", user.key().as_ref()],
        bump
    )]
    pub commitment: Account<'info, TradeCommitment>,
    
    pub system_program: Program<'info, System>,
}

// 执行承诺交易的上下文
#[derive(Accounts)]
pub struct ExecuteCommitment<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    
    // DEX账户
    #[account(
        mut,
        seeds = [b"dex_account".as_ref()],
        bump
    )]
    pub dex_account: Account<'info, crate::DexAccount>,
    
    // 交易承诺账户
    #[account(
        mut,
        seeds = [b"commitment", user.key().as_ref()],
        bump,
        constraint = commitment.user == user.key(),
        constraint = !commitment.executed @ DexError::CommitmentAlreadyExecuted
    )]
    pub commitment: Account<'info, TradeCommitment>,
    
    // 智能交易所需的账户
    pub smart_trade_ctx: router::SmartTradeContext<'info>,
    
    // 系统程序
    pub system_program: Program<'info, System>,
}

// 批量执行承诺交易的上下文
#[derive(Accounts)]
pub struct BatchExecuteCommitment<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    
    // DEX账户
    #[account(
        mut,
        seeds = [b"dex_account".as_ref()],
        bump
    )]
    pub dex_account: Account<'info, crate::DexAccount>,
    
    // 系统程序
    pub system_program: Program<'info, System>,
    
    // 智能交易所需的账户
    pub smart_trade_ctx: router::SmartTradeContext<'info>,
}

// 批量执行承诺交易的参数
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CommitmentExecutionParams {
    // 承诺账户地址
    pub commitment_address: Pubkey,
    // 代币铸币厂
    pub token_mint: Pubkey,
    // 输入金额
    pub amount_in: u64,
    // 最小输出金额
    pub min_amount_out: u64,
    // 是否为购买操作
    pub is_buy: bool,
    // DEX类型
    pub dex_type: DexType,
    // 随机数
    pub nonce: [u8; 32],
}

// 检查承诺过期的上下文
#[derive(Accounts)]
pub struct CheckExpiredCommitment<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    
    // DEX账户
    #[account(
        mut,
        seeds = [b"dex_account".as_ref()],
        bump,
        constraint = dex_account.authority == authority.key()
    )]
    pub dex_account: Account<'info, crate::DexAccount>,
    
    // 交易承诺账户
    #[account(
        mut,
        constraint = !commitment.executed @ DexError::CommitmentAlreadyExecuted
    )]
    pub commitment: Account<'info, TradeCommitment>,
}

// 检查承诺是否过期
pub fn check_expired_commitment(
    ctx: Context<CheckExpiredCommitment>,
) -> Result<()> {
    // 获取当前区块
    let clock = Clock::get()?;
    let current_slot = clock.slot;
    
    // 检查承诺是否已过期
    require!(
        current_slot > ctx.accounts.commitment.expiry_slot,
        DexError::CommitmentNotExpired
    );
    
    // 标记承诺为已执行（防止重复检查）
    ctx.accounts.commitment.executed = true;
    
    // 更新统计数据
    let dex_account = &mut ctx.accounts.dex_account;
    dex_account.expired_commitments = dex_account.expired_commitments.checked_add(1).unwrap_or(dex_account.expired_commitments);
    
    // 记录承诺过期信息
    msg!("承诺已过期: 用户 {}, 创建区块 {}, 过期区块 {}, 当前区块 {}", 
        ctx.accounts.commitment.user,
        ctx.accounts.commitment.created_slot,
        ctx.accounts.commitment.expiry_slot,
        current_slot
    );
    
    // 发出承诺过期事件
    emit!(CommitmentExpired {
        user: ctx.accounts.commitment.user,
        commitment_hash: ctx.accounts.commitment.commitment_hash,
        created_slot: ctx.accounts.commitment.created_slot,
        expiry_slot: ctx.accounts.commitment.expiry_slot,
        checked_at: clock.unix_timestamp,
        checked_slot: current_slot,
    });
    
    Ok(())
}

// 创建交易承诺
pub fn create_commitment(
    ctx: Context<CreateCommitment>,
    commitment_hash: [u8; 32],
    min_slot_delay: u64,
) -> Result<()> {
    // 验证延迟区块数是否在允许范围内
    require!(
        min_slot_delay >= MIN_COMMITMENT_DELAY && min_slot_delay <= MAX_COMMITMENT_DELAY,
        DexError::InvalidSlotDelay
    );
    
    // 获取当前区块和时间
    let clock = Clock::get()?;
    let current_slot = clock.slot;
    let current_timestamp = clock.unix_timestamp;
    
    // 计算最早执行区块和过期区块
    let min_slot = current_slot.checked_add(min_slot_delay).unwrap_or(current_slot);
    let expiry_slot = current_slot.checked_add(COMMITMENT_EXPIRY).unwrap_or(current_slot);
    
    // 初始化承诺账户
    let commitment = &mut ctx.accounts.commitment;
    commitment.user = ctx.accounts.user.key();
    commitment.commitment_hash = commitment_hash;
    commitment.min_slot = min_slot;
    commitment.expiry_slot = expiry_slot;
    commitment.executed = false;
    commitment.created_at = current_timestamp;
    commitment.created_slot = current_slot;
    
    // 更新统计数据
    let dex_account = &mut ctx.accounts.dex_account;
    dex_account.total_commitments = dex_account.total_commitments.checked_add(1).unwrap_or(dex_account.total_commitments);
    
    // 记录承诺创建信息
    msg!("交易承诺已创建: 用户 {}, 最早执行区块 {}, 过期区块 {}", 
        ctx.accounts.user.key(), min_slot, expiry_slot);
    
    // 发出承诺创建事件
    emit!(CommitmentCreated {
        user: ctx.accounts.user.key(),
        commitment_hash,
        min_slot,
        expiry_slot,
        created_at: current_timestamp,
        created_slot: current_slot,
    });
    
    Ok(())
}

// 执行承诺交易
pub fn execute_commitment(
    ctx: Context<ExecuteCommitment>,
    token_mint: Pubkey,
    amount_in: u64,
    min_amount_out: u64,
    is_buy: bool,
    dex_type: DexType,
    nonce: [u8; 32],
) -> Result<()> {
    // 获取当前区块
    let clock = Clock::get()?;
    let current_slot = clock.slot;
    
    // 检查承诺是否已成熟
    require!(
        current_slot >= ctx.accounts.commitment.min_slot,
        DexError::CommitmentNotMatured
    );
    
    // 检查承诺是否已过期
    require!(
        current_slot <= ctx.accounts.commitment.expiry_slot,
        DexError::CommitmentExpired
    );
    
    // 计算承诺哈希
    let calculated_hash = calculate_commitment_hash(
        token_mint,
        amount_in,
        min_amount_out,
        is_buy,
        dex_type.clone(),
        nonce,
    );
    
    // 验证承诺哈希
    require!(
        calculated_hash == ctx.accounts.commitment.commitment_hash,
        DexError::CommitmentHashMismatch
    );
    
    // 标记承诺为已执行
    ctx.accounts.commitment.executed = true;
    
    // 更新统计数据
    let dex_account = &mut ctx.accounts.dex_account;
    dex_account.executed_commitments = dex_account.executed_commitments.checked_add(1).unwrap_or(dex_account.executed_commitments);
    
    // 记录承诺执行信息
    msg!("执行承诺交易: 用户 {}, 代币 {}, 金额 {}, 最小输出 {}, 操作 {}", 
        ctx.accounts.user.key(),
        token_mint,
        amount_in,
        min_amount_out,
        if is_buy { "买入" } else { "卖出" }
    );
    
    // 根据DEX类型执行交易
    match dex_type {
        DexType::Auto => {
            // 使用智能路由
            router::smart_trade(
                ctx.accounts.smart_trade_ctx.into(),
                token_mint,
                amount_in,
                min_amount_out,
                is_buy,
            )?;
        },
        DexType::PumpFun => {
            // 直接使用Pump.fun
            if is_buy {
                pumpfun::buy_token(
                    ctx.accounts.smart_trade_ctx.pump_trade_ctx.into(),
                    amount_in,
                    min_amount_out,
                )?;
            } else {
                pumpfun::sell_token(
                    ctx.accounts.smart_trade_ctx.pump_trade_ctx.into(),
                    amount_in,
                    min_amount_out,
                )?;
            }
        },
        DexType::Raydium => {
            // 直接使用Raydium
            if is_buy {
                raydium::buy_token(
                    ctx.accounts.smart_trade_ctx.raydium_trade_ctx.into(),
                    amount_in,
                    min_amount_out,
                )?;
            } else {
                raydium::sell_token(
                    ctx.accounts.smart_trade_ctx.raydium_trade_ctx.into(),
                    amount_in,
                    min_amount_out,
                )?;
            }
        },
    }
    
    // 发出承诺执行事件
    emit!(CommitmentExecuted {
        user: ctx.accounts.user.key(),
        token_mint,
        amount_in,
        min_amount_out,
        is_buy,
        dex_type: format!("{:?}", dex_type),
        executed_at: clock.unix_timestamp,
        executed_slot: current_slot,
    });
    
    Ok(())
}

// 批量执行承诺交易
pub fn batch_execute_commitments(
    ctx: Context<BatchExecuteCommitment>,
    params: Vec<CommitmentExecutionParams>,
) -> Result<()> {
    // 验证参数数量
    require!(!params.is_empty(), DexError::EmptyBatchInstructions);
    require!(params.len() <= 5, DexError::TooManyBatchInstructions); // 限制最多5个承诺
    
    // 获取当前区块和时间
    let clock = Clock::get()?;
    let current_slot = clock.slot;
    let current_timestamp = clock.unix_timestamp;
    
    // 记录批量执行开始
    msg!("开始批量执行 {} 个承诺交易", params.len());
    
    // 执行每个承诺
    for (i, param) in params.iter().enumerate() {
        // 获取承诺账户
        let commitment_info = ctx.remaining_accounts.get(i)
            .ok_or(DexError::InvalidArgument)?;
        
        // 验证承诺账户是否属于用户
        let commitment_data = commitment_info.try_borrow_data()?;
        let mut account_data: &[u8] = &commitment_data;
        let commitment = TradeCommitment::try_deserialize(&mut account_data)?;
        
        // 验证承诺所有者
        require!(commitment.user == ctx.accounts.user.key(), DexError::InvalidArgument);
        
        // 检查承诺是否已执行
        require!(!commitment.executed, DexError::CommitmentAlreadyExecuted);
        
        // 检查承诺是否已成熟
        require!(current_slot >= commitment.min_slot, DexError::CommitmentNotMatured);
        
        // 检查承诺是否已过期
        require!(current_slot <= commitment.expiry_slot, DexError::CommitmentExpired);
        
        // 计算承诺哈希
        let calculated_hash = calculate_commitment_hash(
            param.token_mint,
            param.amount_in,
            param.min_amount_out,
            param.is_buy,
            param.dex_type.clone(),
            param.nonce,
        );
        
        // 验证承诺哈希
        require!(calculated_hash == commitment.commitment_hash, DexError::CommitmentHashMismatch);
        
        // 标记承诺为已执行 (需要通过CPI调用)
        let mut commitment_account_data = commitment_info.try_borrow_mut_data()?;
        let mut account_data: &mut [u8] = &mut commitment_account_data;
        let mut commitment = TradeCommitment::try_deserialize(&mut account_data)?;
        commitment.executed = true;
        let mut writer = std::io::Cursor::new(commitment_account_data.as_mut());
        commitment.try_serialize(&mut writer)?;
        
        // 记录承诺执行信息
        msg!("执行承诺交易 #{}: 用户 {}, 代币 {}, 金额 {}, 最小输出 {}, 操作 {}", 
            i + 1,
            ctx.accounts.user.key(),
            param.token_mint,
            param.amount_in,
            param.min_amount_out,
            if param.is_buy { "买入" } else { "卖出" }
        );
        
        // 根据DEX类型执行交易
        match param.dex_type {
            DexType::Auto => {
                // 使用智能路由
                router::smart_trade(
                    ctx.accounts.smart_trade_ctx.into(),
                    param.token_mint,
                    param.amount_in,
                    param.min_amount_out,
                    param.is_buy,
                )?;
            },
            DexType::PumpFun => {
                // 直接使用Pump.fun
                if param.is_buy {
                    pumpfun::buy_token(
                        ctx.accounts.smart_trade_ctx.pump_trade_ctx.into(),
                        param.amount_in,
                        param.min_amount_out,
                    )?;
                } else {
                    pumpfun::sell_token(
                        ctx.accounts.smart_trade_ctx.pump_trade_ctx.into(),
                        param.amount_in,
                        param.min_amount_out,
                    )?;
                }
            },
            DexType::Raydium => {
                // 直接使用Raydium
                if param.is_buy {
                    raydium::buy_token(
                        ctx.accounts.smart_trade_ctx.raydium_trade_ctx.into(),
                        param.amount_in,
                        param.min_amount_out,
                    )?;
                } else {
                    raydium::sell_token(
                        ctx.accounts.smart_trade_ctx.raydium_trade_ctx.into(),
                        param.amount_in,
                        param.min_amount_out,
                    )?;
                }
            },
        }
        
        // 发出承诺执行事件
        emit!(CommitmentExecuted {
            user: ctx.accounts.user.key(),
            token_mint: param.token_mint,
            amount_in: param.amount_in,
            min_amount_out: param.min_amount_out,
            is_buy: param.is_buy,
            dex_type: format!("{:?}", param.dex_type),
            executed_at: current_timestamp,
            executed_slot: current_slot,
        });
    }
    
    // 记录批量执行完成
    msg!("批量执行 {} 个承诺交易完成", params.len());
    
    Ok(())
}

// 计算承诺哈希
fn calculate_commitment_hash(
    token_mint: Pubkey,
    amount_in: u64,
    min_amount_out: u64,
    is_buy: bool,
    dex_type: DexType,
    nonce: [u8; 32],
) -> [u8; 32] {
    // 将交易参数序列化为字节
    let mut data = Vec::new();
    data.extend_from_slice(&token_mint.to_bytes());
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&min_amount_out.to_le_bytes());
    data.push(if is_buy { 1 } else { 0 });
    data.push(match dex_type {
        DexType::Auto => 0,
        DexType::PumpFun => 1,
        DexType::Raydium => 2,
    });
    data.extend_from_slice(&nonce);
    
    // 计算哈希
    let hash_result = hash(&data);
    hash_result.0
}

// 承诺创建事件
#[event]
pub struct CommitmentCreated {
    pub user: Pubkey,
    pub commitment_hash: [u8; 32],
    pub min_slot: u64,
    pub expiry_slot: u64,
    pub created_at: i64,
    pub created_slot: u64,
}

// 承诺执行事件
#[event]
pub struct CommitmentExecuted {
    pub user: Pubkey,
    pub token_mint: Pubkey,
    pub amount_in: u64,
    pub min_amount_out: u64,
    pub is_buy: bool,
    pub dex_type: String,
    pub executed_at: i64,
    pub executed_slot: u64,
}

// 承诺过期事件
#[event]
pub struct CommitmentExpired {
    pub user: Pubkey,
    pub commitment_hash: [u8; 32],
    pub created_slot: u64,
    pub expiry_slot: u64,
    pub checked_at: i64,
    pub checked_slot: u64,
}

// 查询承诺统计的上下文
#[derive(Accounts)]
pub struct GetCommitmentStats<'info> {
    // DEX账户
    #[account(
        seeds = [b"dex_account".as_ref()],
        bump
    )]
    pub dex_account: Account<'info, crate::DexAccount>,
}

// 承诺统计结构
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CommitmentStats {
    pub total_commitments: u64,
    pub executed_commitments: u64,
    pub expired_commitments: u64,
    pub pending_commitments: u64,
}

// 查询承诺统计
pub fn get_commitment_stats(
    ctx: Context<GetCommitmentStats>,
) -> Result<CommitmentStats> {
    let dex_account = &ctx.accounts.dex_account;
    
    // 计算待处理的承诺数量
    let pending_commitments = dex_account.total_commitments
        .checked_sub(dex_account.executed_commitments)
        .and_then(|result| result.checked_sub(dex_account.expired_commitments))
        .unwrap_or(0);
    
    // 创建统计结构
    let stats = CommitmentStats {
        total_commitments: dex_account.total_commitments,
        executed_commitments: dex_account.executed_commitments,
        expired_commitments: dex_account.expired_commitments,
        pending_commitments,
    };
    
    // 记录统计信息
    msg!("承诺统计: 总数 {}, 已执行 {}, 已过期 {}, 待处理 {}", 
        stats.total_commitments,
        stats.executed_commitments,
        stats.expired_commitments,
        stats.pending_commitments
    );
    
    Ok(stats)
} 