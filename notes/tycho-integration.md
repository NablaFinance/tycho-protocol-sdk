# Nabla Tycho Integration Plan

What is confusing is that there are two types of documentation:
- General Integration Documentation: [Integrations Overview](https://docs.propellerheads.xyz/integrations/)
- Tycho-Specific Documentation: [Tycho Integration](https://docs.propellerheads.xyz/tycho/)


## 1. Protocol Logic Implementation

### Key Resources
- [VM Integration](https://docs.propellerheads.xyz/integrations/logic/vm-integration/ethereum-solidity)
- [Tycho Protocol SDK - Logic/VM Integration](https://docs.propellerheads.xyz/tycho/for-dexs/protocol-integration-sdk/logic/vm-integration)
- [SDK Interfaces](https://github.com/propeller-heads/tycho-protocol-sdk/tree/main/evm/src/interfaces)
- [SDK Template](https://github.com/propeller-heads/tycho-protocol-sdk/tree/main/evm/src/template)
- Example Implementation: [Balancer V2 Adapter](https://github.com/propeller-heads/tycho-protocol-sdk/tree/main/evm/src/balancer-v2)


### Implementation Steps
1. **VM Integration**
    - Implement the `ISwapAdapter` interface in Solidity to simulate Nabla's AMM logic.
    - Provide a manifest file describing the adapter's capabilities and deployment metadata.

2. **Core Methods in `ISwapAdapter`**
    - `price`: Calculate prices for given amounts in `buyToken`/`sellToken` units.
    - `swap`: Execute swaps, returning gas usage and pricing details.
    - `getLimits`: Define trading limits per token.
    - `getCapabilities`: Describe supported operations for pools.
    - *Optional*: `getTokens` and `getPoolIds` (useful for testing, non-essential).

3. **Testing and Validation**
    - Conduct testing using Foundry.
    - Perform fork testing against live contracts.
    - Include fuzz testing and reference implementations (e.g., Balancer V2).

4. **Nabla-Specific Considerations**
    - Integrate oracle-based pricing and EV:GO volatility protection in methods like `price` and `swap`.
    - Ensure manifest accuracy for describing capabilities, pool addresses, and runtime bytecode.


# Interfaces provided by Propellerheads

1. [ISwapAdapterTypes.sol](https://github.com/propeller-heads/tycho-protocol-sdk/blob/main/evm/src/interfaces/ISwapAdapterTypes.sol)
   - Enums:
     - `Orderside`: trade direction (buy or sell)
     - `Capability`: defines features of trading pool (buy/sell, price functions, fee handling, etc.) to accomodate different protocols
   - Structs:
     - `Fraction`: rational numbers (numerator and denominator)
     - `Trade`: amount of tokens, gas used, price
   - Errors:
     - `Unavailable`
     - `LimitExceeded`
     - `NotImplemented`

2. [ISwapAdapter.sol](https://github.com/propeller-heads/tycho-protocol-sdk/blob/main/evm/src/interfaces/ISwapAdapter.sol)
   - functions
     - `price`: pool prices for specified amounts of tokens
     - `swap`: simulate execution between two tokens in a pool
     - `getLimits`: maximum trade limits for a given token pair in a pool
     - `getCapabilitites`: retrieve pool features
     - `getTokens`: tokens available in pool
     - `getPoolIds`: range of pool IDs for a protocol

3. [ISwapExecutor.sol](https://github.com/propeller-heads/tycho-protocol-sdk/blob/main/evm/src/interfaces/ISwapExecutor.sol)
   - functions
     - `swap`: execute a trade on the liquidity pool

#### Requirements
- Nabla’s AMM logic in Solidity
    - Dynamic fee mechanism
    - Oracle logic and data sources
    - EV:GO volatility proection details and how it interfaces with swaps
    - Protocol-specific mechanisms like backstop pools or volatility protection

#### Questions:
- Q: Is the NablaPortal interface sufficient, or are additional interfaces needed for backstop and swap pools?
  A: It's not clear to me what , or even if any, interfaces are req, for the protocol logic. If the adapter and executor suffice, why would they require any interfaces from Nabla
- Q: Will Nabla expose details like pool structures, oracle pricing, and EV:GO mechanisms?
  A: As it is right now, ev.go is fully abstracted from the entrypoints. I don't see a need for such exposure.
- Q: Are custom parameters (e.g., volatility thresholds) required in the adapter logic?
  A: Good question,  depends on how will they use the adapter. If it, as i suspect, used for fork testing there shouldn't be a need for that. Re. backstop pool, it's only relevant for LPs and not swap users.
- Q: Are token limits (`getLimits`) and dynamic fees easily accessible through existing APIs or infrastructure?
  A: Fees are currently not dynamic (subject to change though), "token limits" are only dependent on the pool states, so fully on chain.
- Q: Should fork tests simulate specific scenarios like high-volatility conditions?
  A: We probably need to cross check with balancer impl 
- Q: Are optional methods like `getTokens` or `getPoolIds` feasible to implement now, or should we focus solely on required methods?
  A: focus on required, then optionals if low hanging fruit


## 2. Indexing

### Key Resources
- [Integration Indexing Overview](https://docs.propellerheads.xyz/integrations/indexing/overview)
- [Tycho Protocol SDK Indexing](https://docs.propellerheads.xyz/tycho/for-dexs/protocol-integration-sdk/indexing)
- Data Models: [Tycho Protocol SDK](https://github.com/propeller-heads/tycho-protocol-sdk/tree/main/proto/tycho/evm/v1)

### Data Model and Changes
- **State Changes:**
    - New protocol components (e.g., pools, pairs) detected via `BlockTransactionProtocolComponents`.
    - State changes in contracts (e.g., storage slot updates).
    - ERC20 balance changes derived from relative deltas.
    - Component tracking is strictly validated: no unannounced state changes should occur.
- **Intermediate Data:**
    - Custom Protobuf messages defined in `proto/custom-messages.proto` and linked via `substreams.yaml`.

### Integration Requirements
- Substreams must produce a `BlockChanges` output, ensuring:
    - No duplicate transactions in the changes attribute.
    - Balances and integers are encoded as unsigned big-endian, and strings as UTF-8.
    - Reserved attribute names are only used for their intended purposes.
- The indexing system should ensure that each relative change is assigned a strictly increasing ordinal to maintain transaction granularity and aggregation accuracy.


### Substreams Integration Steps

1. **Setup**
    - Install Rust and Substreams.
    - Clone the Propeller Protocol Lib repository.
    - Create a new package by copying the Ethereum template and adapting it for the specific protocol.
    - Generate the necessary protobuf code using the Substreams CLI (`substreams protogen ./proto`).
    - Add the new package to the workspace by modifying `Cargo.toml`.

2. **Package Structure**
    - `map_components`: Detect new protocol components (e.g., pools, pairs). Emit `BlockTransactionProtocolComponents` to capture all newly created components within a block.
    - `store_components`: Persist detected components for further processing. This store should 
        - Map component IDs to their associated contracts.
        - Include mechanisms to detect duplicates or unregistered state changes.
    - `map_relative_balances`: Track relative balance deltas, normalize them into absolute balances, and emit them as `BlockBalanceDeltas`.
    - `store_balances`: Store balances using additive logic to track changes over time. Should leverage additive storage (`StoreAddBigInt`) to track absolute balances efficiently.
    - `map_protocol_changes`: Combine all data into `BlockChanges` using helper functions:
        - `tycho_substreams::balances::aggregate_balances_changes`
        - `tycho_substreams::contract::extract_contract_changes`

3. **Testing**
    - Validate indexing output against expected states over a specified block range.
    - Use YAML configuration for tests.
    - Ensure environment variables (e.g., `RPC_URL`, `SUBSTREAMS_API_TOKEN`) are properly set.

### Reserved attributes

Reserved attributes are predefined names used exclusively for specific purposes in protocol indexing.
- **Static Attributes:**
    1. `manual_updates`: Control updates for components with frequent changes.
    2. `pool_id`: Specify pool IDs when differing from `ProtocolComponent.id`.
- **State Attributes:**
    1. `stateless_contract_addr_{index}`: Address of stateless contracts.
    2. `stateless_contract_code_{index}`: Bytecode of stateless contracts.
    3. `balance_owner`: Token owner in protocols with vaults.
    4. `update_marker`: Trigger updates when `manual_updates` is enabled.

### Requirements: 
- All the reserved attribute names and their purposes ()
- Knowing the specific types of protocol state changes to index. State changes are classified into categories of balance changes, storage slot updates, and new component creation.
- The roles and relationships of all contracts in the protocol. This is critical to defining the `map_components` and `store_components` logic.
- Understanding how to configure Substreams modules for Nabla protocol. We need to identify and handle dependencies between contracts.


### Questions:
- Q: Does Nabla have existing indexing infrastructure (e.g., subgraphs)?
  A: Subgraphs on Alchemy @Niels is in charge of that
- Q: Are there silent state changes (implicit updates) that occur (e.g., balance updates not linked to explicit events)?
  A: `totalLiablities`, `reserve`, `reserveWithSlippage` are updated on swaps without appearing in events
- Q: Are there any state changes that depend on multiple contracts working together (e.g., changes in one contract triggering updates in another)? How should these dependencies be handled in the indexing pipeline?
  A: no such state changes, although swap outcomes depend on states of numerous contracts. Not sure how to handle these dependencies yet
- Q: Are there edge cases requiring special treatment?
  A: don't think so, but remains to be seen


## 3. Simulation / Execution

### Key Resources
- [Simulation Overview](https://docs.propellerheads.xyz/integrations/execution/overview)
- [Tycho Protocol SDK Simulation](https://docs.propellerheads.xyz/tycho/for-dexs/protocol-integration-sdk/simulation)
- To enable simulation, need to first be integration into https://github.com/propeller-heads/tycho-simulation

### Implementation Options
- **Native Protocol (Rust):**
    - Define a protocol state struct implementing the `ProtocolSim` trait.
    - Implement `TryFromWithBlock` for `ComponentWithState`.
- **VM Protocol (EVM):**
    - Implement the `ISwapAdapter` (see Protocol Logic Implementation).
    - Generate adapter runtime (`evm/scripts/buildRuntime.sh`) and place it in `tycho-simulations/src/protocol/vm/assets`. Follow the naming convention: `<Protocol><Version>Adapter.evm.runtime`.
    - Use filters to exclude unsupported pools during registration.

If the implementation does not support all pools for a protocol:
- Create a filter function to exclude unsupported pools.
- Use the filter when registering the exchange in `ProtocolStreamBuilder`.
- see: https://github.com/propeller-heads/tycho-simulation/blob/03d845a363836e6371e10e9f24d9c7f2042fa4db/src/evm/protocol/filters.rs


### SwapEncoder
- https://docs.propellerheads.xyz/integrations/execution/swap-encoder
- https://github.com/propeller-heads/tycho-protocol-sdk/blob/main/docs/execution/swap-encoder.md

An interface to encode the necessary data for a swap used by the `SwapExecutor`. 

Key method:
- `encode_swap_struct` encodes swap details into a bytes object for execution.
    - `swap`: Dictionary containing swap details:
        - `pool_id`: Identifier of the liquidity pool.
        - `sell_token`: Token to be sold (e.g., DAI).
        - `buy_token`: Token to be bought (e.g., WETH).
        - `split`: Split percentage between pools (often set to 0).
        - `sell_amount`: Amount of sell_token to use in the swap.
        - `buy_amount`: Amount of buy_token expected from the swap.
        - `token_approval_needed`: Boolean indicating if token approval is required.
        - `pool_tokens`: Optional tuple for additional pool-specific token data.
        - `pool_type`: Type of pool for the swap (e.g., "BalancerStablePoolState").
    - `receiver`: Address receiving the output tokens.
    - `encoding_context`: Context-specific additional data for encoding.
    - `**kwargs`: Additional protocol-specific parameters.
    Returns: Encoded swap data as a bytes object.

### SwapExecutor
- https://docs.propellerheads.xyz/integrations/execution/swap-executor
- https://github.com/propeller-heads/tycho-protocol-sdk/blob/main/docs/execution/swap-executor.md

The ISwapExecutor interface facilitates token swaps on a protocol's liquidity pool, using protocol-specific logic to determine input or output amounts.

Key method:
- `swap(uint256 givenAmount, bytes calldata data)` executes a token swap on the liquidity pool.
    - `givenAmount`: Token amount for the swap (either input or output).
    - `data`: Encoded protocol-specific information (e.g., pool and token addresses), passed from the `SwapStructEncoder`.
    - Returns: 

Implementation steps:

1. Define Protocol-Specific Logic:
    - Implement the swap function to interact with the protocol's liquidity pool.
    - Decode the data parameter to extract necessary details like token/pool addresses.
2. Handle Input and Output Swaps:
    - Identify if givenAmount represents the input or output token.
    - Use the pool's pricing logic to calculate the corresponding swapped amount.
3. Error Handling:
    - Implement checks using `ISwapExecutorErrors`:
        - `InvalidParameterLength`: Ensure data contains the required parameters for decoding.
        - `UnknownPoolType`: Handle unsupported pool types gracefully.
4. Manage Token Approvals:
    - Ensure token allowances are granted before swaps.
    - Automate this step if required by the protocol.
5. Support Token Transfers:
    - Ensure the swapped tokens are transferred to the designated receiver within the swap function or in a separate transfer step.
6. Optimize for Gas Efficiency:
    - Streamline the swap logic to minimize gas usage, though assembly is not required.
7. Ensure Security:
    - Validate input parameters (e.g., givenAmount, data structure).
    - Protect against reentrancy attacks using best practices like the checks-effects-interactions pattern.
    - Safeguard access control to ensure only authorized entities can call the swap function.


### Questions
- Q: Do you have an example swap flow we can analyze?
- Q: Should token approvals be managed within `SwapExecutor` or externally?
