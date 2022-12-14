/// Takes an hex string and applies Percent-Encoding, returning an encoded version.
pub fn encode(hex_string: &str) -> String {
    if hex_string.is_empty() {
        return hex_string.to_string();
    }
    let mut encoded_hex_string = hex_string
        .chars()
        .collect::<Vec<char>>()
        .chunks(2)
        .map(|c| c.iter().collect::<String>())
        .collect::<Vec<String>>()
        .join("%");
    encoded_hex_string.insert(0, '%');
    encoded_hex_string
}

/// Takes an encoded string and decodes it.
pub fn decode(hex_str: &str) -> String {
    let mut out = Vec::new();
    let mut iter = hex_str.chars();

    while let Some(c) = iter.next() {
        match c {
            '%' => {
                let c1 = iter.next().unwrap();
                let c2 = iter.next().unwrap();
                out.push(c1.to_string().to_lowercase());
                out.push(c2.to_string().to_lowercase());
            }
            _ => out.push(format!("{:x}", c.to_string().as_bytes()[0])),
        }
    }

    out.join("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_empty_string_returns_empty_string() {
        assert_eq!("", encode(""));
    }

    #[test]
    fn test_encode_info_hash() {
        let info_hash = "2c6b6858d61da9543d4231a71db4b1c9264b0685";
        let expected_info_hash = "%2c%6b%68%58%d6%1d%a9%54%3d%42%31%a7%1d%b4%b1%c9%26%4b%06%85";

        assert_eq!(expected_info_hash, encode(info_hash));
    }

    #[test]
    fn test_hex_decoder() {
        let infohash = "%124Vx%9A%BC%DE%F1%23Eg%89%AB%CD%EF%124Vx%9A";
        let infohash_bytes = super::decode(infohash);
        assert_eq!(infohash_bytes, "123456789abcdef123456789abcdef123456789a");
    }
}
