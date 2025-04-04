#![no_main]

use coprocessor_sdk::sdk::Builder;
use crypto_bigint::{Zero, U256};
use trading_volumn_lib::prepare_test_receipts;

pico_sdk::entrypoint!(main);
pub fn main() {
    let test_receipts = prepare_test_receipts();

    let mut volume = U256::zero();
    let mut sdk: coprocessor_sdk::sdk::SDK = Builder::new()
        .with_receipts(test_receipts.receipts)
        .init(test_receipts.max_receipts as u32, 0, 0);

    if let Some(receipts) = sdk.receipts.clone() {
        for receipt in receipts {
            if !(receipt.fields[0].log_pos == receipt.fields[1].log_pos) {
                panic!("log field log pos mismatches");
            }

            let receipt_user_addr = receipt.fields[1].value;

            if test_receipts.expect_user_addr != receipt_user_addr {
                panic!("user address mismatches");
            }
            if receipt.fields[0].contract != test_receipts.expect_usdc_pool {
                panic!("usdc pool address mismatches");
            }

            if receipt.fields[0].topic != test_receipts.expect_event_swap {
                panic!("swap event topic mismatches");
            }

            if receipt.fields[1].topic != test_receipts.expect_event_swap {
                panic!("transfer event topic mismatches");
            }

            volume += receipt.fields[0].value;
        }
    }
    pico_sdk::io::commit_coprocessor_bytes(&mut sdk, &mut volume.to_be_bytes());
}
