use crate::ContractError::{AllPending, Unauthorized};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint256,
};
use ethabi::{Address, Contract, Function, Hash, Param, ParamType, StateMutability, Token, Uint};
use std::collections::BTreeMap;
use std::str::FromStr;
// use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{CreateOrderParams, ExecuteMsg, InstantiateMsg, Metadata, PalomaMsg, QueryMsg};
use crate::state::{State, STATE, WITHDRAW_TIMESTAMP};

/*
// version info for migration info
const CONTRACT_NAME: &str = "crates.io:short-funding-bot-cw";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
*/

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = State {
        retry_delay: msg.retry_delay,
        job_id: msg.job_id.clone(),
        owner: info.sender.clone(),
        metadata: Metadata {
            creator: msg.creator,
            signers: msg.signers,
        },
    };
    STATE.save(deps.storage, &state)?;
    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender)
        .add_attribute("job_id", msg.job_id))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<PalomaMsg>, ContractError> {
    match msg {
        ExecuteMsg::Withdraw {
            bot,
            amount0,
            amount1,
            order_params,
            swap_min_amount,
        } => withdraw(
            deps,
            env,
            info,
            bot,
            amount0,
            amount1,
            order_params,
            swap_min_amount,
        ),
        ExecuteMsg::SetPaloma {} => set_paloma(deps, info),
        ExecuteMsg::UpdateCompass { new_compass } => update_compass(deps, info, new_compass),
    }
}

pub fn withdraw(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    bot: String,
    amount0: Uint256,
    amount1: Uint256,
    order_params: CreateOrderParams,
    swap_min_amount: Uint256,
) -> Result<Response<PalomaMsg>, ContractError> {
    let state = STATE.load(deps.storage)?;
    if state.owner != info.sender {
        return Err(Unauthorized {});
    }
    #[allow(deprecated)]
    let contract: Contract = Contract {
        constructor: None,
        functions: BTreeMap::from_iter(vec![(
            "withdraw".to_string(),
            vec![Function {
                name: "withdraw".to_string(),
                inputs: vec![
                    Param {
                        name: "bot".to_string(),
                        kind: ParamType::Address,
                        internal_type: None,
                    },
                    Param {
                        name: "amount0".to_string(),
                        kind: ParamType::Uint(256),
                        internal_type: None,
                    },
                    Param {
                        name: "amount1".to_string(),
                        kind: ParamType::Uint(256),
                        internal_type: None,
                    },
                    Param {
                        name: "order_params".to_string(),
                        kind: ParamType::Tuple(vec![
                            ParamType::Tuple(vec![
                                ParamType::Address,
                                ParamType::Address,
                                ParamType::Address,
                                ParamType::Address,
                                ParamType::Address,
                                ParamType::Array(Box::new(ParamType::Address)),
                            ]),
                            ParamType::Tuple(vec![
                                ParamType::Uint(256),
                                ParamType::Uint(256),
                                ParamType::Uint(256),
                                ParamType::Uint(256),
                                ParamType::Uint(256),
                                ParamType::Uint(256),
                                ParamType::Uint(256),
                            ]),
                            ParamType::Uint(256),
                            ParamType::Bool,
                            ParamType::Bool,
                            ParamType::FixedBytes(32),
                        ]),
                        internal_type: None,
                    },
                    Param {
                        name: "swap_min_amount".to_string(),
                        kind: ParamType::Uint(256),
                        internal_type: None,
                    }
                ],
                outputs: Vec::new(),
                constant: None,
                state_mutability: StateMutability::NonPayable,
            }],
        )]),
        events: BTreeMap::new(),
        errors: BTreeMap::new(),
        receive: false,
        fallback: false,
    };
    let mut tokens: Vec<Token> = vec![];
    let retry_delay: u64 = state.retry_delay;
    if let Some(timestamp) = WITHDRAW_TIMESTAMP.may_load(deps.storage, bot.clone())? {
        if timestamp.plus_seconds(retry_delay).lt(&env.block.time) {
            tokens.push(Token::Address(Address::from_str(bot.as_str()).unwrap()));
            tokens.push(Token::Uint(Uint::from_big_endian(&amount0.to_be_bytes())));
            tokens.push(Token::Uint(Uint::from_big_endian(&amount1.to_be_bytes())));
            tokens.push(Token::Tuple(vec![
                Token::Tuple(vec![
                    Token::Address(
                        Address::from_str(order_params.addresses.receiver.as_str()).unwrap(),
                    ),
                    Token::Address(
                        Address::from_str(order_params.addresses.callback_contract.as_str())
                            .unwrap(),
                    ),
                    Token::Address(
                        Address::from_str(order_params.addresses.ui_fee_receiver.as_str()).unwrap(),
                    ),
                    Token::Address(
                        Address::from_str(order_params.addresses.market.as_str()).unwrap(),
                    ),
                    Token::Address(
                        Address::from_str(order_params.addresses.initial_collateral_token.as_str())
                            .unwrap(),
                    ),
                    Token::Array(
                        order_params
                            .addresses
                            .swap_path
                            .iter()
                            .map(|e| Token::Address(Address::from_str(e.as_str()).unwrap()))
                            .collect(),
                    ),
                ]),
                Token::Tuple(vec![
                    Token::Uint(Uint::from_big_endian(
                        &order_params.numbers.size_delta_usd.to_be_bytes(),
                    )),
                    Token::Uint(Uint::from_big_endian(
                        &order_params
                            .numbers
                            .initial_collateral_delta_amount
                            .to_be_bytes(),
                    )),
                    Token::Uint(Uint::from_big_endian(
                        &order_params.numbers.trigger_price.to_be_bytes(),
                    )),
                    Token::Uint(Uint::from_big_endian(
                        &order_params.numbers.acceptable_price.to_be_bytes(),
                    )),
                    Token::Uint(Uint::from_big_endian(
                        &order_params.numbers.execution_fee.to_be_bytes(),
                    )),
                    Token::Uint(Uint::from_big_endian(
                        &order_params.numbers.callback_gas_limit.to_be_bytes(),
                    )),
                    Token::Uint(Uint::from_big_endian(
                        &order_params.numbers.min_output_amount.to_be_bytes(),
                    )),
                ]),
                Token::Uint(Uint::from_big_endian(
                    &order_params.order_type.to_be_bytes(),
                )),
                Token::Uint(Uint::from_big_endian(
                    &order_params.decrease_position_swap_type.to_be_bytes(),
                )),
                Token::Bool(order_params.is_long),
                Token::Bool(order_params.should_unwrap_native_token),
                Token::FixedBytes(
                    Hash::from_str(order_params.referral_code.as_str())
                        .unwrap()
                        .0
                        .to_vec(),
                ),
            ]));
            tokens.push(Token::Uint(Uint::from_big_endian(
                &swap_min_amount.to_be_bytes(),
            )));
            WITHDRAW_TIMESTAMP.save(deps.storage, bot, &env.block.time)?;
        }
    } else {
        tokens.push(Token::Address(Address::from_str(bot.as_str()).unwrap()));
        tokens.push(Token::Uint(Uint::from_big_endian(&amount0.to_be_bytes())));
        tokens.push(Token::Uint(Uint::from_big_endian(&amount1.to_be_bytes())));
        tokens.push(Token::Tuple(vec![
            Token::Tuple(vec![
                Token::Address(
                    Address::from_str(order_params.addresses.receiver.as_str()).unwrap(),
                ),
                Token::Address(
                    Address::from_str(order_params.addresses.callback_contract.as_str()).unwrap(),
                ),
                Token::Address(
                    Address::from_str(order_params.addresses.ui_fee_receiver.as_str()).unwrap(),
                ),
                Token::Address(Address::from_str(order_params.addresses.market.as_str()).unwrap()),
                Token::Address(
                    Address::from_str(order_params.addresses.initial_collateral_token.as_str())
                        .unwrap(),
                ),
                Token::Array(
                    order_params
                        .addresses
                        .swap_path
                        .iter()
                        .map(|e| Token::Address(Address::from_str(e.as_str()).unwrap()))
                        .collect(),
                ),
            ]),
            Token::Tuple(vec![
                Token::Uint(Uint::from_big_endian(
                    &order_params.numbers.size_delta_usd.to_be_bytes(),
                )),
                Token::Uint(Uint::from_big_endian(
                    &order_params
                        .numbers
                        .initial_collateral_delta_amount
                        .to_be_bytes(),
                )),
                Token::Uint(Uint::from_big_endian(
                    &order_params.numbers.trigger_price.to_be_bytes(),
                )),
                Token::Uint(Uint::from_big_endian(
                    &order_params.numbers.acceptable_price.to_be_bytes(),
                )),
                Token::Uint(Uint::from_big_endian(
                    &order_params.numbers.execution_fee.to_be_bytes(),
                )),
                Token::Uint(Uint::from_big_endian(
                    &order_params.numbers.callback_gas_limit.to_be_bytes(),
                )),
                Token::Uint(Uint::from_big_endian(
                    &order_params.numbers.min_output_amount.to_be_bytes(),
                )),
            ]),
            Token::Uint(Uint::from_big_endian(
                &order_params.order_type.to_be_bytes(),
            )),
            Token::Uint(Uint::from_big_endian(
                &order_params.decrease_position_swap_type.to_be_bytes(),
            )),
            Token::Bool(order_params.is_long),
            Token::Bool(order_params.should_unwrap_native_token),
            Token::FixedBytes(
                Hash::from_str(order_params.referral_code.as_str())
                    .unwrap()
                    .0
                    .to_vec(),
            ),
        ]));
        tokens.push(Token::Uint(Uint::from_big_endian(
            &swap_min_amount.to_be_bytes(),
        )));
        WITHDRAW_TIMESTAMP.save(deps.storage, bot, &env.block.time)?;
    }
    if tokens.is_empty() {
        Err(AllPending {})
    } else {
        Ok(Response::new()
            .add_message(CosmosMsg::Custom(PalomaMsg {
                job_id: state.job_id,
                payload: Binary::new(
                    contract
                        .function("withdraw")
                        .unwrap()
                        .encode_input(tokens.as_slice())
                        .unwrap(),
                ),
                metadata: state.metadata,
            }))
            .add_attribute("action", "create_next_bot"))
    }
}

pub fn set_paloma(deps: DepsMut, info: MessageInfo) -> Result<Response<PalomaMsg>, ContractError> {
    let state = STATE.load(deps.storage)?;
    if state.owner != info.sender {
        return Err(Unauthorized {});
    }
    #[allow(deprecated)]
    let contract: Contract = Contract {
        constructor: None,
        functions: BTreeMap::from_iter(vec![(
            "set_paloma".to_string(),
            vec![Function {
                name: "set_paloma".to_string(),
                inputs: vec![],
                outputs: Vec::new(),
                constant: None,
                state_mutability: StateMutability::NonPayable,
            }],
        )]),
        events: BTreeMap::new(),
        errors: BTreeMap::new(),
        receive: false,
        fallback: false,
    };
    Ok(Response::new()
        .add_message(CosmosMsg::Custom(PalomaMsg {
            job_id: state.job_id,
            payload: Binary::new(
                contract
                    .function("set_paloma")
                    .unwrap()
                    .encode_input(&[])
                    .unwrap(),
            ),
            metadata: state.metadata,
        }))
        .add_attribute("action", "set_paloma"))
}

pub fn update_compass(
    deps: DepsMut,
    info: MessageInfo,
    new_compass: String,
) -> Result<Response<PalomaMsg>, ContractError> {
    let state = STATE.load(deps.storage)?;
    if state.owner != info.sender {
        return Err(Unauthorized {});
    }
    let new_compass_address: Address = Address::from_str(new_compass.as_str()).unwrap();
    #[allow(deprecated)]
    let contract: Contract = Contract {
        constructor: None,
        functions: BTreeMap::from_iter(vec![(
            "update_compass".to_string(),
            vec![Function {
                name: "update_compass".to_string(),
                inputs: vec![Param {
                    name: "new_compass".to_string(),
                    kind: ParamType::Address,
                    internal_type: None,
                }],
                outputs: Vec::new(),
                constant: None,
                state_mutability: StateMutability::NonPayable,
            }],
        )]),
        events: BTreeMap::new(),
        errors: BTreeMap::new(),
        receive: false,
        fallback: false,
    };

    Ok(Response::new()
        .add_message(CosmosMsg::Custom(PalomaMsg {
            job_id: state.job_id,
            payload: Binary::new(
                contract
                    .function("update_compass")
                    .unwrap()
                    .encode_input(&[Token::Address(new_compass_address)])
                    .unwrap(),
            ),
            metadata: state.metadata,
        }))
        .add_attribute("action", "update_compass"))
}

pub fn update_blueprint(
    deps: DepsMut,
    info: MessageInfo,
    new_blueprint: String,
) -> Result<Response<PalomaMsg>, ContractError> {
    let state = STATE.load(deps.storage)?;
    if state.owner != info.sender {
        return Err(Unauthorized {});
    }
    let new_blueprint_address: Address = Address::from_str(new_blueprint.as_str()).unwrap();
    #[allow(deprecated)]
    let contract: Contract = Contract {
        constructor: None,
        functions: BTreeMap::from_iter(vec![(
            "update_blueprint".to_string(),
            vec![Function {
                name: "update_blueprint".to_string(),
                inputs: vec![Param {
                    name: "new_compass".to_string(),
                    kind: ParamType::Address,
                    internal_type: None,
                }],
                outputs: Vec::new(),
                constant: None,
                state_mutability: StateMutability::NonPayable,
            }],
        )]),
        events: BTreeMap::new(),
        errors: BTreeMap::new(),
        receive: false,
        fallback: false,
    };

    Ok(Response::new()
        .add_message(CosmosMsg::Custom(PalomaMsg {
            job_id: state.job_id,
            payload: Binary::new(
                contract
                    .function("update_blueprint")
                    .unwrap()
                    .encode_input(&[Token::Address(new_blueprint_address)])
                    .unwrap(),
            ),
            metadata: state.metadata,
        }))
        .add_attribute("action", "update_blueprint"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    unimplemented!()
}
