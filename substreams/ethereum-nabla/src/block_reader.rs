use crate::proto::sf::bstream;
use prost::Message;
use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use substreams_ethereum::pb::eth;
use tempfile::TempDir;

pub const BLOCKS_PER_FILE: u64 = 100;

pub struct BlockProcessor<'a> {
    merged_dir: &'a str,
    current_blocks: VecDeque<eth::v2::Block>,
    temp_dir: TempDir,
    next_expected_block: u64,
    final_block: u64,
}

impl<'a> BlockProcessor<'a> {
    pub fn new(merged_dir: &'a str) -> Result<Self, Box<dyn std::error::Error>> {
        // Read all `.dbin.zst` files in the merged directory
        let mut merged_files: Vec<PathBuf> = fs::read_dir(merged_dir)?
            .filter_map(|entry| entry.ok().map(|e| e.path()))
            .filter(|path| {
                path.to_str()
                    .map_or(false, |p| p.ends_with(".dbin.zst"))
            })
            .collect();

        if merged_files.is_empty() {
            return Err("No .dbin.zst files found in the merged directory".into());
        }

        merged_files.sort_unstable();
        let block_numbers: Vec<u64> = merged_files
            .iter()
            .map(|path| extract_block_number(path))
            .collect::<Result<Vec<_>, _>>()?;
        let first_block = *block_numbers.first().unwrap();
        let first_block_final_file = *block_numbers.last().unwrap();
        let final_block = first_block_final_file + BLOCKS_PER_FILE;

        block_numbers
            .iter()
            .enumerate()
            .skip(1)
            .all(|(i, &block)| block - block_numbers[i - 1] == BLOCKS_PER_FILE)
            .then(|| Ok(()))
            .unwrap_or_else(|| Err(format!("Non-continuous merged block file range.")))?;

        Ok(Self {
            merged_dir,
            current_blocks: VecDeque::new(),
            temp_dir: TempDir::new()?,
            next_expected_block: first_block,
            final_block,
        })
    }

    fn process_batch(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.clean_temp_dir()?;

        let block_no = self.next_expected_block;
        let merge_dir = self.merged_dir;
        let temp_path = self.temp_dir.path().to_str().unwrap();
        let block_range = format!("{}:{}", block_no, block_no + BLOCKS_PER_FILE - 1);

        [
            format!("fireeth tools unmerge-blocks {merge_dir} {temp_path} {block_range}"),
            format!("zstd -d --rm {temp_path}/*"),
            format!("dbin-to-bin {temp_path}/*"),
        ]
        .iter()
        .try_for_each(|command| execute(command))?;

        let bin_files = fs::read_dir(self.temp_dir.path())?
            .filter_map(|entry| entry.ok().map(|e| e.path()))
            .filter(|path| {
                path.extension()
                    .map_or(false, |e| e == "bin")
            });

        self.current_blocks
            .extend(bin_files.filter_map(|bin_file| {
                decode_bin_file(&bin_file)
                    .payload
                    .map(|payload| decode_ethereum_block(payload.value))
            }));

        Ok(())
    }

    fn clean_temp_dir(&self) -> Result<(), Box<dyn std::error::Error>> {
        fs::read_dir(self.temp_dir.path())?
            .filter_map(Result::ok)
            .try_for_each(|entry| fs::remove_file(entry.path()))
            .map_err(Into::into)
    }
}

impl<'a> Iterator for BlockProcessor<'a> {
    type Item = eth::v2::Block;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next_expected_block == self.final_block {
            return None;
        } else if self.current_blocks.is_empty() {
            self.process_batch()
                .expect(&format!("Error processing block '{}'", self.next_expected_block));
        }
        match self.current_blocks.pop_front() {
            Some(block) if block.number == self.next_expected_block => {
                self.next_expected_block += 1;
                Some(block)
            }
            Some(block) => {
                panic!(
                    "Block sequence mismatch: expected {}, got {}",
                    self.next_expected_block, block.number
                );
            }
            _ => {
                panic!("Expected block {}, but no block was found", self.next_expected_block);
            }
        }
    }
}

fn execute(command: &str) -> Result<(), Box<dyn std::error::Error>> {
    match Command::new("sh")
        .arg("-c")
        .arg(command)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()?
        .success()
    {
        true => Ok(()),
        false => Err(format!("Command failed: {}", command).into()),
    }
}

fn extract_block_number(merged_file: &Path) -> Result<u64, Box<dyn std::error::Error>> {
    merged_file
        .file_name()
        .ok_or("Failed to obtain file name")?
        .to_str()
        .ok_or("Failed to convert file path to string")?
        .split('.')
        .next()
        .ok_or("Failed to split file name to extract base name")?
        .parse::<u64>()
        .map_err(|_| "Failed to parse block number from file name".into())
}

fn decode_bin_file(bin_file: &Path) -> bstream::v1::Block {
    let file = fs::File::open(bin_file).expect("Failed to open processed `.bin` file");
    let block_data = std::io::Read::bytes(file)
        .collect::<Result<Vec<u8>, _>>()
        .expect("Failed to read `.bin` file");
    bstream::v1::Block::decode(&*block_data).expect(&format!("Failed to decode: {:?}", bin_file))
}

fn decode_ethereum_block(payload_value: Vec<u8>) -> eth::v2::Block {
    eth::v2::Block::decode(&*payload_value).expect("Failed to decode Ethereum block")
}
