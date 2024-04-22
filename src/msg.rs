use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Binary, CustomMsg, Uint256};

#[cw_serde]
pub struct InstantiateMsg {
    pub retry_delay: u64,
    pub job_id: String,
    pub creator: String,
    pub signers: Vec<String>,
}

#[cw_serde]
pub struct CreateOrderParamsAddresses {
    pub receiver: String,
    pub callback_contract: String,
    pub ui_fee_receiver: String,
    pub market: String,
    pub initial_collateral_token: String,
    pub swap_path: Vec<String>,
}

#[cw_serde]
pub struct CreateOrderParamsNumbers {
    pub size_delta_usd: Uint256,
    pub initial_collateral_delta_amount: Uint256,
    pub trigger_price: Uint256,
    pub acceptable_price: Uint256,
    pub execution_fee: Uint256,
    pub callback_gas_limit: Uint256,
    pub min_output_amount: Uint256,
}

#[cw_serde]
pub struct CreateOrderParams {
    pub addresses: CreateOrderParamsAddresses,
    pub numbers: CreateOrderParamsNumbers,
    pub order_type: Uint256,
    pub decrease_position_swap_type: Uint256,
    pub is_long: bool,
    pub should_unwrap_native_token: bool,
    pub referral_code: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    Withdraw {
        bot: String,
        amount0: Uint256,
        amount1: Uint256,
        order_params: CreateOrderParams,
        swap_min_amount: Uint256,
    },
    SetPaloma {},
    UpdateCompass {
        new_compass: String,
    },
}

#[cw_serde]
#[derive(Eq)]
pub struct Metadata {
    pub creator: String,
    pub signers: Vec<String>,
}

/// Message struct for cross-chain calls.
#[cw_serde]
pub struct PalomaMsg {
    /// The ID of the paloma scheduled job to run.
    pub job_id: String,
    /// The payload, ABI encoded for the target chain.
    pub payload: Binary,
    /// Metadata
    pub metadata: Metadata,
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(GetJobIdResponse)]
    GetJobId {},
}

#[cw_serde]
pub struct GetJobIdResponse {
    pub job_id: String,
}

impl CustomMsg for PalomaMsg {}
