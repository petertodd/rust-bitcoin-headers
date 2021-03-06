use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use util::hex::ToHex;
use bitcoin::header::BlockHeader;
use bitcoin;
use std::collections::HashMap;

pub fn start(
    block_headers_bytes: Arc<Mutex<Vec<u8>>>,
    host: String,
    username: String,
    password: Option<String>,
) {
    let start = Instant::now();

    let genesis_block_hash =
        String::from("000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f");
    let mut block_hash: String = genesis_block_hash.clone();
    let mut last_block: usize = 0;
    let mut min_block_hash: String = genesis_block_hash;

    let mut synced_height: usize = 0;
    let mut block_headers_map = HashMap::new();

    loop {
        let r = bitcoin::rpc::get_block_header(
            block_hash.clone(),
            host.clone(),
            username.clone(),
            password.clone(),
        );
        match r {
            Ok(block_header_rpc_response) => {
                let block_header_rpc: bitcoin::rpc::BlockHeaderRpc =
                    block_header_rpc_response.result;
                let height = block_header_rpc.height.clone() as usize;
                if last_block == 0 && height % 1000 == 0 {
                    println!(
                        "Block #{} with hash {} elapsed {} seconds",
                        height,
                        block_hash,
                        start.elapsed().as_secs()
                    );
                    synced_height = sync(
                        &mut block_headers_map,
                        &block_headers_bytes,
                        height,
                        synced_height,
                    );
                }

                let block_hash_option = block_header_rpc.nextblockhash.clone();
                let sleep = match block_hash_option {
                    Some(val) => {
                        block_hash = val;
                        let block_header = BlockHeader::from_block_header_rpc(block_header_rpc);

                        block_headers_map.insert(height, block_header);
                        let hash_hex = block_header.hash_be().to_hex();
                        if min_block_hash > hash_hex {
                            min_block_hash = hash_hex;
                            println!("Block #{} with hash {} is the min!", height, min_block_hash);
                        }

                        false
                    }
                    None => {
                        if height != last_block {
                            synced_height = sync(
                                &mut block_headers_map,
                                &block_headers_bytes,
                                height,
                                synced_height,
                            );
                            println!(
                                "Block #{} with hash {} synced_height {}",
                                height,
                                block_hash,
                                synced_height
                            );
                        }
                        last_block = height;
                        block_hash = block_headers_map
                            .get(&(height - 6))
                            .unwrap()
                            .hash_be()
                            .to_hex(); //going back 6 blocks to support reorgs

                        true
                    }
                };

                if sleep {
                    thread::sleep(Duration::from_secs(60));
                }
            }
            Err(e) => {
                println!("{:?} with hash {}", e, block_hash);
                thread::sleep(Duration::from_secs(10));
            }
        }
    }
}

fn sync(
    block_headers_map: &mut HashMap<usize, BlockHeader>,
    block_headers_bytes: &Arc<Mutex<Vec<u8>>>,
    height: usize,
    synced_height: usize,
) -> usize {
    let sync_to_option = height.checked_sub(6);
    match sync_to_option {
        Some(sync_to) => {
            let mut block_headers_bytes_lock = block_headers_bytes.lock().unwrap();
            for i in synced_height..sync_to {
                match i % 2016 {
                    0 => block_headers_bytes_lock
                        .extend(block_headers_map.remove(&i).unwrap().as_bytes().into_iter()),
                    _ => block_headers_bytes_lock.extend(
                        block_headers_map
                            .remove(&i)
                            .unwrap()
                            .as_compressed_bytes()
                            .into_iter(),
                    ),
                }
            }
            sync_to
        }
        None => synced_height,
    }
}
