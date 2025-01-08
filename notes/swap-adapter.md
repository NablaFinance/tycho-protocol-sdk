# NablaPortalSwapAdapter

The purpose of the `NablaPortalSwapAdapter` is to abstract Nabla's router-level logic into Tycho's unified interface for swaps, making Nabla one of the protocols Tycho can interact with. This involves implementing only the Nabla-specific parts necessary to translate Tycho's swap interface into Nabla's requirements. The adapter should act as a thin abstraction layer between Tycho and Nabla. 

## Interfaces provided by Propellerheads

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

## Swaps

Tycho has a generic swap interface, while Nabla requires token paths and router paths. Our adapter needs to fill in the gaps so Tycho can work with Nabla. Nabla's swap process involves multiple contracts:
- NablaPortal orchestrates swaps.
- NablaRouter handles token routing across pools.
- Pools are single-asset ERC20s.

The swap function in NablaPortal iterates over `_tokenPath` and `_routerPath`:
```
for (uint256 i = 0; i < _routerPath.length; i++) {
    address router = _routerPath[i];
    address tokenInOut[0] = _tokenPath[i];
    address tokenInOut[1] = _tokenPath[i + 1];
    INablaRouter(router).swapExactTokensForTokensWithoutPriceFeedUpdate(..., tokenInOut, ...);
}
```

A critical observation is that swaps in Nabla do not directly target individual pools using `poolId`. Instead, swaps are routed via `NablaRouter`. This brings up an important question: 

**Should the NablaRouter be considered the equivalent of a traditional multi-asset pool?**

### Arguments For:
- **Functional Equivalence**:  
  Routers facilitate swaps between single-asset pools, effectively acting as the functional equivalent of multi-asset pools in traditional DEXs. Without this equivalence, the interface loses cohesion with the intended purpose of `poolId`.  
- **Alignment with Tycho Interface**:  
  Assigning `poolId` to routers (e.g., their contract address) ensures that the `poolId` parameter retains its intended utility in the Tycho interface. Without it, all ISwapAdapter methods requiring `poolId` would need significant re-interpretation or workaround solutions.  
- **Simplified Logic**:  
  Identifying routers as pools offloads the complexity of computing `_tokenPath` and `_routerPath` for swaps. By mapping `poolId` to a router, these paths can be abstracted from the interface layer, leaving pathfinding and route optimization as the responsibility of solvers.  
- **Conceptual Consistency with Interface Design**:  
  Routers handle multiple asset pairs and their swaps, much like pools do in traditional DEX designs. Assigning them a `poolId` streamlines the integration and better fits Tycho's abstraction.

### Arguments Against:
- **Conceptual Difference**:  
  Treating routers as pools introduces a mismatch with Nabla’s actual architecture, where routers are explicitly not pools but intermediaries for multi-pool routing. While this is a semantic difference, it may cause confusion when reasoning about the system.  
- **Deviation from Naming Conventions**:  
  Traditional multi-asset pools have distinct characteristics (e.g., liquidity reserves for multiple tokens), whereas NablaRouters are simply routing mechanisms. Using the term “pool” for routers could muddy the conceptual waters, even if the practical equivalence holds.

Semantics are crucial here. Recognizing that NablaRouter is functionally equivalent to a "pool" clarifies the role of `poolId` in the Tycho interface. Assigning `poolId` to the router (e.g., using its contract address) preserves the integrity of the abstraction and avoids reimplementing pathfinding logic in the interface layer.

#### Complication:
Router-level swap functions are gated by the `allowed(amount)` modifier, restricting direct invocation to the `NablaPortal`. This means that instead of calling swap functions directly on a router, the `NablaPortal` must be used to execute swaps.  
To adapt this:
- The `tokenPath` can be defined as `[sellToken, buyToken]`.  
- The `routerPath` can be defined as `[router]`.  
- The swap is then invoked on the `NablaPortal`, effectively achieving the same outcome as calling the router directly.  


## Methods


### `getCapabilities`
✅ SellOrder					Required for swaps; directly supported by Nabla.
✅ BuyOrder						Nabla allows buy-side swaps (opposite of sell).
✅ PriceFunction				Implemented via `quoteSwapExactTokensForTokens`.
❌ FeeOnTransfer				No explicit support for tokens with transfer fees.
❌ ConstantPrice				Price changes based on slippage and trade size.
❌ TokenBalanceIndependent		Nabla enforces limits based on token balances.
✅ ScaledPrices					Token price scaling is supported in `_convertAmount`.
✅ HardLimits					Sell limits are enforced in Nabla.
❌ MarginalPrice				No direct marginal price calculation support.

Note: only UniswapV2 returns `MarginalPrice` as well as part of Capabilities, but in their implementation for determining marginal prices is **incorrect**... as they [they just return zero](https://github.com/propeller-heads/tycho-protocol-sdk/blob/b8aeaa3dc6e7242a5dd23681921258ef2cb3c6dd/evm/src/uniswap-v2/UniswapV2SwapAdapter.sol#L134-L141)



---
Note, below this line is somewhat outdates (still consider graph construction and pathfinding)
---


### `price`
Returns the prices for specified trade amounts, including any fees. This method is critical for external solvers to estimate costs and benefits of trades without executing them.

Signature: `function price(bytes32 poolId, address sellToken, address buyToken, uint256[] memory specifiedAmounts) external returns (Fraction[] memory calculatedPrices)`

Nabla provides a `quoteSwapExactTokensForTokens` method, which calculates a price quote for a given swap without actually performing it. This is what we want to use for implementing the `price` function in the Tycho interface, because:
- The `price` function in Tycho should "Return prices in buyToken/sellToken units."
- Using `quoteSwapExactTokensForTokens` ensures that the price estimation includes Nabla’s routing cost and fees.
- Ensures that the returned prices are being scaled to token decimals.

**Purpose:**
- To provide price estimates for specified amounts using Nabla's oracle data.
- While optional (can revert if unimplemented), this functionality is critical for external solvers that need to estimate costs/benefits before committing to a swap. If this method isn't implemented, solvers have to approximate prices numerically based on other data (e.g. swap results).

**Notes:**
- Calculates prices for specified amounts, ideally including all fees. For dynamic fees, the returned price should include the minimum possible fee. This can be used for efficient price discovery without actually executing a trade.
- `_priceUpdateData` is essential for price updates during a (multihop) swap. It is passed to `NablaPortal._updatePriceFeeds` before invoking `_swap`. It needs to be called right before calling `priceOracleAdapter.getAssetPrice()`, which is invoked internally in `NablaRouter._executeSwap`.
	- `NablaPortal.priceOracleAdapter` must be queried for the price of `sellToken` and `buyToken` (price of intermediate tokens in swap path may be outdated as they are inconsequential)
	- Updating price feeds incurs costs and is a payable operation. This conflicts with the view requirement of the `price` method
	- Consider caching price data to reduce query costs.
- fees: `SwapPool.swapFees()` returns `SwapPool.swapFeeConfig`, which includes`lpFee`, `backstopFee` and `protocolFee`. These are used in `SwapPool._calculateSwapOutParameters`.
- If unimplemented, this function can revert with `NotImplemented`.

Steps:
1. Verify inputs:
	- `specifiedAmounts` is non-zero.
	- `sellToken` and `buyToken` exist in the routers' swappools retrieved via `NablaPortal.getRouters()`
	- Ensure both tokens have valid price feeds in the oracle adapter
	- Optional: We could check for existence of a tokenPath and routerPath
	- Optional: We could iterate the former to check that `!guardOn[router] || guardOracle.isPermitted(tokenIn, tokenOut)`
2. Fetch router and pool data
	- Iterate through the routers (`NablaPortal.getRouters()`)
	- check `NablaPortal.getRouterAssets(router_address)` to see if `sellToken` and `buyToken` are part of it.
	- if so, check that the pool `NablaRouter.poolByAsset(asset_address)` exists.
3. Update price feeds
	- Construct `_priceUpdateData` to update the prices of `sellToken` and `buyToken`
	- Use `priceOracleAdapter = IPriceOracleAdapter(NablaPortal.oracleAdapter)` to call `getUpdateFee` and `updatePriceFeeds`
4. Fetch Updated Prices
	- `sellTokenPrice = priceOracleAdapter.getAssetPrice(sellToken)`
	- `buyTokenPrice = priceOracleAdapter.getAssetPrice(buyToken)`
	- verify that `sellTokenPrice > 0` and `buyTokenPrice > 0`
5. Calculate prices
	- for each `specifiedAmount`
		- convert `specifiedAmount` of `sellToken` to its equivalent in `buyToken`
		- incorporate trading fees, both static and dynamic (use minimum fee for dynamic fees)
			- fees apply to each swap out of a pool (hence fees are tokenPath and routerPath dependent)
			- fees include: `lpFee`, `backstopFee` and `protocolFee`, which are quantities defined in `SwapPool.swapFeeConfig` and scaled proportial to the `specifiedAmount`.
			- `lpFee` is adjusted (`effectiveLpFee_`) using `slippageCurve.inverseDiagonal`, but can only be lower or equal to the original `lpFee`.
			- `protocolFee` is adjusted for slippage as well. First the `newReserveWithSlippage` is computed using `slippageCurve.psi`, which is dependent on the `effectiveLpFee_`. The `protocolFeeWithSlippage_` is `max(reserveWithSlippageAfterAmountOut - newReserveWithSlippage, 0)`. Q: Does this means the `protocolFeeWithSlippage_` is always <= to the original `protocolFee`?
		- `price = specifiedAmount * (sellTokenPrice / buyTokenPrice) * (1 - feeRate)` 
		- However, we need store as `Fraction`:
			- `numerator = specifiedAmount * sellTokenPrice * (1 - feeRate)`
			- `denominator = buyTokenPrice`



#### NablaPortal.quoteSwapExactTokensForTokens
- Signature: `function quoteSwapExactTokensForTokens(uint256 _amountIn, address[] calldata _tokenPath, address[] calldata _routerPath, uint256[] calldata _tokenPrices) external view returns (uint256 amountOut_)`

We can levarage `quoteSwapExactTokensForTokens`. This function gives a straightforward way to calculate the output amount (`amountOut_`) along with the total swap fees for a multi-hop route. It abstracts away all the slippage, fee computation, and reserve balance complexity. By leveraging it, we avoid reimplementing the nuanced logic of slippage, effective fees, and reserve dynamics in `SwapPool`. Instead, you simply call this function to get the final output amount and use the price ratio as a fraction: `price = amountOut / specifiedAmount`.

Required inputs:
- `_amountIn`: Your `specifiedAmount` in `sellToken` units.
- `_tokenPath`: A route from `sellToken` to `buyToken` (can be direct or multi-hop).
- `_routerPath`: Routers for each hop in `_tokenPath`.
- `_tokenPrices`: Prices of tokens in `_tokenPath`.

Notes: 
- `specifiedAmount` can be passed directly as `_amountIn`.
- `_tokenPath` and `_routerPath` need to be computed from `sellToken` and `buyToken`
- `_tokenPrices` need to be obtained from an oracle for all tokens in the `_tokenPath`
- the quoting mechanism essentially discards detailed fees since they are already accounted for in the returned `effectiveAmount_`



Requirements:
1. Protocol logic: Provides simulations of the protocols logic.
	- Via VM integration (or via implementation of a Rust trait, but not yet implemented)
2. Indexing: Provide access to the protocol state that the simulation needs.
	- provide a substreams package that emits a set of messages
3. Execution: Define how to encode and execute swaps against the protocol
	- SwapExecutor: Component to swap over liquidity pools. Handles token approvals, manages input/output amounts, and executes securely and gas-efficiently. You need to implement your own SwapExecutor (Solidity contract), tailored to your protocol's logic.
	- SwapStructEncoder: Implement a SwapStructEncoder Python class, compatible with your SwapExecutor, formats input/output tokens, pool addresses, and other parameters correctly for the SwapExecutor.

Updated Steps:
1. Verify inputs:
	- `specifiedAmounts` is non-zero.
2. Build a graph of possible routes
	- Use `NablaPortal.getRouters()` -> `NablaRouter.getRouterAssets()` -> `NablaRouter.poolByAsset()` to map routers and pools.
	- Represent the data as an undirected graph.
	- Paths should not contain cycles
	- Paths should not visit a given router more than once
3.  Find paths using BFS (or Dijkstra if only shortest matters)
	- A valid `tokenPath` and `routerPath` must exist for swapping `sellToken` to `buyToken`
	- For each iteration in a path (== combination of `tokenPath` and `routerPath`)
		- Tokens must have valid price feeds in the `oracleAdapter`
		- If there exists a `guardOracle` for the router, it should permit the token swap (`require(!guardOn[router] || guardOracle.isPermitted(tokenIn, tokenOut)`)
4. Fetch price data
	- For the tokens in the shortest path (or for the set of tokens in all paths)
	- Obtain oracle adapter: `priceOracleAdapter = IPriceOracleAdapter(NablaPortal.oracleAdapter)`
	- Define `_priceUpdateData`
		- 
	- Compute: `updateFee = priceOracleAdapter.getUpdateFee(_priceUpdateData)` (in Wei)
	- invoke: `priceOracleAdapter.updatePriceFeeds(_priceUpdateData)`
	- For each token in the path(s):
		`tokenPrice = priceOracleAdapter.getAssetPrice(token)`
5. Compute `amountOut`
	- For a given path (`tokenPath` + `routerPath`):
		- Iterate: for each `specifiedAmount` in `specifiedAmounts`
			- define `tokenPrices` to match tokens in `tokenPath`
			- `amountOut = quoteSwapExactTokensForTokens(specifiedAmount, tokenPath, routerPath, tokenPrices)`
6. Return:
	- Choose a path. For this chosen path
	- `updateFee = priceOracleAdapter.getUpdateFee(_priceUpdateData)` (NOTE: other fees already deducted in `quoteSwapExactTokensForTokens`)
	- `correctedAmountOut = ((amountOut * buyTokenPrice) - updateFee) / buyTokenPrice`  NOTE: units need to be consistent (Wei)!
	- `calculatedPrices = [amountOut / specifiedAmount for amountOut, specifiedAmount in zip(correctedAmountsOut, specifiedAmounts)]`


Questions:
- why is `backstopFee_` not used? Incomplete feature implementation?
- is iterative fetching prices in batch cheaper than fetching iteratively?
- should we fetch data for the set of assets in the possible paths of a swap? Or only the shortest path (which may fail)? Should we iterate multiple paths? Only in case of failure, or always all of them to obtain the best?
- since slippage can have a positive effect on a swap, should we allow paths through multiple assets of the same router instead of excluding them, or is it possible that the slippage benefit outweighs the costs (gas + fees)?
- the `updateFee` we compute from fetching price data if not included in the `amountOut` obtained from the `quoteSwapExactTokensForTokens`. We could correct for this. However, if we obtain prices for more assets than in the tokenPath, the `updateFee` might be inflated.
- When we compute `amountOut` we could iterate ALL paths. We also need to iterate all `specifiedAmounts`. We might find that for smaller quantities in the array of `specifiedAmounts` we get better prices when iterating a certain path, whereas for larger quantities we not be able to use this path, or get better prices using a different path. Do we want to use different paths for different quantities if the cost is lower (price is better)?

Notes:
	- we could find the largest quantity in `specifiedAmounts` and try swap that first. If a path can accomodate this, it can accomodate all smaller amounts. In fact, reserve data is available for asset pools, which allows us to rule out paths that can't support the largest `specifiedAmount` unfront.
	- `priceOracleAdapter.getUpdateFee` is a `public view` so we can get `updateFee` for free. Thus, we can obtain the `updateFee` for the token path that was used to compute the prices at the end and correct for this.



### swap
This is the core functionality required for routing swaps. It executes a trade on a specified pool, modifies the state, and provides details of the trade (calculated amount, gas used, price). On NablePortal, all swap functions invoke `_updatePriceFeeds(oracleAdapter, _priceUpdateData)` and then `_swap`. `NablePortal._swap` iterates a routerpath to swap tokens, swapping into one and out of another single asset pool. Each `swapIntoFromRouter` invokes `slippageCurve.inverseHorizontal` once, whereas the `swapOutFromRouter` invokes `slippageCurve.inverseDiagonal` once and `slippageCurve.psi` twice.

Signature: `function swap(bytes32 poolId, address sellToken, address buyToken, OrderSide side, uint256 specifiedAmount) external override returns (Trade memory trade)`

**Notes:**
- We need `_priceUpdateData` for the `oracleAdapter` to update price feeds.
- The adapter should implement graph-based pathfinding logic to identify possible routes and translate Tycho's generic swap input (`sellToken`, `buyToken`, `amount`) into Nabla-specific parameters (`_tokenPath`, `_routerPath`).
- Once the paths and data are determined, the adapter should call Nabla's router functions (e.g., `swapExactTokensForTokens`)
- If a path fails, the adapter may optionally retry with alternate paths, or it may pass the failure back to Tycho.
- Nabla requires `priceUpdateData`, the adapter should handle its construction or fetching (e.g., from an oracle)

Steps:
1. Inputs from Tycho: `sellToken`, `buyToken`, `side`, and `specifiedAmount`.
	- The `poolId`, isn't used and doesn't directly help with routing in the Nabla framework. Instead, we can iterate the routers and access the available pools for the assets from there.
2. Use `sellToken` and `buyToken` to build a graph of possible routes.
	- Use `NablaPortal.getRouters()` -> `NablaRouter.getRouterAssets()` -> `NablaRouter.poolByAsset()` to map routers and pools.
	- Represent the data as an undirected graph.
3. Find paths using BFS (or Dijkstra if only shortest matters)
	- paths should not contain cycles
	- paths should not visit a given router more than once
4. Select the shortest or "best" path based on criteria (e.g. fewest hops, lowest fees, highest liquidity)
5. Translate the selected path into Nabla's router-specific parameters
6. Call nabla router to execute the trade and return a `Trade` struct containing:
	- `calculatedAmount` (amount sold or bought).
	- `gasUsed` (gas consumed by the trade).
	- `price` (post-trade price or fallback `Fraction(0, 1)` if unavailable).


### getLimits
Retrieves the maximum trade amounts to prevent exceeding pool constraints. Limits ensure safe trade execution without errors.

Signature: `getLimits(bytes32 poolId, address sellToken, address buyToken) external view override returns (uint256[] memory limits)`

**Purpose:**
- Provides the maximum amounts for `sellToken` and `buyToken`.
- Ensures swaps stay within pool boundaries.

**Notes:**
- Nabla uses single asset pools, so limits depend on individual pool constraints.
- Implement logic to calculate these limits based on Nabla's pool mechanics.


### getCapabilities
Returns an array of `Capability` values representing the features of the trading pool.
Signature: `getCapabilities(bytes32 poolId, address sellToken, address buyToken) external pure override returns (Capability[] memory capabilities)`

**Purpose:**
- Advertises pool capabilities such as:
    - Support for buy/sell orders.
    - Scaled prices.
    - Hard limits.
- Helps Tycho understand the pool's behavior and features.

**Notes:**
- If a feature isn't supported, it should not be included in the returned array.


### getTokens
Returns the list of token addresses in a specified pool.

Signature: `function getTokens(bytes32 poolId) external pure override returns (address[] memory tokens)`

**Notes:**
- Nabla doesn't directly associate pools with `poolId`.
- Each pool holds a single token.


### getPoolIds
Returns a range of pool IDs starting from a specified offset, up to a specified limit.

Signature: `function getPoolIds(uint256 offset, uint256 limit) external pure override returns (bytes32[] memory ids)`

**Notes:**
   - Nabla doesn't directly associate pools with `poolId` in the way Tycho expects.
   - Instead, use the `NablaRouter` to retrieve pools by asset.


### Extra notes:
- `_updatePriceFeeds`: outside of NablaPortal, this function is called in
	- `NablaRouter.swapExactTokensForTokens`
	- `NablaBackstopPool.deposit`
	- `NablaBackstopPool.finalizeWithdrawBackstopLiquidity`
	- `NablaBackstopPool.redeemSwapPoolShares`
	- `NablaBackstopPool.finalizeWithdrawExcessSwapLiquidity`
	- `NablaBackstopPool.redeemCrossSwapPoolShares`
	Q: why is this function not called in SwapPool?





---


## `NablaPortal.quoteSwapExactTokensForTokens`

```

    /**
     * @notice Get a quote for how many `_toToken` tokens `_amountIn` many `tokenIn`
     *         tokens can currently be swapped for.
     * @param _amountIn     The amount of input tokens to swap
     * @param _tokenPath    Array of tokens to swap along the route
     * @param _routerPath   Array of routers to use
     * @param _tokenPrices  Array of token prices fetched off-chain
     * @return amountOut_    Number of `_toToken` tokens that such a swap would yield right now
     */
    function quoteSwapExactTokensForTokens(
        uint256 _amountIn,
        address[] calldata _tokenPath,
        address[] calldata _routerPath,
        uint256[] calldata _tokenPrices
    ) external view returns (uint256 amountOut_) {
        require(_tokenPath.length == _routerPath.length + 1, "NP:quoteSwapExactTokensForTokens:ROUTER_TOKEN_ARRAY_SIZE");
        require(_tokenPath.length == _tokenPrices.length, "NP:quoteSwapExactTokensForTokens:TOKEN_PRICES_ARRAY_SIZE");

        uint256 amountIn = _amountIn;
        address[] memory tokenInOut = new address[](2);
        uint256[] memory tokenPricesInOut = new uint256[](2);

        for (uint256 i = 0; i < _routerPath.length; i++) {
            tokenInOut[0] = _tokenPath[i];
            tokenInOut[1] = _tokenPath[i + 1];

            tokenPricesInOut[0] = _tokenPrices[i];
            tokenPricesInOut[1] = _tokenPrices[i + 1];

            (amountIn,) = INablaRouter(_routerPath[i]).getAmountOut(_amountIn, tokenInOut, tokenPricesInOut);

            _amountIn = amountIn;
        }

        amountOut_ = amountIn;
    }
```

## `NablaRouter.getAmountOut`

```
    /**
     * @notice Get a quote for how many `_toToken` tokens `_amountIn` many `tokenIn`
     *         tokens can currently be swapped for.
     * @param _amountIn     The amount of input tokens to swap
     * @param _tokenInOut   Array of size two, indicating the in and out token
     * @param _tokenPrices  Array of size two, indicating the in and out token prices fetched off-chain
     * @return amountOut_   Number of `_toToken` tokens that such a swap would yield right now
     * @return swapFee_     The fee that is charged for the swap (in `_toToken` tokens)
     */
    function getAmountOut(uint256 _amountIn, address[] calldata _tokenInOut, uint256[] calldata _tokenPrices)
        external
        view
        returns (uint256 amountOut_, uint256 swapFee_)
    {
        require(_tokenPrices.length == 2, "NR:getAmountOut:TOKEN_PRICE_ARRAY_SIZE");

        return _getAmountOut(_amountIn, _tokenInOut, _tokenPrices);
    }
}
```

## `RouterCore._getAmountOut`

```

    /**
     * @notice Get a quote for how many `_toToken` tokens `_amountIn` many `tokenIn`
     *         tokens can currently be swapped for.
     * @param _amountIn     The amount of input tokens to swap
     * @param _tokenInOut   Array of size two, indicating the in and out token
     * @param _tokenPrices  Array of size two, indicating the in and out token prices fetched off-chain from Pyth oracle
     * @return amountOut_   Number of `_toToken` tokens that such a swap would yield right now
     * @return swapFee_     The fee that is charged for the swap (in `_toToken` tokens)
     */
    function _getAmountOut(uint256 _amountIn, address[] calldata _tokenInOut, uint256[] memory _tokenPrices)
        internal
        view
        returns (uint256 amountOut_, uint256 swapFee_)
    {
        require(_amountIn > 0, "RC:_getAmountOut:ZERO_AMOUNT");
        require(_tokenInOut.length == 2, "RC:_getAmountOut:TOKEN_ARRAY_SIZE");
        require(_tokenInOut[0] != _tokenInOut[1], "RC:_getAmountOut:TOKEN_ARRAY_DUPLICATE");

        address fromToken = _tokenInOut[0];
        address toToken = _tokenInOut[1];

        uint256 tokenPriceFrom = _tokenPrices[0];
        uint256 tokenPriceTo = _tokenPrices[1];

        uint256 rawOutAmount = _getAmountOutHelper(fromToken, toToken, _amountIn, tokenPriceFrom, tokenPriceTo);

        uint256 protocolFeeWithSlippage;
        uint256 effectiveLpFee;
        uint256 backstopFee;

        (amountOut_, protocolFeeWithSlippage, effectiveLpFee, backstopFee) =
            poolByAsset[toToken].quoteSwapOut(rawOutAmount);

        swapFee_ = protocolFeeWithSlippage + effectiveLpFee + backstopFee;
    }
```

## `RouterCore._getAmountOutHelper`

```
    function _getAmountOutHelper(
        address _fromToken,
        address _toToken,
        uint256 _amountIn,
        uint256 _tokenPriceFrom,
        uint256 _tokenPriceTo
    ) private view returns (uint256 rawOutAmount_) {
        //Cache to save some gas
        ISwapPoolPermissioned poolByAssetFromToken = poolByAsset[_fromToken];
        ISwapPoolPermissioned poolByAssetToToken = poolByAsset[_toToken];

        require(address(poolByAssetFromToken) != address(0), "RC:_getAmountOutHelper:ASSET_NOT_REGISTERED");
        require(address(poolByAssetToToken) != address(0), "RC:_getAmountOutHelper:ASSET_NOT_REGISTERED");

        // user funds into swap pool
        uint256 effectiveAmountIn = poolByAssetFromToken.quoteSwapInto(_amountIn);

        rawOutAmount_ = _convertAmount(
            effectiveAmountIn,
            _tokenPriceFrom,
            _tokenPriceTo,
            poolByAssetFromToken.assetDecimals(),
            poolByAssetToToken.assetDecimals()
        );
    }

    /**
     * @notice Price convert amounts of tokens
     * @notice The two involved tokens are called "from token" and "to token"
     * @notice Allows that tokens have different numbers of decimals
     * @param _fromAmount The amount of the from token
     * @param _fromPrice The price of the from token (in terms of a reference asset)
     * @param _toPrice The price of the to token (in terms of a reference asset)
     * @param _fromDecimals The number of decimals of the from token
     * @param _toDecimals The number of decimals of the to token
     * @return toAmount_ The equivalent amount of to tokens
     */
    function _convertAmount(
        uint256 _fromAmount,
        uint256 _fromPrice,
        uint256 _toPrice,
        uint8 _fromDecimals,
        uint8 _toDecimals
    ) internal pure returns (uint256 toAmount_) {
        if (_fromDecimals > _toDecimals) {
            toAmount_ = (_fromAmount * _fromPrice) / _toPrice / (10 ** uint256(_fromDecimals - _toDecimals));
        } else {
            toAmount_ = (_fromAmount * _fromPrice * (10 ** uint256(_toDecimals - _fromDecimals))) / _toPrice;
        }
    }
```

## `SwapPool.quoteSwapOut`

```

    /**
     * @notice Get a quote for the effective amount of tokens, incl. slippage and fees
     * @param _amount The amount of pool asset to swap out of the pool
     * @return effectiveAmount_ Effective amount, incl. slippage and fees
     * @return protocolFeeWithSlippage_ The protocol fee that is to be sent to the treasury
     * @return effectiveLpFee_ The actual LP fee – totalLiabilities should be incremented by this value
     * @return backstopFee_ The effective backstop fee
     */
    function quoteSwapOut(
        uint256 _amount
    )
        public
        view
        returns (
            uint256 effectiveAmount_,
            uint256 protocolFeeWithSlippage_,
            uint256 effectiveLpFee_,
            uint256 backstopFee_
        )
    {
        (
            effectiveAmount_,
            protocolFeeWithSlippage_,
            effectiveLpFee_,
            ,
            backstopFee_
        ) = _calculateSwapOutParameters(_amount);
    }

    /**
     * @notice Complete calculation involved in a swap out operation
     * @param _amount The raw amount of assets to swap out
     * @return effectiveAmount_ The actual amount to return to the user
     * @return protocolFeeWithSlippage_ The protocol fee that is to be sent to the treasury
     * @return effectiveLpFee_ The actual LP fee – totalLiabilities should be incremented by this value
     * @return newReserve_ The new value of `reserve` after this swap out
     * @return backstopFee_ The effective backstop fee
     */
    function _calculateSwapOutParameters(
        uint256 _amount
    )
        internal
        view
        returns (
            uint256 effectiveAmount_,
            uint256 protocolFeeWithSlippage_,
            uint256 effectiveLpFee_,
            uint256 newReserve_,
            uint256 backstopFee_
        )
    {
        require(_amount > 0, "SP:_calculateSwapOutParameters:ZERO_AMOUNT");

        uint256 oldTotalLiabilities = totalLiabilities;
        uint256 oldReserveWithSlippage = reserveWithSlippage;

        uint256 lpFee = (_amount * swapFeeConfig.lpFee) / 1_000_000;
        backstopFee_ = (_amount * swapFeeConfig.backstopFee) / 1_000_000;
        uint256 protocolFee = (_amount * swapFeeConfig.protocolFee) / 1_000_000;

        uint256 reducedReserve = backstopFee_ + protocolFee < _amount
            ? reserve + backstopFee_ + protocolFee - _amount
            : reserve;

        effectiveLpFee_ = slippageCurve.inverseDiagonal(
            reducedReserve,
            oldTotalLiabilities,
            oldReserveWithSlippage,
            poolAssetDecimals
        );

        if (effectiveLpFee_ > lpFee) {
            effectiveLpFee_ = lpFee;
        }

        uint256 reserveWithSlippageAfterAmountOut = slippageCurve.psi(
            reducedReserve + effectiveLpFee_,
            oldTotalLiabilities + effectiveLpFee_,
            poolAssetDecimals
        );

        if (reserveWithSlippageAfterAmountOut > oldReserveWithSlippage) {
            reserveWithSlippageAfterAmountOut = oldReserveWithSlippage;
        }
        unchecked {
            effectiveAmount_ =
                oldReserveWithSlippage -
                reserveWithSlippageAfterAmountOut;
        }
        newReserve_ = reducedReserve + effectiveLpFee_ - protocolFee;

        uint256 newReserveWithSlippage = slippageCurve.psi(
            newReserve_,
            oldTotalLiabilities + effectiveLpFee_,
            poolAssetDecimals
        );
        unchecked {
            protocolFeeWithSlippage_ = newReserveWithSlippage >
                reserveWithSlippageAfterAmountOut
                ? 0
                : reserveWithSlippageAfterAmountOut - newReserveWithSlippage;
        }
    }
```

## `SwapPool.quoteSwapInto`

```
    /**
     * @notice Get a quote for the effective amount of tokens for a swap into
     * @param _amount The amount of pool tokens to swap into the pool
     * @return effectiveAmount_ Effective amount, incl. slippage (penalty or rewards)
     */
    function quoteSwapInto(
        uint256 _amount
    ) public view returns (uint256 effectiveAmount_) {
        require(_amount > 0, "SP:quoteSwapInto:ZERO_AMOUNT");

        effectiveAmount_ = _quoteSwapInto(_amount);
    }

    /**
     * @notice Get a quote for the effective amount of tokens for a swap into
     * @param _amount The amount of pool tokens to swap into the pool
     * @return effectiveAmount_ Effective amount, incl. slippage (penalty or rewards)
     */
    function quoteSwapInto(
        uint256 _amount
    ) public view returns (uint256 effectiveAmount_) {
        require(_amount > 0, "SP:quoteSwapInto:ZERO_AMOUNT");

        effectiveAmount_ = _quoteSwapInto(_amount);
    }

    /**
     * @notice Complete calculation involved in a swap into operation
     * @param _amount The amount of pool tokens to swap into the pool
     * @return effectiveAmount_ Effective amount, incl. slippage (penalty or rewards)
     */
    function _quoteSwapInto(
        uint256 _amount
    ) internal view returns (uint256 effectiveAmount_) {
        uint256 oldTotalLiabilities = totalLiabilities;
        uint256 oldReserve = reserve;

        effectiveAmount_ = slippageCurve.inverseHorizontal(
            oldReserve,
            oldTotalLiabilities,
            reserveWithSlippage + _amount,
            poolAssetDecimals
        );

        require(
            (oldReserve + effectiveAmount_) <=
                (maxCoverageRatioForSwapIn * oldTotalLiabilities) / 100,
            "SP:quoteSwapInto:EXCEEDS_MAX_COVERAGE_RATIO"
        );
    }
```


--- 

## What is `bytes[] calldata _priceUpdateData`?

This parameter for Pyth's price update process refers to the encoded data that contains information about updated prices for various price feeds. This data is crucial for interacting with Pyth's `updatePriceFeeds`. The `priceUpdateData` comes from Hermes, Pyth's price update service. Hermes provides multiple ways to retrieve price updates:
- REST API: Use endpoints such as /v2/updates/price/latest to fetch updates in a single request.
- Streaming: Continuously stream updates using SSE (Server-Sent Events).
- SDK: A language-specific SDK can be used to fetch and encode the data.

The data is provided in a binary format (hexadecimal string) and includes details such as:
- Price feed ID (e.g. BTC/USD or ETH/USD).
- Current price, confidence interval, and exponent.
- Publish time and metadata (e.g. slot, proof availability time).

On-chain contracts need this encoded data to update the Pyth price feeds. The process involves:
1. Using `fee = getUpdateFee(_priceUpdateData)` to calculate the fee required for the update.
2. Calling `updatePriceFeeds{value: fee}(_priceUpdateData)` to push the price data to the on-chain Pyth contract
