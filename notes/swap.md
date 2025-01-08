
# Protocol Logic Implementation

## Key Resources
- [ISwapAdapter.sol](https://github.com/propeller-heads/tycho-protocol-sdk/blob/main/evm/src/interfaces/ISwapAdapter.sol)
- [ISwapAdapterTypes.sol](https://github.com/propeller-heads/tycho-protocol-sdk/blob/main/evm/src/interfaces/ISwapAdapterTypes.sol)
- [ISwapExecutor.sol](https://github.com/propeller-heads/tycho-protocol-sdk/blob/main/evm/src/interfaces/ISwapExecutor.sol)
- [TemplateSwapAdapter.sol](https://github.com/propeller-heads/tycho-protocol-sdk/blob/main/evm/src/template/TemplateSwapAdapter.sol)
- [manifest.yaml](https://github.com/propeller-heads/tycho-protocol-sdk/blob/main/evm/src/template/manifest.yaml)

## Nabla Protocol Specifics
- Pyth oracle
- Single asset liquidity pools
- EV:GO
- SlippageCurve


## NablaPortal

function -> event

- **Swaps**
	- swapEthForExactTokens -> EthForExactTokensSwapped
	- swapExactTokensForEth -> ExactTokensForEthSwapped
	- swapExactTokensForTokens -> ExactTokensForTokensSwapped
- **Asset registration**
	- unregisterAsset -> AssetRegistered
	- unregisterAsset -> AssetUnregistered


## Swap

1. `NablaPortal.swapEthForExactTokens`
	- required: `_tokenPath.length >= 2`
	- required: `_tokenPath[0] == address(WETH)`
	- invokes: `updateFee = _updatePriceFeeds(oracleAdapter, _priceUpdateData)`
		- see: `OraclePriceUpdater._updatePriceFeeds`
	- required: `msg.value >= _amountIn + updateFee`
	- invokes: `WETH.deposit{value: _amountIn}()`
	- invokes: `swapAmounts = _swap(_amountIn, _amountOutMin, _tokenPath, _routerPath, _to, _deadline)`
		- see: `NablaPortal._swap`
	- computes: `refundAmount = msg.value - _amountIn - updateFee`
	- emit: `EthForExactTokensSwapped(msg.sender, _to, _routerPath, _tokenPath, swapAmounts)`
	- if `refundAmount > 0`: 
		- `(bool success,) = msg.sender.call{value: refundAmount}("")`
		- required: `success`
	- returns: `swapAmounts[1]`, which is the amount of (non-ETH) tokens

2. `NablaPortal.swapExactTokensForEth`
	- required: `_to != address(0)`
	- required: `_tokenPath.length >= 2`
	- required: `_tokenPath[_tokenPath.length - 1] == address(WETH)`
	- invokes: `_updatePriceFeeds(oracleAdapter, _priceUpdateData)`
		- see: `OraclePriceUpdater._updatePriceFeeds` 
	- invokes: `IERC20(_tokenPath[0]).safeTransferFrom(msg.sender, address(this), _amountIn)`
	- invokes: `swapAmounts = _swap(_amountIn, _amountOutMin, _tokenPath, _routerPath, address(this), _deadline)`
		- see: `NablaPortal._swap`
	- emit: `ExactTokensForEthSwapped(msg.sender, _to, _routerPath, _tokenPath, swapAmounts)`
	- invoke: `WETH.withdraw(swapAmounts[1])`
	- invoke: `(bool success,) = _to.call{value: swapAmounts[1]}("")`
	- required: `success`
	- returns: `swapAmounts[1]`, which is the amount of ETH tokens

3. `NablaPortal.swapExactTokensForTokens`
	- required: `_tokenPath.length >= 2`
	- invokes: `_updatePriceFeeds(oracleAdapter, _priceUpdateData)`
		- see: `OraclePriceUpdater._updatePriceFeeds`
	- invokes: `IERC20(_tokenPath[0]).safeTransferFrom(msg.sender, address(this), _amountIn)`
	- invokes: `swapAmounts = _swap(_amountIn, _amountOutMin, _tokenPath, _routerPath, _to, _deadline)`
		- see: `NablaPortal._swap`
	- emit:  `ExactTokensForTokensSwapped(msg.sender, _to, _routerPath, _tokenPath, swapAmounts)`
	- returns: `swapAmounts[1]`, which is the amount of (non-ETH) tokens

4. `NablaPortal._swap`
	- invokes: `GatedAccess.allowed(_amountIn)`
	- required: `_to != address(0)`
	- required: `_tokenPath.length == _routerPath.length + 1`
	- assigned: `amountIn = _amountIn`, `amountOutMin = 0`, `receiver = address(this)` and `tokenInOut = new address[](2)`
	- iterate all router paths:
		- assigned: `router = _routerPath[i]`, `tokenIn = _tokenPath[i]` and `tokenOut = _tokenPath[i + 1]`
		- required: `assetsByRouter[router][tokenIn] && assetsByRouter[router][tokenOut]` (both tokens listed in router)
		- if on final loop iteration:
			- assigned: `amountOutMin = _amountOutMin` and `receiver = _to`
		- assigned: `tokenInOut[0] = tokenIn` and `tokenInOut[1] = tokenOut`
		- required: `!guardOn[router] || guardOracle.isPermitted(tokenIn, tokenOut)` (TODO check `GuardOracle`)
		- invokes: `swapAmounts_ = INablaRouter(router).swapExactTokensForTokensWithoutPriceFeedUpdate(amountIn, amountOutMin, tokenInOut, receiver, _deadline` (no `PriceFeedUpdate` called since `_updatePriceFeeds` already called prior in wrapping functions)
			- see: `NablaRouter.swapExactTokensForTokensWithoutPriceFeedUpdate`
		- assigned: `amountIn = swapAmounts_[1]` (previous "out" is next iteration's "in")
	- assigned: `swapAmounts_[0] = _amountIn`
	- returns: `swapAmounts_`, an array of size two, containing the input and output amount

5. `NablaRouter.swapExactTokensForTokensWithoutPriceFeedUpdate`
	- invokes: `_swapExactTokensForTokens(_amountIn, _amountOutMin, _tokenInOut, _to, _deadline)`
		- see: `RouterCore._swapExactTokensForTokens`
	returns: `amounts_`, an array of size two, containing the input and output amount

6. `RouterCore._swapExactTokensForTokens`
	- required: `_amountIn > 0`
	- required: `block.timestamp <= _deadline`
	- required: `_to != address(0)` (again!)
	- invokes: `amountOut = _executeSwap(_amountIn, _tokenInOut, _to)`
		- see: `RouterCore._executeSwap`
	- required: `amountOut >= _amountOutMin`
	- assigned: `amounts_ = new uint256[](2)`, `amounts_[0] = _amountIn` and `amounts_[1] = amountOut`
	- emit: `Swap(msg.sender, _amountIn, amountOut, _tokenInOut[0], _tokenInOut[1], _to)`
	- returns: `amounts_`, an array of size two, containing the input and output amount

7. `RouterCore._executeSwap`
	- required: `_tokenInOut.length == 2` (again)
	- required: `_tokenInOut[0] != _tokenInOut[1]`
	- assigned: `fromToken = _tokenInOut[0]`, `toToken = _tokenInOut[1]`, `poolByAssetFromToken = poolByAsset[fromToken]` and `poolByAssetToToken = poolByAsset[toToken]`
	- required: `address(poolByAssetFromToken) != address(0)` and `address(poolByAssetToToken) != address(0)`
	- assigned: `priceOracleAdapter = oracleAdapter`
	- invokes: `tokenPriceFrom = priceOracleAdapter.getAssetPrice(fromToken)` and `tokenPriceTo = priceOracleAdapter.getAssetPrice(toToken)`
	- invokes: `IERC20(fromToken).safeTransferFrom(msg.sender, address(this), _amountIn)`
	- invokes: `effectiveAmountIn = poolByAssetFromToken.swapIntoFromRouter(_amountIn)`
		- see: `SwapPool.swapIntoFromRouter`
	- invokes: `rawOutAmount = _convertAmount(effectiveAmountIn, tokenPriceFrom, tokenPriceTo, poolByAssetFromToken.assetDecimals(), poolByAssetToToken.assetDecimals())`
		- see: `RouterCore._convertAmount`
	- invokes: `amountOut_ = poolByAssetToToken.swapOutFromRouter(rawOutAmount)`
		- see: `SwapPool.swapOutFromRouter`

8. `RouterCore._convertAmount`
	if `_fromDecimals > _toDecimals`:
		- `toAmount_ = (_fromAmount * _fromPrice) / _toPrice / (10 ** uint256(_fromDecimals - _toDecimals))`
	else:
		- `toAmount_ = (_fromAmount * _fromPrice * (10 ** uint256(_toDecimals - _fromDecimals))) / _toPrice`
	- returns: `toAmount_`, The equivalent amount of to tokens

9. `SwapPool.swapIntoFromRouter`
	- invokes: `effectiveAmount_ = _quoteSwapInto(_amount)`
		- see: `SwapPool._quoteSwapInto`
	- assigned: `reserve = reserve + effectiveAmount_` and `reserveWithSlippage = reserveWithSlippage + _amount`
	- invokes: `poolAsset.safeTransferFrom(msg.sender, address(this), _amount)`
	- returns `effectiveAmount_ Effective amount, incl. slippage (rewards or penalties)`

10. `SwapPool._quoteSwapInto`
	- assigned: `oldTotalLiabilities = totalLiabilities` and `oldReserve = reserve` (why??)
	- invokes: `effectiveAmount_ = slippageCurve.inverseHorizontal(oldReserve, oldTotalLiabilities, reserveWithSlippage + _amount, poolAssetDecimals);`
		- see: `slippageCurve.inverseHorizontal`
	- required: `(oldReserve + effectiveAmount_) <= (maxCoverageRatioForSwapIn * oldTotalLiabilities) / 100`
		- default: `uint256 public maxCoverageRatioForSwapIn = 200;`
		- can be set by owner, will emit `MaxCoverageRatioForSwapInSet(msg.sender, _maxCoverageRatio)`
	- returns: `effectiveAmount_`, Effective amount, incl. slippage (penalty or rewards)

11. `SwapPool.swapOutFromRouter`
	- assigned: `protocolFeeWithSlippage`, `effectiveLpFee`, `newReserve` and `backstopFee`
	- invokes: `(effectiveAmount_, protocolFeeWithSlippage, effectiveLpFee, newReserve, backstopFee) = _calculateSwapOutParameters(_amount)`
		- see: `SwapPool._calculateSwapOutParameters`
	- required: `effectiveAmount_ + protocolFeeWithSlippage <= poolAsset.balanceOf(address(this))`
	- assigned: `totalLiabilities = totalLiabilities + effectiveLpFee`, `reserve = newReserve` and `reserveWithSlippage -= effectiveAmount_ + protocolFeeWithSlippage`
	- emit: `ChargedSwapFees(effectiveLpFee, backstopFee, protocolFeeWithSlippage)`
	- if `effectiveAmount_ > 0`: `poolAsset.safeTransfer(msg.sender, effectiveAmount_)`
	- if `protocolFeeWithSlippage > 0`: `poolAsset.safeTransfer(protocolTreasury, protocolFeeWithSlippage)`
	- returns: `effectiveAmount_` actual withdraw amount

12. `SwapPool._calculateSwapOutParameters`
	- required: `_amount > 0`
	- assigned: `oldTotalLiabilities = totalLiabilities`, `oldReserveWithSlippage = reserveWithSlippage`
	- computed: `lpFee = (_amount * swapFeeConfig.lpFee) / 1_000_000`, `backstopFee_ = (_amount * swapFeeConfig.backstopFee) / 1_000_000` and `protocolFee = (_amount * swapFeeConfig.protocolFee) / 1_000_000`
	- computed: `reducedReserve = backstopFee_ + protocolFee < _amount ? reserve + backstopFee_ + protocolFee - _amount : reserve;`
	- invokes: `effectiveLpFee_ = slippageCurve.inverseDiagonal( reducedReserve, oldTotalLiabilities, oldReserveWithSlippage, poolAssetDecimals)`
		- see: `slippageCurve.inverseDiagonal`
	- if `effectiveLpFee_ > lpFee`: `effectiveLpFee_ = lpFee`
	- invokes: `reserveWithSlippageAfterAmountOut = slippageCurve.psi(reducedReserve + effectiveLpFee_, oldTotalLiabilities + effectiveLpFee_, poolAssetDecimals)`
		- see: `slippageCurve.psi`
	- if `reserveWithSlippageAfterAmountOut > oldReserveWithSlippage`: `reserveWithSlippageAfterAmountOut = oldReserveWithSlippage`
	- unchecked: `effectiveAmount_ = oldReserveWithSlippage - reserveWithSlippageAfterAmountOut`
	- assigned: `newReserve_ = reducedReserve + effectiveLpFee_ - protocolFee`
	- invokes: `newReserveWithSlippage = slippageCurve.psi( newReserve_, oldTotalLiabilities + effectiveLpFee_, poolAssetDecimals)`
		- see: `slippageCurve.psi`
	- unchecked: `protocolFeeWithSlippage_ = newReserveWithSlippage > reserveWithSlippageAfterAmountOut ? 0 : reserveWithSlippageAfterAmountOut - newReserveWithSlippage`
	- returns: 
		- `effectiveAmount_` Effective amount, incl. slippage and fees
		- `protocolFeeWithSlippage_` The protocol fee that is to be sent to the treasury
		- `effectiveLpFee_` The actual LP fee – totalLiabilities should be incremented by this value
		- `backstopFee_` The effective backstop fee



13. `OraclePriceUpdater._updatePriceFeeds`
14. `slippageCurve.inverseHorizontal`
15. `slippageCurve.inverseDiagonal`
16. `slippageCurve.psi`



Questions:

**General Functionality**:
1. Q: Why are there three swap methods (`swapExactTokensForTokens`, `swapEthForExactTokens`, and `swapExactTokensForEth`)
   A: ETH (unlike WETH) does not conform to the ERC20 standard, which means direct ETH swaps need additional handling to convert between ETH and ERC20 tokens. On L2 environments (where ETH is wrapped as WETH), the `swapExactTokensForTokens` method suffices since all swaps can be treated as ERC20-to-ERC20.

2. What does `_priceUpdateData` contain?
    - Is `_priceUpdateData` strictly metadata, such as price feed identifiers (e.g., asset addresses), or does it include actual price data? 
    - If it contains price data, does this assume the caller (e.g., an external solver) is trusted? Are there mechanisms, like proofs or signatures, to verify the authenticity and integrity of this data? 
    Update: as I understand it now, the `_priceUpdateData` comes from Hermes, Pyth's price update service. The data is provided in a binary format (hexadecimal string) and includes details such as:
		- Price feed ID (e.g. BTC/USD or ETH/USD).
		- Current price, confidence interval, and exponent.
		- Publish time and metadata (e.g. slot, proof availability time).

3. How does the `ISwapAdapter.swap` interface work without `_priceUpdateData`?
    - Since `_priceUpdateData` is not passed to the `ISwapAdapter.swap` interface, is it correct to assume that pathfinding and graph construction should be handled within the `NablaPortalSwapAdapter.swap`? 

4. Should the `getLimits` function use `_quoteSwapInto` and `_calculateSwapOutParameters`?
    - For determining sell-side and buy-side limits, would it be more consistent to use the results of `_quoteSwapInto` and `_calculateSwapOutParameters` respectively?


**Redundancy and Inconsistencies**
5. Why is `to_` checked only in `swapExactTokensForEth`?
    - The `to_` address is validated only in `swapExactTokensForEth`, but not in the other swap methods (`swapEthForExactTokens` and `swapTokensForExactTokens`). 
    - If `to_` is already validated in the nested `_swap` function, why is it checked explicitly here? Are these additional checks redundant? Could there be edge cases requiring this duplication?

6. Why is the `updateFee` only used in `swapEthForExactTokens`?
    - The `updateFee` returned by `_updatePriceFeeds` is only considered in `swapEthForExactTokens` but not in the other swap methods. Is this intentional? Shouldn't the update fee apply uniformly across all swap methods?

7. Q: Why does only `swapEthForExactTokens` have a `refundAmount`?
    - Is there a specific reason `refundAmount` is included only in `swapEthForExactTokens`? If ETH refunding is required here, shouldn't similar refund logic apply to other swap methods when input tokens exceed the required amount?
   A: Unlike ERC20 tokens, ETH cannot have fractional approvals, hence users must send an approximate amount. Other methods don't need this since ERC20 transfers are exact.

8. Why is `_tokenInOut[0] != _tokenInOut[1]` validated late?
    - The check `_tokenInOut[0] != _tokenInOut[1]` happens within `RouterCore._executeSwap` but is not performed earlier. Shouldn't this basic validation occur at the beginning of the transaction lifecycle to save gas in case of invalid input?


**Assumptions and Security**
9. Are oracle price checks after `_updatePriceFeeds` implicit?
    - The `swap` functions call `_updatePriceFeeds`, but there’s no explicit validation of the freshness or validity of the returned prices. Is this an implicit assumption, or should there be explicit checks for stale or invalid data?

10. Custody of funds during swaps:
    - In calls like `IERC20(_tokenPath[0]).safeTransferFrom(msg.sender, address(this), amount)`, the contract takes custody of funds before subsequent operations. Could the design avoid transferring funds to intermediate contracts (e.g., NablaRouter) and instead transfer directly to the `SwapPool` contract to save gas?


	- `NablaRouter.swapExactTokensForTokens` and `NablaRouterswapExactTokensForTokensWithoutPriceFeedUpdate`: both functions are marked external, meaning any EOA or contract can invoke them. This raises concern about potential for arbitrage exploitation:
		- Assume Token A's market price falls relative to Token B.
		- You could call swapExactTokensForTokensWithoutPriceFeedUpdate to swap Token A for Token B at the outdated, more favorable price.
		- After swapping to Token B, you could invoke swapExactTokensForTokens, which updates the price feeds, and swap Token B back to Token A, gaining more Token A than you initially held.




---


1. `NablaPortal.swapEthForExactTokens`

```
    /**
     * @notice Swap ETH for tokens using the NablaRouter
     * @param _amountIn         The amount of input ETH to swap
     * @param _amountOutMin     The minimum amount of output token that the user will accept
     * @param _tokenPath        Array of tokens to swap along the route (first token must be WETH)
     * @param _routerPath       Array of routers to use
     * @param _to               The recipient of the output tokens
     * @param _deadline         Unix timestamp after which the transaction will revert
     * @param _priceUpdateData  Array of price update data
     * @return amountOut_         Output amount of tokens
     * @dev By calling this function the price feed gets be updated (IPriceOracleAdapter.updatePriceFeeds)
     */
    function swapEthForExactTokens(
        uint256 _amountIn,
        uint256 _amountOutMin,
        address[] calldata _tokenPath,
        address[] calldata _routerPath,
        address _to,
        uint256 _deadline,
        bytes[] calldata _priceUpdateData
    ) external payable whenNotPaused nonReentrant returns (uint256 amountOut_) {
        require(_tokenPath.length >= 2, "NP:swapEthForExactTokens:INVALID_TOKEN_PATH_LENGTH");
        require(_tokenPath[0] == address(WETH), "NP:swapEthForExactTokens:INVALID_TOKEN_PATH_START");

        uint256 updateFee = _updatePriceFeeds(oracleAdapter, _priceUpdateData);

        require(msg.value >= _amountIn + updateFee, "NP:swapEthForExactTokens:INSUFFICIENT_ETH");

        WETH.deposit{value: _amountIn}();

        uint256[] memory swapAmounts = _swap(_amountIn, _amountOutMin, _tokenPath, _routerPath, _to, _deadline);

        uint256 refundAmount = msg.value - _amountIn - updateFee;

        emit EthForExactTokensSwapped(msg.sender, _to, _routerPath, _tokenPath, swapAmounts);

        if (refundAmount > 0) {
            (bool success,) = msg.sender.call{value: refundAmount}("");
            require(success, "NP:swapEthForExactTokens:REFUND_FAILED");
        }

        return swapAmounts[1];
    }
```


2. `NablaPortal.swapExactTokensForEth`

```
    /**
     * @notice Swap tokens using the NablaRouter and receive ETH
     * @param _amountIn         The amount of input tokens to swap
     * @param _amountOutMin     The minimum amount of ETH that the user will accept
     * @param _tokenPath        Array of tokens to swap along the route (last token must be WETH)
     * @param _routerPath       Array of routers to use
     * @param _to               The recipient of ETH
     * @param _deadline         Unix timestamp after which the transaction will revert
     * @param _priceUpdateData  Array of price update data
     * @return amountOut_         Output amount of ETH
     * @dev By calling this function the price feed gets be updated (IPriceOracleAdapter.updatePriceFeeds)
     */
    function swapExactTokensForEth(
        uint256 _amountIn,
        uint256 _amountOutMin,
        address[] calldata _tokenPath,
        address[] calldata _routerPath,
        address _to,
        uint256 _deadline,
        bytes[] calldata _priceUpdateData
    ) external payable whenNotPaused nonReentrant returns (uint256 amountOut_) {
        require(_to != address(0), "NP:swapExactTokensForEth:INVALID_TO_ADDRESS");
        require(_tokenPath.length >= 2, "NP:swapExactTokensForEth:INVALID_TOKEN_PATH_LENGTH");
        require(_tokenPath[_tokenPath.length - 1] == address(WETH), "NP:swapExactTokensForEth:INVALID_TOKEN_PATH_END");

        _updatePriceFeeds(oracleAdapter, _priceUpdateData);

        IERC20(_tokenPath[0]).safeTransferFrom(msg.sender, address(this), _amountIn);

        uint256[] memory swapAmounts =
            _swap(_amountIn, _amountOutMin, _tokenPath, _routerPath, address(this), _deadline);

        emit ExactTokensForEthSwapped(msg.sender, _to, _routerPath, _tokenPath, swapAmounts);

        WETH.withdraw(swapAmounts[1]);

        (bool success,) = _to.call{value: swapAmounts[1]}("");
        require(success, "NP:swapExactTokensForEth:ETH_TRANSFER_FAILED");

        return swapAmounts[1];
    }
```


3. `NablaPortal.swapExactTokensForTokens`

```
    /**
     * @notice Swap tokens using the NablaRouter
     * @param _amountIn         The amount of input tokens to swap
     * @param _amountOutMin     The minimum amount of output token that the user will accept
     * @param _tokenPath        Array of tokens to swap along the route
     * @param _routerPath       Array of routers to use
     * @param _to               The recipient of the output tokens
     * @param _deadline         Unix timestamp after which the transaction will revert
     * @param _priceUpdateData  Array of price update data
     * @return amountOut_         Output amount of tokens
     * @dev By calling this function the price feed gets be updated (IPriceOracleAdapter.updatePriceFeeds)
     */
    function swapExactTokensForTokens(
        uint256 _amountIn,
        uint256 _amountOutMin,
        address[] calldata _tokenPath,
        address[] calldata _routerPath,
        address _to,
        uint256 _deadline,
        bytes[] calldata _priceUpdateData
    ) external payable whenNotPaused nonReentrant returns (uint256 amountOut_) {
        require(_tokenPath.length >= 2, "NP:swapExactTokensForTokens:INVALID_TOKEN_PATH_LENGTH");

        _updatePriceFeeds(oracleAdapter, _priceUpdateData);

        IERC20(_tokenPath[0]).safeTransferFrom(msg.sender, address(this), _amountIn);

        uint256[] memory swapAmounts = _swap(_amountIn, _amountOutMin, _tokenPath, _routerPath, _to, _deadline);

        emit ExactTokensForTokensSwapped(msg.sender, _to, _routerPath, _tokenPath, swapAmounts);

        return swapAmounts[1];
    }
```


4. `NablaPortal._swap`

```
    /**
     * @notice Swap tokens using the NablaRouter
     * @param _amountIn         The amount of input tokens to swap
     * @param _amountOutMin     The minimum amount of output token that the user will accept
     * @param _tokenPath        Array of tokens to swap along the route
     * @param _routerPath       Array of routers to use
     * @param _to               The recipient of the output tokens
     * @param _deadline         Unix timestamp after which the transaction will revert
     * @return swapAmounts_      Array of size two, containing the input and output amount
     * @dev Before calling this function the price feed should be updated (IPriceOracleAdapter.updatePriceFeeds)
     */
    function _swap(
        uint256 _amountIn,
        uint256 _amountOutMin,
        address[] calldata _tokenPath,
        address[] calldata _routerPath,
        address _to,
        uint256 _deadline
    ) internal returns (uint256[] memory swapAmounts_) {
        _allowed(_amountIn);
        require(_to != address(0), "NP:_swap:INVALID_TO_ADDRESS");
        require(_tokenPath.length == _routerPath.length + 1, "NP:_swap:ROUTER_TOKEN_ARRAY_SIZE");

        uint256 amountIn = _amountIn;
        uint256 amountOutMin = 0;
        address receiver = address(this);

        address[] memory tokenInOut = new address[](2);

        for (uint256 i = 0; i < _routerPath.length; i++) {
            address router = _routerPath[i];
            address tokenIn = _tokenPath[i];
            address tokenOut = _tokenPath[i + 1];

            require(
                assetsByRouter[router][tokenIn] && assetsByRouter[router][tokenOut], "NP:_swap:INVALID_ROUTER_OR_TOKEN"
            );

            if (i == _routerPath.length - 1) {
                amountOutMin = _amountOutMin;
                receiver = _to;
            }

            tokenInOut[0] = tokenIn;
            tokenInOut[1] = tokenOut;

            require(!guardOn[router] || guardOracle.isPermitted(tokenIn, tokenOut), "NP:_swap:GUARD_ORACLE_REJECTED");

            swapAmounts_ = INablaRouter(router).swapExactTokensForTokensWithoutPriceFeedUpdate(
                amountIn, amountOutMin, tokenInOut, receiver, _deadline
            );

            amountIn = swapAmounts_[1];
        }

        swapAmounts_[0] = _amountIn;
    }
```

5. `NablaRouter.swapExactTokensForTokensWithoutPriceFeedUpdate`

```
    /**
     * @notice Swap some `_fromToken` tokens for `_toToken` tokens,
     *         ensures `_amountOutMin` and `_deadline`, sends funds to `_to` address, without updating price feeds
     * @notice `msg.sender` needs to grant the router contract a sufficient allowance beforehand
     * @param _amountIn     The amount of input tokens to swap
     * @param _amountOutMin The minimum amount of output token that the user will accept
     * @param _tokenInOut   Array of size two, indicating the in and out token
     * @param _to           The recipient of the output tokens
     * @param _deadline     Unix timestamp after which the transaction will revert
     * @return amounts_     Array of size two, containing the input and output amount
     * @dev Before calling this function the price feed should be updated (IPriceOracleAdapter.updatePriceFeeds)
     */
    function swapExactTokensForTokensWithoutPriceFeedUpdate(
        uint256 _amountIn,
        uint256 _amountOutMin,
        address[] calldata _tokenInOut,
        address _to,
        uint256 _deadline
    ) external whenNotPaused allowed(_amountIn) returns (uint256[] memory amounts_) {
        return _swapExactTokensForTokens(_amountIn, _amountOutMin, _tokenInOut, _to, _deadline);
    }

```


6. `RouterCore._swapExactTokensForTokens`

```

    /**
     * @notice Swap some `_fromToken` tokens for `_toToken` tokens,
     *         ensures `_amountOutMin` and `_deadline`, sends funds to `_to` address
     * @notice `msg.sender` needs to grant the router contract a sufficient allowance beforehand
     * @param _amountIn         The amount of input tokens to swap
     * @param _amountOutMin     The minimum amount of output token that the user will accept
     * @param _tokenInOut       Array of size two, indicating the in and out token
     * @param _to               The recipient of the output tokens
     * @param _deadline         Unix timestamp after which the transaction will revert
     * @return amounts_     Array of size two, containing the input and output amount
     */
    function _swapExactTokensForTokens(
        uint256 _amountIn,
        uint256 _amountOutMin,
        address[] calldata _tokenInOut,
        address _to,
        uint256 _deadline
    ) internal returns (uint256[] memory amounts_) {
        require(_amountIn > 0, "RC:_swapExactTokensForTokens:ZERO_AMOUNT");
        require(block.timestamp <= _deadline, "RC:_swapExactTokensForTokens:EXPIRED");
        require(_to != address(0), "RC:_swapExactTokensForTokens:INVALID_TO_ADDRESS");

        uint256 amountOut = _executeSwap(_amountIn, _tokenInOut, _to);

        require(amountOut >= _amountOutMin, "RC:_swapExactTokensForTokens:BELOW_MINIMUM");

        amounts_ = new uint256[](2);
        amounts_[0] = _amountIn;
        amounts_[1] = amountOut;

        emit Swap(msg.sender, _amountIn, amountOut, _tokenInOut[0], _tokenInOut[1], _to);
    }
```


7. `RouterCore._executeSwap`

```
    function _executeSwap(uint256 _amountIn, address[] calldata _tokenInOut, address _to)
        internal
        returns (uint256 amountOut_)
    {
        require(_tokenInOut.length == 2, "RC:_executeSwap:TOKEN_ARRAY_SIZE");
        require(_tokenInOut[0] != _tokenInOut[1], "RC:_executeSwap:TOKEN_ARRAY_DUPLICATE");

        address fromToken = _tokenInOut[0];
        address toToken = _tokenInOut[1];

        //Cache to save some gas
        ISwapPoolPermissioned poolByAssetFromToken = poolByAsset[fromToken];
        ISwapPoolPermissioned poolByAssetToToken = poolByAsset[toToken];

        require(address(poolByAssetFromToken) != address(0), "RC:_executeSwap:ASSET_NOT_REGISTERED");
        require(address(poolByAssetToToken) != address(0), "RC:_executeSwap:ASSET_NOT_REGISTERED");

        //Cache to save some gas
        IPriceOracleGetter priceOracleAdapter = oracleAdapter;

        uint256 tokenPriceFrom = priceOracleAdapter.getAssetPrice(fromToken);
        uint256 tokenPriceTo = priceOracleAdapter.getAssetPrice(toToken);

        // send user funds
        IERC20(fromToken).safeTransferFrom(msg.sender, address(this), _amountIn);

        // explicit block scoping to prevent "stack too deep" error when reading `_amountIn`
        {
            // user funds into swap pool
            uint256 effectiveAmountIn = poolByAssetFromToken.swapIntoFromRouter(_amountIn);

            uint256 rawOutAmount = _convertAmount(
                effectiveAmountIn,
                tokenPriceFrom,
                tokenPriceTo,
                poolByAssetFromToken.assetDecimals(),
                poolByAssetToToken.assetDecimals()
            );

            // send funds to user
            amountOut_ = poolByAssetToToken.swapOutFromRouter(rawOutAmount);
        }

        IERC20(toToken).safeTransfer(_to, amountOut_);
    }
```

8. `RouterCore._convertAmount`

```
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


9. `SwapPool.swapIntoFromRouter`

```
    /**
     * @notice Get called by Router to deposit an amount of pool asset
     * @notice Can only be called by Router
     * @param _amount The amount of pool tokens to swap into the pool
     * @return effectiveAmount_ Effective amount, incl. slippage (rewards or penalties)
     */
    function swapIntoFromRouter(uint256 _amount)
        external
        nonReentrant
        onlyRouter
        whenNotPaused
        returns (uint256 effectiveAmount_)
    {
        effectiveAmount_ = _quoteSwapInto(_amount);
        reserve = reserve + effectiveAmount_;
        reserveWithSlippage = reserveWithSlippage + _amount;

        poolAsset.safeTransferFrom(msg.sender, address(this), _amount);
    }
```


10. `SwapPool._quoteSwapInto`

```
    /**
     * @notice Complete calculation involved in a swap into operation
     * @param _amount The amount of pool tokens to swap into the pool
     * @return effectiveAmount_ Effective amount, incl. slippage (penalty or rewards)
     */
    function _quoteSwapInto(uint256 _amount) internal view returns (uint256 effectiveAmount_) {
        uint256 oldTotalLiabilities = totalLiabilities;
        uint256 oldReserve = reserve;

        effectiveAmount_ = slippageCurve.inverseHorizontal(
            oldReserve, oldTotalLiabilities, reserveWithSlippage + _amount, poolAssetDecimals
        );

        require(
            (oldReserve + effectiveAmount_) <= (maxCoverageRatioForSwapIn * oldTotalLiabilities) / 100,
            "SP:quoteSwapInto:EXCEEDS_MAX_COVERAGE_RATIO"
        );
    }
```


11. `NablaRouter.swapOutFromRouter`


```
    /**
     * @notice get called by Router to withdraw amount of pool asset
     * @notice Can only be called by Router
     * @param _amount The amount of pool asset to withdraw
     * @return effectiveAmount_ actual withdraw amount
     */
    function swapOutFromRouter(
        uint256 _amount
    )
        external
        nonReentrant
        onlyRouter
        whenNotPaused
        returns (uint256 effectiveAmount_)
    {
        uint256 protocolFeeWithSlippage;
        uint256 effectiveLpFee;
        uint256 newReserve;
        uint256 backstopFee;

        (
            effectiveAmount_,
            protocolFeeWithSlippage,
            effectiveLpFee,
            newReserve,
            backstopFee
        ) = _calculateSwapOutParameters(_amount);

        require(
            effectiveAmount_ + protocolFeeWithSlippage <=
                poolAsset.balanceOf(address(this)),
            "SP:swapOutFromRouter:OUT_OF_FUNDS"
        );

        totalLiabilities = totalLiabilities + effectiveLpFee;
        reserve = newReserve;
        reserveWithSlippage -= effectiveAmount_ + protocolFeeWithSlippage;

        emit ChargedSwapFees(
            effectiveLpFee,
            backstopFee,
            protocolFeeWithSlippage
        );

        if (effectiveAmount_ > 0) {
            poolAsset.safeTransfer(msg.sender, effectiveAmount_);
        }

        if (protocolFeeWithSlippage > 0) {
            poolAsset.safeTransfer(protocolTreasury, protocolFeeWithSlippage);
        }
    }
```


12. `_calculateSwapOutParameters`


```
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

See swap_adapter.md next
