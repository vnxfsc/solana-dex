use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use solana_program::{
    program::invoke,
    pubkey::Pubkey,
    system_instruction,
    instruction::{Instruction, AccountMeta},
};
use std::str::FromStr;

// 更新为正确的Pump.fun程序ID
pub const PUMP_PROGRAM_ID: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";
// 更新为正确的Pump.fun费用账户
pub const PUMP_FEE_ACCOUNT: &str = "3XMrhbv989VxAMi3DErLV9eJht1pHppW5LbKxe9fkEFR";

// 获取Pump.fun程序ID
pub fn get_pump_program_id() -> Pubkey {
    Pubkey::from_str(PUMP_PROGRAM_ID).unwrap()
}

// 获取Pump.fun费用账户
pub fn get_pump_fee_account() -> Pubkey {
    Pubkey::from_str(PUMP_FEE_ACCOUNT).unwrap()
}

// 在Pump.fun上购买代币
pub fn buy_token(
    ctx: Context<TradeToken>,
    amount_out: u64,  // 期望获得的代币数量
    max_sol_cost: u64,  // 最大SOL花费（滑点控制）
) -> Result<()> {
    msg!("在Pump.fun上购买代币: 期望获得 {} 代币, 最大SOL花费: {}", amount_out, max_sol_cost);
    msg!("代币Mint地址: {}", ctx.accounts.token_mint.key());
    
    // 构建Pump.fun的交易数据 - 根据真实交易格式
    let mut data = vec![0u8; 16];
    // 将amount_out编码到data中 (期望获得的代币数量)
    data[0..8].copy_from_slice(&amount_out.to_le_bytes());
    // 将max_sol_cost编码到data中 (最大SOL花费)
    data[8..16].copy_from_slice(&max_sol_cost.to_le_bytes());
    
    // 获取Pump.fun程序ID
    let pump_program_id = get_pump_program_id();
    
    // 构建Pump.fun的交易指令 - 根据真实交易格式
    let swap_ix = Instruction {
        program_id: pump_program_id,
        accounts: vec![
            // 根据真实交易格式添加所需账户
            AccountMeta::new_readonly(ctx.accounts.global_state.key(), false), // Global state
            AccountMeta::new(get_pump_fee_account(), false), // Fee recipient
            AccountMeta::new_readonly(ctx.accounts.token_mint.key(), false), // Mint
            AccountMeta::new(ctx.accounts.bonding_curve.key(), false), // Bonding curve
            AccountMeta::new(ctx.accounts.bonding_curve_token_account.key(), false), // Associated bonding curve
            AccountMeta::new(ctx.accounts.user_token_account.key(), false), // Associated user
            AccountMeta::new(ctx.accounts.user.key(), true), // User (signer)
            AccountMeta::new_readonly(ctx.accounts.system_program.key(), false), // System program
            AccountMeta::new_readonly(ctx.accounts.token_program.key(), false), // Token program
            AccountMeta::new_readonly(ctx.accounts.rent.key(), false), // Rent
            AccountMeta::new_readonly(ctx.accounts.event_authority.key(), false), // Event authority
            AccountMeta::new_readonly(pump_program_id, false), // Program
        ],
        data: data,
    };
    
    // 执行Pump.fun的交易指令
    invoke(
        &swap_ix,
        &[
            ctx.accounts.global_state.to_account_info(),
            ctx.accounts.fee_recipient.to_account_info(),
            ctx.accounts.token_mint.to_account_info(),
            ctx.accounts.bonding_curve.to_account_info(),
            ctx.accounts.bonding_curve_token_account.to_account_info(),
            ctx.accounts.user_token_account.to_account_info(),
            ctx.accounts.user.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.rent.to_account_info(),
            ctx.accounts.event_authority.to_account_info(),
            ctx.accounts.pump_program.to_account_info(),
        ],
    )?;
    
    msg!("交易完成，获得代币");
    Ok(())
}

// 在Pump.fun上卖出代币
pub fn sell_token(
    ctx: Context<TradeToken>,
    amount_in: u64,  // 输入的代币数量
    min_sol_out: u64,  // 最小获得的SOL数量（滑点控制）
) -> Result<()> {
    msg!("在Pump.fun上卖出代币: {} 代币, 最小获得SOL数量: {}", amount_in, min_sol_out);
    msg!("代币Mint地址: {}", ctx.accounts.token_mint.key());
    
    // 1. 转移代币到Pump.fun的绑定曲线代币账户
    let transfer_cpi_accounts = Transfer {
        from: ctx.accounts.user_token_account.to_account_info(),
        to: ctx.accounts.bonding_curve_token_account.to_account_info(),
        authority: ctx.accounts.user.to_account_info(),
    };
    
    let cpi_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        transfer_cpi_accounts,
    );
    
    token::transfer(cpi_ctx, amount_in)?;
    
    // 2. 调用Pump.fun的交易指令
    // 构建Pump.fun的交易数据 - 根据真实交易格式
    let mut data = vec![0u8; 16];
    // 将amount_in编码到data中 (卖出的代币数量)
    data[0..8].copy_from_slice(&amount_in.to_le_bytes());
    // 将min_sol_out编码到data中 (最小获得的SOL数量)
    data[8..16].copy_from_slice(&min_sol_out.to_le_bytes());
    
    // 获取Pump.fun程序ID
    let pump_program_id = get_pump_program_id();
    
    // 构建Pump.fun的交易指令 - 根据真实交易格式
    let swap_ix = Instruction {
        program_id: pump_program_id,
        accounts: vec![
            // 根据真实交易格式添加所需账户
            AccountMeta::new_readonly(ctx.accounts.global_state.key(), false), // Global state
            AccountMeta::new(get_pump_fee_account(), false), // Fee recipient
            AccountMeta::new_readonly(ctx.accounts.token_mint.key(), false), // Mint
            AccountMeta::new(ctx.accounts.bonding_curve.key(), false), // Bonding curve
            AccountMeta::new(ctx.accounts.bonding_curve_token_account.key(), false), // Associated bonding curve
            AccountMeta::new(ctx.accounts.user_token_account.key(), false), // Associated user
            AccountMeta::new(ctx.accounts.user.key(), true), // User (signer)
            AccountMeta::new_readonly(ctx.accounts.system_program.key(), false), // System program
            AccountMeta::new_readonly(ctx.accounts.token_program.key(), false), // Token program
            AccountMeta::new_readonly(ctx.accounts.rent.key(), false), // Rent
            AccountMeta::new_readonly(ctx.accounts.event_authority.key(), false), // Event authority
            AccountMeta::new_readonly(pump_program_id, false), // Program
        ],
        data: data,
    };
    
    // 执行Pump.fun的交易指令
    invoke(
        &swap_ix,
        &[
            ctx.accounts.global_state.to_account_info(),
            ctx.accounts.fee_recipient.to_account_info(),
            ctx.accounts.token_mint.to_account_info(),
            ctx.accounts.bonding_curve.to_account_info(),
            ctx.accounts.bonding_curve_token_account.to_account_info(),
            ctx.accounts.user_token_account.to_account_info(),
            ctx.accounts.user.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.rent.to_account_info(),
            ctx.accounts.event_authority.to_account_info(),
            ctx.accounts.pump_program.to_account_info(),
        ],
    )?;
    
    msg!("交易完成，获得SOL");
    Ok(())
}

// 检查代币是否在Pump.fun上
pub fn is_token_on_pump(
    ctx: Context<CheckTokenLocation>,
    token_mint: Pubkey,
) -> Result<bool> {
    // 记录查询信息
    msg!("检查代币是否在Pump.fun上: {}", token_mint);
    
    // 计算绑定曲线账户的PDA地址
    let seeds = &[b"bonding-curve", token_mint.as_ref()];
    let (bonding_curve_pda, _) = Pubkey::find_program_address(seeds, &get_pump_program_id());
    
    msg!("计算的绑定曲线PDA地址: {}", bonding_curve_pda);
    
    // 尝试获取绑定曲线账户数据
    // 在实际实现中，我们需要检查账户是否存在
    // 这里我们通过检查账户数据大小来判断
    let bonding_curve_account_info = ctx.accounts.bonding_curve.to_account_info();
    
    // 检查账户是否存在且数据大小正确
    if bonding_curve_account_info.data_is_empty() {
        msg!("绑定曲线账户不存在，代币不在Pump.fun上");
        return Ok(false);
    }
    
    // 检查账户是否为BondingCurve类型
    // 在实际实现中，我们需要检查账户的discriminator
    // 这里我们假设账户已经被正确反序列化
    
    // 检查绑定曲线是否已完成
    if ctx.accounts.bonding_curve.complete {
        msg!("绑定曲线已完成，代币不再在Pump.fun上交易");
        return Ok(false);
    }
    
    // 检查绑定曲线是否有足够的流动性
    if ctx.accounts.bonding_curve.virtual_sol_reserves == 0 || ctx.accounts.bonding_curve.virtual_token_reserves == 0 {
        msg!("绑定曲线没有足够的流动性，代币不可交易");
        return Ok(false);
    }
    
    msg!("代币在Pump.fun上可用，虚拟SOL储备: {}, 虚拟代币储备: {}", 
        ctx.accounts.bonding_curve.virtual_sol_reserves,
        ctx.accounts.bonding_curve.virtual_token_reserves
    );
    Ok(true)
}

// 获取Pump.fun上的价格
pub fn get_price(
    ctx: Context<GetPriceContext>,
    amount_in: u64,
    is_buy: bool,  // true表示买入，false表示卖出
) -> Result<u64> {
    // 检查输入金额是否大于0
    if amount_in == 0 {
        msg!("错误: 输入金额不能为0");
        return Err(ProgramError::InvalidArgument.into());
    }
    
    // 记录查询信息
    msg!("查询Pump.fun上的价格: 输入金额 {}, 操作类型: {}", 
        amount_in, 
        if is_buy { "买入" } else { "卖出" }
    );
    msg!("代币Mint地址: {}", ctx.accounts.token_mint.key());
    
    // 从绑定曲线账户中获取价格计算所需的数据
    // 在实际实现中，我们需要从绑定曲线账户中读取这些数据
    // 这里我们假设已经从账户中读取了以下数据
    let virtual_token_reserves = ctx.accounts.bonding_curve.virtual_token_reserves;
    let virtual_sol_reserves = ctx.accounts.bonding_curve.virtual_sol_reserves;
    
    msg!("绑定曲线虚拟代币储备: {}", virtual_token_reserves);
    msg!("绑定曲线虚拟SOL储备: {}", virtual_sol_reserves);
    
    // 根据Pump.fun的恒定乘积公式计算价格
    // 公式: k = virtual_token_reserves * virtual_sol_reserves
    let k = virtual_token_reserves.checked_mul(virtual_sol_reserves).ok_or(ProgramError::ArithmeticOverflow)?;
    
    let price = if is_buy {
        // 买入时，计算获得的代币数量
        // 新的虚拟SOL储备 = virtual_sol_reserves + amount_in
        let new_virtual_sol_reserves = virtual_sol_reserves.checked_add(amount_in).ok_or(ProgramError::ArithmeticOverflow)?;
        
        // 新的虚拟代币储备 = k / new_virtual_sol_reserves
        let new_virtual_token_reserves = k.checked_div(new_virtual_sol_reserves).ok_or(ProgramError::ArithmeticOverflow)?;
        
        // 获得的代币数量 = virtual_token_reserves - new_virtual_token_reserves
        let tokens_out = virtual_token_reserves.checked_sub(new_virtual_token_reserves).ok_or(ProgramError::ArithmeticOverflow)?;
        
        // 计算每SOL可获得的代币数量
        let tokens_per_sol = tokens_out.checked_div(amount_in).unwrap_or(0);
        
        msg!("Pump.fun买入: 每SOL可获得{}代币", tokens_per_sol);
        tokens_per_sol
    } else {
        // 卖出时，计算获得的SOL数量
        // 新的虚拟代币储备 = virtual_token_reserves + amount_in
        let new_virtual_token_reserves = virtual_token_reserves.checked_add(amount_in).ok_or(ProgramError::ArithmeticOverflow)?;
        
        // 新的虚拟SOL储备 = k / new_virtual_token_reserves
        let new_virtual_sol_reserves = k.checked_div(new_virtual_token_reserves).ok_or(ProgramError::ArithmeticOverflow)?;
        
        // 获得的SOL数量 = virtual_sol_reserves - new_virtual_sol_reserves
        let sol_out = virtual_sol_reserves.checked_sub(new_virtual_sol_reserves).ok_or(ProgramError::ArithmeticOverflow)?;
        
        // 计算每代币可获得的SOL数量（以lamports为单位）
        let lamports_per_token = sol_out.checked_mul(1_000_000_000).unwrap_or(0).checked_div(amount_in).unwrap_or(0);
        
        msg!("Pump.fun卖出: 每代币可获得{}lamports", lamports_per_token);
        lamports_per_token
    };
    
    msg!("Pump.fun上的最终价格: {}", price);
    Ok(price)
}

// Pump.fun交易所需的账户结构
#[derive(Accounts)]
pub struct TradeToken<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    
    // 代币Mint地址
    pub token_mint: Account<'info, token::Mint>,
    
    // Pump.fun全局状态账户
    #[account(
        seeds = [b"global"],
        bump,
    )]
    pub global_state: Account<'info, Global>,
    
    // Pump.fun费用接收账户
    #[account(
        mut,
        constraint = fee_recipient.key() == global_state.fee_recipient
    )]
    pub fee_recipient: AccountInfo<'info>,
    
    // Pump.fun绑定曲线账户
    #[account(
        mut,
        seeds = [b"bonding-curve", token_mint.key().as_ref()],
        bump,
    )]
    pub bonding_curve: Account<'info, BondingCurve>,
    
    // Pump.fun绑定曲线代币账户
    #[account(mut)]
    pub bonding_curve_token_account: Account<'info, TokenAccount>,
    
    // 用户代币账户
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,
    
    // 系统程序
    pub system_program: Program<'info, System>,
    
    // 代币程序
    pub token_program: Program<'info, Token>,
    
    // 租金程序
    pub rent: AccountInfo<'info>,
    
    // 事件权限账户
    #[account(
        constraint = event_authority.key() == Pubkey::from_str("Ce6TQqeHC9p8KetsN6JsjHK7UTZk7nasjjnr7XxXp9F1").unwrap()
    )]
    pub event_authority: AccountInfo<'info>,
    
    // Pump.fun程序
    #[account(
        constraint = pump_program.key() == get_pump_program_id()
    )]
    pub pump_program: AccountInfo<'info>,
}

// 检查代币位置所需的账户结构
#[derive(Accounts)]
pub struct CheckTokenLocation<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    
    // Pump.fun全局状态账户
    pub global_state: Account<'info, Global>,
    
    // Pump.fun绑定曲线账户
    // 注意：这里我们不使用seeds约束，因为我们需要动态检查不同的代币
    pub bonding_curve: Account<'info, BondingCurve>,
    
    // 系统程序
    pub system_program: Program<'info, System>,
}

// 获取价格所需的账户结构
#[derive(Accounts)]
pub struct GetPriceContext<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    
    // 代币Mint地址
    pub token_mint: Account<'info, token::Mint>,
    
    // Pump.fun全局状态账户
    pub global_state: Account<'info, Global>,
    
    // Pump.fun绑定曲线账户
    #[account(
        seeds = [b"bonding-curve", token_mint.key().as_ref()],
        bump,
    )]
    pub bonding_curve: Account<'info, BondingCurve>,
    
    // Pump.fun绑定曲线代币账户
    pub bonding_curve_token_account: Account<'info, TokenAccount>,
    
    // 系统程序
    pub system_program: Program<'info, System>,
    
    // 代币程序
    pub token_program: Program<'info, Token>,
}

// 定义Pump.fun的Global账户结构
#[account]
pub struct Global {
    pub initialized: bool,
    pub authority: Pubkey,
    pub fee_recipient: Pubkey,
    pub initial_virtual_token_reserves: u64,
    pub initial_virtual_sol_reserves: u64,
    pub initial_real_token_reserves: u64,
    pub token_total_supply: u64,
    pub fee_basis_points: u64,
}

// 定义Pump.fun的BondingCurve账户结构
#[account]
pub struct BondingCurve {
    pub virtual_token_reserves: u64,
    pub virtual_sol_reserves: u64,
    pub real_token_reserves: u64,
    pub real_sol_reserves: u64,
    pub token_total_supply: u64,
    pub complete: bool,
} 