//! `generate-genesis` subcommand - generates an empty genesis block and saves it to a file.

use std::{fs::File, io::Write, path::PathBuf, sync::Arc};

use abscissa_core::{Command, Runnable};
use chrono::Utc;
use clap::Parser;

use zebra_chain::{
    block::{Block, Hash, Header, Height},
    fmt::HexDebug,
    serialization::ZcashSerialize,
    transaction::Transaction,
    transparent::Input,
    work::{difficulty::CompactDifficulty, equihash::Solution},
};

/// Generate an empty genesis block and save it to a file
#[derive(Command, Debug, Parser)]
pub struct GenerateGenesisCmd {
    /// The file path to write the generated genesis block to.
    #[clap(help = "The file path to write the genesis block JSON to")]
    output_file: PathBuf,
}

impl Runnable for GenerateGenesisCmd {
    /// Generate and save an empty genesis block.
    #[allow(clippy::print_stdout)]
    fn run(&self) {
        let genesis_block = Self::create_empty_genesis_block();

        match Self::save_to_json(&genesis_block, &self.output_file) {
            Ok(()) => {
                println!("Successfully generated genesis block at: {}", self.output_file.display());
            }
            Err(error) => {
                eprintln!("Failed to write genesis block: {error}");
                std::process::exit(1);
            }
        }
    }
}

impl GenerateGenesisCmd {
    /// Create an empty genesis block with minimal valid content.
    fn create_empty_genesis_block() -> Block {
        // Use regtest difficulty (easiest valid difficulty): 0x200f0f0f in display order
        // This is the same difficulty as the regtest genesis block
        let difficulty_threshold = CompactDifficulty::from_bytes_in_display_order(&[0x20, 0x0f, 0x0f, 0x0f])
            .expect("regtest difficulty is valid");

        // Create an empty genesis block header
        let header = Header {
            version: 4,
            previous_block_hash: Hash([0; 32]),
            merkle_root: zebra_chain::block::merkle::Root([0; 32]),
            commitment_bytes: HexDebug([0; 32]),
            time: Utc::now(),
            difficulty_threshold,
            nonce: HexDebug([0; 32]),
            solution: Solution::for_proposal(),
        };

        // Create a minimal coinbase transaction for height 1
        // (Height 0 has special genesis validation requirements, so we use height 1 for simplicity)
        let block_height = Height(1);
        let coinbase_input = Input::new_coinbase(block_height, vec![], None);

        // Create a minimal coinbase transaction with no outputs
        let coinbase_transaction = Transaction::V4 {
            inputs: vec![coinbase_input],
            outputs: vec![],
            lock_time: zebra_chain::transaction::LockTime::min_lock_time_timestamp(),
            expiry_height: block_height,
            joinsplit_data: None,
            sapling_shielded_data: None,
        };

        Block {
            header: Arc::new(header),
            transactions: vec![Arc::new(coinbase_transaction)],
        }
    }

    /// Save the genesis block to a JSON file.
    fn save_to_json(block: &Block, output_file: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        // Serialize the block to bytes using Zcash serialization
        let mut block_bytes = Vec::new();
        block.zcash_serialize(&mut block_bytes)?;

        // Convert to hex
        let block_hex = hex::encode(&block_bytes);

        // Extract block height from coinbase
        let height = block.coinbase_height().map(|h| h.0).unwrap_or(0);

        // Create a JSON structure with metadata and hex data
        let json_output = serde_json::json!({
            "block_hex": block_hex,
            "block_size_bytes": block_bytes.len(),
            "block_hash": block.hash().to_string(),
            "height": height,
            "version": block.header.version,
            "time": block.header.time.to_rfc3339(),
            "transactions": block.transactions.len(),
        });

        // Write to file
        let mut file = File::create(output_file)?;
        file.write_all(serde_json::to_string_pretty(&json_output)?.as_bytes())?;

        Ok(())
    }
}
