Propellerheads


## Protocol Integration SDK
- https://docs.propellerheads.xyz/tycho/for-dexs/protocol-integration-sdk

Requirements:
1. Protocol logic: Provides simulations of the protocols logic.
	- Via VM integration (or via implementation of a Rust trait, but not yet implemented)
2. Indexing: Provide access to the protocol state that the simulation needs.
	- provide a substreams package that emits a set of messages
3. Execution: Define how to encode and execute swaps against the protocol
	- SwapExecutor: Component to swap over liquidity pools. Handles token approvals, manages input/output amounts, and executes securely and gas-efficiently. You need to implement your own SwapExecutor (Solidity contract), tailored to your protocol's logic.
	- SwapStructEncoder: Implement a SwapStructEncoder Python class, compatible with your SwapExecutor, formats input/output tokens, pool addresses, and other parameters correctly for the SwapExecutor.


# References implementation

## 1. Protocol logic
https://github.com/propeller-heads/tycho-protocol-sdk/tree/main/evm/src/interfaces
https://github.com/propeller-heads/tycho-protocol-sdk/tree/main/evm/src/balancer-v2

- Implement the ISwapAdapter.sol interface.
- Create a manifest file summarizing the protocol's metadata.

### Interfaces provided by Propellerheads

1. [ISwapAdapterTypes.sol](https://github.com/propeller-heads/tycho-protocol-sdk/blob/main/evm/src/interfaces/ISwapAdapterTypes.sol)
   - Enums:
  	 - Orderside: trade direction (buy or sell)
  	 - Capability: defines features of trading pool (buy/sell, price functions, fee handling, etc.) to accomodate different protocols
   - Structs:
  	 - Fraction: rational numbers (numerator and denominator)
  	 - Trade: amount of tokens, gas used, price
   - Errors:
     - Unavailable
     - LimitExceeded
     - NotImplemented

2. [ISwapAdapter.sol](https://github.com/propeller-heads/tycho-protocol-sdk/blob/main/evm/src/interfaces/ISwapAdapter.sol)
   - functions
     - price: pool prices for specified amounts of tokens
     - swap: simulate execution between two tokens in a pool
     - getLimits: maximum trade limits for a given token pair in a pool
     - getCapabilitites: retrieve pool features
     - getTokens: tokens available in pool
     - getPoolIds: range of pool IDs for a protocol

3. [ISwapExecutor.sol](https://github.com/propeller-heads/tycho-protocol-sdk/blob/main/evm/src/interfaces/ISwapExecutor.sol)
   - functions
     - swap: execute a trade on the liquidity pool


### Reference Implementation: Balancer

1. BalancerV2SwapAdapter.sol
   - functions
     - priceSingle: determines price of token pair by performing off-chain simulation of a swap with queryBatchSwap.
     - getSellAmount: 
     - price: helper function to iterate over priceSingle and return an array of prices
     - swap: for both OrderSides, calculate amount, transfer tokens and interact with Balancer's vault.swap. Calculates gas consumption and slippage limits (listed as TODO). Gas compute can be used for fine-tuning transaction costs and SWAP_DEADLINE_SEC ensures safety in volatile markets. Also uses safeIncreaseAllowance for handling token allowances (but is not set back at the end?)
     - getLimits: calculate maximum allowable sell and buy limits for a pool such that they respect Balancer's reserve constraints. Check for certain specific conditions (pre-mint of BPT tokens and verification of circulating supply to prevent underflow errors)
     - maybeGetBptTokenIndex
     - getBptCirculatingSupply
     - getCapabilities: expose supported capabilitites: SellOrder, BuyOrder, PriceFunction, HardLimits
     - getTokens: returns list of token addresses for poolId 
     - getPoolIds: Balancer does not support this (NotImplemented)

2. BalancerSwapExecutor.sol
   - Constants: vaultAddress, swapSelector and maxUint256
   - functions:
     - swap: performs the call to the Balancer vault.swap function using assembly

3. manifest.yaml
provides metadata and configuration for integrating the BalancerV2SwapAdapter contract 
- author
- constants (protocol_gas, capabilities)
- constract
- instances: deployment details for different chains (chain id, vault address)
- tests


## 2. substreams
https://github.com/propeller-heads/tycho-protocol-sdk/blob/main/substreams/ethereum-balancer-v2


- buf.gen.yaml: some basic configuration for protobuf
- build.rs: script for generating `src/abi` (substream code) from `abi` content (json ABIs)
- integation_test.tycho.yaml
- substreams.yaml
- `abi` folder
	- get_abis.py: simple script to obtain ABIs in json, stored in `abi` folder for all relevant contracts.
- `src` folder
	- `abi`: build.rs (in repo root) leverages `use substreams_ethereum::Abigen` to do most of the work, namely creating `src/abi` for the substreams.
	- `lib.rs`: simply references the modules 
	- `modules.rs`: contains the code that is references in the substreams.yaml. The code is used for the indexing of events related to Balancer as follows:
	  - stores protocol components like pools
	  - track balance delta's from events (PoolBalanceChanged, Swap, and PoolBalanceManaged)
	  - record changes grouped by transcations
	  functions:
	    - map_components: identifies newly created pools, and store_components persists their identifiers
	    - store_components: stores protocol components in a substream store
	    - map_relative_balances: calculates balance deltas for tokens in pools
	    - store_balances: aggregates balances into final balances stored in substream store
	    - map_protocol_changes: combines all transaction-level changes (e.g., new pools, token balance deltas, contract storage updates)
	- `pool_factories.rs`: 
	  functions:
	    - address_map: uses match to compare the provided pool_factory_address against known factory addresses. For each factory address, specific decoding logic is applies using the appropriate ABI to handle pool creation
	      - function analyzes the transaction to retrieve logs and calls associated with pool creation
	      - helper functions: get_pool_registered and get_token_registered


## 3. Encoding and execution of swaps
https://github.com/propeller-heads/tycho-protocol-sdk/tree/main/evm/src/balancer-v2
https://github.com/propeller-heads/tycho-protocol-sdk/blob/main/propeller-swap-encoders/propeller_swap_encoders/balancer.py

- SwapExecutor: see above
- SwapStructEncoder: very simple
