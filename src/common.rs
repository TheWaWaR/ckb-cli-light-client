use std::str::FromStr;

use ckb_types::H256;

#[derive(Debug, Clone)]
pub struct HexH256(pub H256);

impl FromStr for HexH256 {
    type Err = anyhow::Error;
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let input = remove0x(input);
        Ok(HexH256(H256::from_str(input)?))
    }
}

pub fn remove0x(value: &str) -> &str {
    if let Some(stripped) = value.strip_prefix("0x") {
        stripped
    } else {
        value
    }
}
