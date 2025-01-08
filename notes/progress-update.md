# Progress Update

## 21-12-2024

- **Minimal VM Contract Setup**:
  - Set up the basics for the VM contract interfaces:
    - Created `NablaPortalSwapAdapter.sol` contract.
    - Added the manifest file.
    - Wrote basic tests in `NablaPortalSwapAdapter.t.sol`.
  - Got access to the source code of the contracts now. Haven't looked at it yet, but this was previously blocking for implementing contract interfaces and logic.

- **Substreams Work**:
  - Implemented some foundational features:
    1. Generated the ABI code for the NablaPortal contract.
    2. Used Substreams and Buf to generate protobuf files from `substreams.yaml`.
    3. Started working on the `map_components` function, but it doesn’t filter the correct components from transaction logs yet.
  - **Testing**:
    - Set up a template for `test_map_components`, focusing on integration/e2e tests rather than just unit tests.
    - Used `firehose-ethereum` to fetch block data from Arbitrum (block range `266195000–266196000`, spans contract creation block `266195245`).
    - Tested component mapping with actual data to verify functionality.
    - Stuck on generating protobuf files into Rust code here. There seems to be a path issue with `Timestamp.proto`, despite it being available in `/usr/local/include` :shrug:

Next Steps
1. Substreams:
   Goal: Implement `map_protocol_changes` to integrate data into final `BlockChanges` model.
   - For now, focus on `map_components` and `store_components` for substreams. Implementing other modules (like balance mapping) can best be delayed until after I understand the contracts and finished the interface logic.
   - Then, proceed with `getCapabilities`, `getLimits`, and `price` from the contract interfaces.



## 22-12-2024

- **Protobuf Setup Fix**:
    Resolved the protobuf installation issue. Yesterday I tried `prost_build`, but couldn’t get it to compile the .proto files to rust. After debugging, I discovered I had three different versions of `protoc` installed across different directories (`/bin/protoc`, `/usr/bin/protoc`, `/usr/local/bin/protoc`), each with its own set of `include` files... I nuked all of them and installed the latest version. After this cleanup, I successfully generated the Rust code from the `.proto` file, resolving this blocker

- **Firehose-Ethereum and .dbin**:
  - Encountered a problem with `fireeth` (firehose-ethereum), which is the recommended tool for testing in the [Substreams documentation](https://docs.substreams.dev/reference-material/indexer-reference/test-locally). It doesn't provide CLI tooling to decode the .dbin files.
    - I suspect this is because they want users to interact with their RPC endpoints continuously (as a paid service), instead of downloading and processing the data once. However, this is pure speculation on my part.
  - Since .dbin files can't be directly read with protobuf, I found relevant code the `streamingfast/dbin` repo to handle this (see [here](https://github.com/streamingfast/dbin/blob/develop/reader.go)). I wrote a small wrapper script in Go to convert `.dbin` files into `.bin` files, which can then be read using protobuf, but only in "raw decode" form. This means that the structure of the Ethereum Block message defined in [`proto/sf/ethereum/type/v2/type.proto`](https://github.com/streamingfast/firehose-ethereum/blob/develop/proto/sf/ethereum/type/v2/type.proto) doesn’t match the structure of the binary data directly.
  - After some soul searching exercises, in the form of trying to match the `protoc --decode_raw` output to message definitions in `.proto` files scattered around in various streamingfast repo's, I found that [`bstream.proto`](https://github.com/streamingfast/bstream/blob/develop/proto/sf/bstream/v1/bstream.proto#L69)'s `Block` message structure closely matched the binary data I was dealing with.
  - I compiled the `proto/sf/bstream/v1/bstream.proto` file into Rust code. Using this structure, I was able to read the binary data into this structured format.
  - Additionally, I was able to deserialize the payload (a `google.protobuf.Any`) into the `substreams_ethereum::eth::v2::Block` struct, which is the format for Ethereum block data that is known and loved :hug:

- **To sum up**:
  - Currently, I have only implemented `map_components` (which is still a work-in-progress), but we now have the ability to test and prototype with real data. None of the other workflows seem to have this local testing capability - but then again, I'm flying solo, so I’m unsure how others have approached this :shrug:
  - Tycho provides a testing suite for Substreams (see [here](https://docs.propellerheads.xyz/tycho/for-dexs/protocol-integration-sdk/indexing/general-integration-steps/4.-testing)), but this is for e2e testing. While this is an excellent tool for full system validation, having the ability to prototype and test with actual data locally is a critical need for a proper development cycle imo.
  - I also reached out to the Tycho team via Telegram regarding this integration, and they’ve indicated they’ll set up a communication channel for support. However, the group has not been established yet.



## 23-12-2024

- **Cleanup and Investigation**:
  - Did some general cleanup but didn't make major progress today. However, I dug into an issue regarding protobuf code generation.
  - I had been using `prost_build` in my `build.rs`, while the rest of the protobuf messages in the Tycho framework are generated via the `substreams protogen` command (`substreams.yaml --exclude-paths="sf/substreams,google"`, [docs](https://docs.propellerheads.xyz/tycho/for-dexs/protocol-integration-sdk/indexing/general-integration-steps/1.-setup#create-a-new-package)).

- **Switching to `substreams protogen`**:
  - Initially using `prost-build = "0.13.4"`, but the Tycho framework workspace uses `prost = "0.11"` and `prost-types = "0.12.3"`.
  - Decided to shift to `substreams protogen` to align with the framework to reduce future headaches:
    - **The Good**:
      - Generated protobuf messages are consistent with the rest of the Tycho framework, reducing discrepancies across the codebase.
      - No need to provide `.proto` files like `google/protobuf/timestamp.proto`.
      - The `mod.rs` files are programmatically generated (thanks to `buf`). Metaprogramming 🥰
    - **The Bad**:
      - `substreams protogen` uses `buf`, a wrapper around `protoc`, which isn’t directly compatible with `prost-build` unless files are generated first.
      - `prost-build` directly uses `protoc`, while `buf` manages Protobuf tools and plugins.
    - **The Ugly**:
      - The `prost` ecosystem components (`prost`, `prost-types`, `prost-build`) need to be in sync. Mismatched versions cause issues like compilation errors, runtime panics, and trait incompatibilities (e.g., `prost::Message` changes between versions).
      - Mismatched `prost` versions across the project cause issues with trait implementations and runtime compatibility. Specifically, `prost = "0.11"` used by the framework is incompatible with the newer code generated by `substreams protogen` (which expects `prost = "0.13"`).

- **Protobuf Compatibility**:
  - `prost 0.13` introduced breaking changes, especially with `prost::Message` and handling types like `Timestamp`. Code generated for `prost 0.13` doesn’t work with earlier versions (e.g., `prost = "0.11"`).
  - To fix this, either:
    1. **Align Prost Versions**: Update the framework to use `prost = "0.13.4"` and `prost-types = "0.13.4"` to resolve trait mismatches. However, this introduces potential breaking changes elsewhere in the codebase (i.e. other protocol integrations).
    2. **Generate Code for Older Prost Versions**: Ensure `substreams protogen` generates code compatible with `prost = "0.11"` by configuring it to use an older `protoc-gen-prost` plugin.  


I've contacted the Propellerheads team on the matter to inquire:
    ```
    Hello Propellerheads team,

    I'm Michael, and I'm currently trying to integrate Nabla into the Tycho Protocol SDK. I'm working on the indexer (Substreams) part and would like to test things as I go along. In the documentation under testing, it says to reach out to you guys to get access to PropellerHeads' private PyPi repository, so here I am.

    I also have an issue with the protobuf setup. I used a custom build.rs, which worked. Then i tried to align with the framework's use of `substreams protogen`, but since it uses an older version of `prost` (0.11) I get `the trait bound Timestamp: prost::Message is not satisfied`. One solution I find online is (logically) to upgrade to 0.13, but it seems like a really bad idea to mix different versions of prost going forward. Have you faced this issue before, and/or do you have any recommendations on how to deal with this?

    As a sidenote, I believe that the `prost` ecosystem components (`prost`, `prost-types`, `prost-build`) need to be in sync. While the latter is currently not used by the framework, I noticed that tycho-protocol-sdk/substreams uses `prost = "0.11"` and `prost-types = "0.12.3"`.
    ```

They responded:
    ```
    Hey, nice to meet you 👋

    I contacted our AWS admin to create some PyPi credentials for you. I'll send them to you as soon as I have them.

    Regarding the prost issue, I’m unfortunately not familiar with this one. If updating to version 0.13 is necessary, you can specify this version in your Cargo.toml. 
    This isn’t the first time we’ve encountered issues with workspace versioning. Since each Substreams module operates as a completely independent entity, we are considering removing it entirely.
    ```

**To Sum Up**:
  - Current setup allows decoding binary data into `bstream.v1.Block` structure and parsing payloads via `substreams_ethereum::pb::eth::v2::Block::decode`.
  - Based on the message exchange with the Tycho team, the need to address compatibility and alignment issues with the Tycho framework is minimal, as they are considering deprecating the use of substreams. We can just use `prost 0.13`, which I currently only use for testing anyway.



## 06-01-2025

- **Swap Workflow Investigation**:
  - Analyzed the entire swap workflow of `NablaRouter`, gaining a detailed understanding of the requirements for implementing `price`, `swap`, `getCapabilities` and `getLimits` functionality.  
  - Also reviewed , identifying requirements for their implementation. This remains a work-in-progress and will continue tomorrow.  
  - I have more documentation I can share (analysis, questions, ideas on implementation). A temporary Git branch seems ideal for this, as it offers version control without impacting permanent history.

Had some unforeseen personal matters come up, delaying the finalization of today's work. Will prioritize completing the pending tasks tomorrow.

**Next Steps**:
- Finalize swap workflow requirements and implement related features in the `NablaPortalSwapAdapter` (for as far as possible, since price via pyth remains somewhat of an open question).
- Resume Substreams work, particularly integration of the firehose merged blocks data for prototype testing.



## 07-01-2025  

### Smart Contract Interface  
- Reassessed `NablaRouter`'s role, concluding it is more akin to a multi-asset pool in conventional terms than Nabla’s `SwapPool`. This is important for the Tycho integration.  
    - Given that:  
        - `NablaPortal`'s swapping method requires a `tokenPath`, an associated `routerPath`, as well as `priceUpdateData`.  
        - `NablaPortal`'s `quoteSwapExactTokensForTokens` method requires `priceUpdateData`.  
        - All `ISwapAdapter` methods to be implemented take a `poolId` parameter as input, whereas `price` and `swap` methods also take `sellToken` and `buyToken` addresses as inputs.  
        - `NablaRouter` is gated (we cannot invoke functions on it directly as an EOA).  
    - From this, it follows that:  
        1. Invoking `price` on the `NablaPortalSwapAdapter` requires invoking `NablaPortal.quoteSwapExactTokensForTokens`, which is a **payable** method.  
        2. The `priceUpdateData` must be provided by the caller (solver) in Nabla. Since this is not part of the `ISwapAdapter`, it needs to be implemented as part of the interface by us.  
        3. Regarding `ISwapAdapter.swap`, if we consider all possible asset pairs as swappable (including multihop), we need to construct the graph and perform the pathfinding, which should be left as an exercise to the solver.  
        4. If, instead, we consider only single-router swaps and use the router address as a `poolId`, then `ISwapAdapter` methods can actually use this input parameter, and solvers can construct their multihop paths.  
    - Documented this in [swap_adapter.md](./swap_adapter.md).  
- Cleaned and pushed a template for the [NablaPortalSwapAdapter](https://github.com/NablaFinance/tycho-protocol-sdk/tree/feat/nabla_interfaces/evm/src/nabla).  

### Substreams  
- Consolidated the custom `prost-build` setup for decoding `bstream` (firehose) blocks. Based on message exchange with the Propellerheads team, decided not to try consolidating the build here:  
    - They confirmed that using version `0.13` was fine to use if we needed it.  
    - They are considering removing the Substreams module entirely, so there is no value in attempting a unified build setup at this point.  
- Cleaned up code related to integrating with the `substreams protogen` setup.  
- The Go module for handling `dbin` to `bin` conversion is not included. These `bstream` block data files are used solely for testing/prototyping. If integration into final tests is necessary, it can be converted into a Rust module or documented for future use.  
    - Did not upload the referenced test file.  
- Cleaned-up branch containing the template for `map_components` logic and prototype test workflows was [pushed](https://github.com/NablaFinance/tycho-protocol-sdk/tree/feat/nabla_substream/substreams/ethereum-nabla).  

### Documentation  
- Uploaded markdown documentation to a [separate branch](https://github.com/NablaFinance/tycho-protocol-sdk/tree/docs/nabla/notes).  

## Next Steps  
- Continue Substreams work.  
- Tycho ISwapAdapter methods implementation requires input on:  
    - Resolving oracle pricing data.  
    - Whether or not to treat/present Nabla routers as pools in a conventional DEX (assigning them `poolIds`).  

