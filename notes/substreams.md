
### Testing
For substream testing purposes, I decided to download some blockchain data using firehose
1. Test Substreams Locally: https://docs.substreams.dev/reference-material/indexer-reference/test-locally
2. Download firehose CLI: https://github.com/streamingfast/firehose-ethereum/releases/
3. Unpack: `sudo tar -xzvf /home/zarathustra/Downloads/firehose-ethereum_linux_x86_64.tar.gz -C /usr/local/bin/`
4. Get Arbitrum Firehose API key and JWT token here: https://pinax.network/en/chain/arbone
5. set these 
    - `export FIREHOSE_API_KEY=<API-KEY>`  (or `set -x FIREHOSE_API_KEY <API-KEY>`)
    - `export FIREHOSE_API_TOKEN=<JWT-TOKEN>`  (or `set -x FIREHOSE_API_TOKEN <JWT-TOKEN>`)
6. Find deploy block of NablaPortal contract on Arbitrum: https://arbiscan.io/address/0xcB94Eee869a2041F3B44da423F78134aFb6b676B
   NOTE: not all blocks are valid starting points it appears, hence I rounded numbers to closest thousand
7. `fireeth tools download-from-firehose arbone.firehose.pinax.network:443 266195000:266196000 ./firehose-data/storage/merged-blocks`
    - verify `fireeth tools print merged-blocks firehose-data/storage/merged-blocks 0266195200`
    - unmerge: `fireeth tools unmerge-blocks ./firehose-data/storage/merged-blocks ./firehose-data/storage/unmerged-blocks 266195200:266195300`

8. install substreams 
   - `git clone https://github.com/streamingfast/substreams`
   - `cd substreams`
   - `go install -v ./cmd/substreams`
   - `sudo mv ~/go/bin/substreams /usr/local/bin/`
9. Install buf
   - Download from here https://github.com/bufbuild/buf/releases
   - unpack and copy binary `tar -xzvf /home/zarathustra/Downloads/buf-Linux-x86_64.tar.gz -C ~/Downloads && sudo cp ~/Downloads/buf/bin/buf /usr/local/bin/`
10. `substreams protogen substreams.yaml --exclude-paths="sf/substreams,google"` 

- need to get this: https://github.com/streamingfast/firehose-ethereum/blob/develop/proto/sf/ethereum/type/v2/type.proto
- https://github.com/protocolbuffers/protobuf/releases/
- `fireeth tools print merged-blocks firehose-data/storage/merged-blocks 0266195200`

- https://github.com/streamingfast/firehose-core/blob/develop/proto/testdata/override/sf/ethereum/type/v2/type.proto



## Generating rust code from proto

https://github.com/tokio-rs/prost
https://github.com/neoeinstein/protoc-gen-prost

- install protoc (I used v29.2). Ensure to move the binary to `usr/local/bin` as well as the `include/` folder to `usr/local/`
- I used https://docs.rs/prost-build/latest/prost_build/ from here to create the rust code from the .proto file


## Dealing with merged-block files
https://github.com/streamingfast/dbin/

Dealing with the merged .dbin.zstd files
- uncompress: `zstd -d ./firehose-data/storage/unmerged-blocks/0266195200-82f04336daa7db65-ea2d79006c783eac-266195000-extracted.dbin.zst -o firehose-data/storage/unmerged-blocks/0266195200-82f04336daa7db65-ea2d79006c783eac-266195000-extracted.dbin`
- I wrote `dbin_writer.go` that converts the `.dbin` to a `.bin`
- verified this can now be read by protobuf via `protoc --decode_raw < firehose-ethereum/proto/data/0266195200.bin`
- could still not read the Block messages. Inspected the .proto file and compared with the --decode_raw output

```
zarathustra@Sils-Maria ~/P/N/dbin_reader> protoc --decode_raw < ../firehose-data/storage/merged-blocks/0266195200.bin | head -n 20
1: 266195200
2: "f30da570fc88c2ccae0e4f17c1a72d7a2dd1bab9b7d89e1382f04336daa7db65"
3: "6cbc2ff4855dc90bd44529b539355ead934fa7827e0ee790ea2d79006c783eac"
4 {
  1: 1729530123
}
5: 266195000
10: 266195199
11 {
  1: "type.googleapis.com/sf.ethereum.type.v2.Block"
  2 {
    1: 3
    2: "\363\r\245p\374\210\302\314\256\016O\027\301\247-z-\321\272\271\267\330\236\023\202\360C6\332\247\333e"
    3: 266195200
    4: 3233
    5 {
      1: "l\274/\364\205]\311\013\324E)\26595^\255\223O\247\202~\016\347\220\352-y\000lx>\254"
      2: "\035\314M\350\336\307]z\253\205\265g\266\314\324\032\323\022E\033\224\212t\023\360\241B\375@\324\223G"
      3: "\244\260\000\000\000\000\000\000\000\000\000sequencer"
      4: "w\346\245\260O\215\025o\365kP\323\037\303\0234\336\276K\202\021\377\"\356S\0036\361\341m\317N"
```

it appears the 11th field is the Block message data, as per `proto/sf/ethereum/type/v2/type.proto`.

did some more soul searching. Found this
- https://github.com/streamingfast/bstream/blob/develop/proto/sf/bstream/v1/bstream.proto#L69

```
message Block {
  uint64 number = 1;
  string id = 2;
  string parent_id = 3;
  google.protobuf.Timestamp timestamp = 4;
  uint64 lib_num = 5;

  Protocol payload_kind = 6 [deprecated=true];
  int32 payload_version = 7 [deprecated=true];
  bytes payload_buffer = 8 [deprecated=true];
  uint64 head_num = 9 [deprecated=true];

  uint64 parent_num = 10;
  google.protobuf.Any payload = 11;
}
```

This seems to be the actual message I'm looking at. Notice how fields 6 - 9 are deprecated, and also absent in the --decode_raw output.
The 11th field is a `payload`, which appears to consist of a message 

