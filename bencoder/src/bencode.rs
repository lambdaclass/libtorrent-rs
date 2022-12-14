use std::collections::BTreeMap;

#[derive(PartialEq, Debug, Clone)]
pub enum Bencode {
    BNumber(i64),
    BString(Vec<u8>),
    BList(Vec<Bencode>),
    BDict(BTreeMap<Vec<u8>, Bencode>),
}

#[derive(PartialEq, Debug)]
pub enum BencodeError {
    InvalidBencode,
    InvalidBencodeType,
    InvalidBencodeNumber,
    InvalidBencodeString,
    InvalidBencodeList,
    InvalidBencodeDict,
}

pub trait ToBencode {
    fn to_bencode(&self) -> Bencode;
}

impl ToBencode for String {
    fn to_bencode(&self) -> Bencode {
        Bencode::BString(self.as_bytes().to_vec())
    }
}

impl ToBencode for i64 {
    fn to_bencode(&self) -> Bencode {
        Bencode::BNumber(*self)
    }
}

impl ToBencode for u64 {
    fn to_bencode(&self) -> Bencode {
        Bencode::BNumber((*self) as i64)
    }
}

impl ToBencode for u32 {
    fn to_bencode(&self) -> Bencode {
        Bencode::BNumber((*self).into())
    }
}

impl ToBencode for u16 {
    fn to_bencode(&self) -> Bencode {
        Bencode::BNumber((*self).into())
    }
}

impl ToBencode for i32 {
    fn to_bencode(&self) -> Bencode {
        Bencode::BNumber((*self).into())
    }
}

impl ToBencode for Vec<u8> {
    fn to_bencode(&self) -> Bencode {
        Bencode::BString(self.clone())
    }
}

impl ToBencode for BTreeMap<Vec<u8>, Bencode> {
    fn to_bencode(&self) -> Bencode {
        Bencode::BDict(self.clone())
    }
}

impl<T: ToBencode> ToBencode for Vec<T> {
    fn to_bencode(&self) -> Bencode {
        Bencode::BList(self.iter().map(|s| s.to_bencode()).collect())
    }
}

impl Bencode {
    /// Parses a bencoded vec of bytes into a Bencode enum.
    ///
    /// # Example
    ///
    /// ```rust
    /// use bencoder::bencode::Bencode;
    ///
    /// // String
    /// let data = b"5:hello";
    /// let bencode = Bencode::decode(&data.to_vec()).unwrap();
    ///
    /// assert_eq!(bencode, Bencode::BString(b"hello".to_vec()));
    ///
    /// // Integer
    /// let data = b"i123e";
    /// let bencode = Bencode::decode(&data.to_vec()).unwrap();
    ///
    /// assert_eq!(bencode, Bencode::BNumber(123));
    /// ```
    pub fn decode(data: &[u8]) -> Result<Bencode, BencodeError> {
        let (bencode, _) = Bencode::do_decode(&data[0..])?;
        Ok(bencode)
    }

    fn do_decode(data: &[u8]) -> Result<(Bencode, usize), BencodeError> {
        if data.is_empty() {
            return Err(BencodeError::InvalidBencode);
        };
        match data[0] {
            b'i' => Bencode::decode_number(data),
            b'l' => Bencode::decode_list(data),
            b'd' => Bencode::decode_dict(data),
            b'0'..=b'9' => Bencode::decode_string(data),
            _ => Err(BencodeError::InvalidBencode),
        }
    }

    fn decode_string(data: &[u8]) -> Result<(Bencode, usize), BencodeError> {
        let mut i = 0;
        while data[i] != b':' {
            i += 1;
        }
        let length = &data[0..i];
        let length = match String::from_utf8(length.to_vec()) {
            Ok(s) => s,
            Err(_) => return Err(BencodeError::InvalidBencodeString),
        };
        let length = match length.parse::<i64>() {
            Ok(n) => n,
            Err(_) => return Err(BencodeError::InvalidBencodeString),
        };
        let mut i = i + 1;
        let mut string = Vec::new();
        for _ in 0..length {
            string.push(data[i]);
            i += 1;
        }
        Ok((Bencode::BString(string), i))
    }

    fn decode_number(data: &[u8]) -> Result<(Bencode, usize), BencodeError> {
        let mut i = 1;
        while data[i] != b'e' {
            i += 1;
        }
        let number = &data[1..i];
        let number = match String::from_utf8(number.to_vec()) {
            Ok(s) => s,
            Err(_) => return Err(BencodeError::InvalidBencodeNumber),
        };
        let number = match number.parse::<i64>() {
            Ok(n) => n,
            Err(_) => return Err(BencodeError::InvalidBencodeNumber),
        };
        Ok((Bencode::BNumber(number), i + 1))
    }

    fn decode_list(data: &[u8]) -> Result<(Bencode, usize), BencodeError> {
        let mut i = 1;
        let mut list = Vec::new();
        while data[i] != b'e' {
            let (value, size) = Bencode::do_decode(&data[i..])?;
            list.push(value);
            i += size;
        }
        Ok((Bencode::BList(list), i + 1))
    }

    fn decode_dict(data: &[u8]) -> Result<(Bencode, usize), BencodeError> {
        let mut i = 1;
        let mut dict = BTreeMap::new();
        while data[i] != b'e' {
            let (key, size) = Bencode::do_decode(&data[i..])?;
            i += size;
            let (value, size) = Bencode::do_decode(&data[i..])?;
            i += size;
            match key {
                Bencode::BString(key) => dict.insert(key, value),
                _ => return Err(BencodeError::InvalidBencodeDict),
            };
        }
        Ok((Bencode::BDict(dict), i + 1))
    }

    /// Encodes a Bencode enum into a bencoded vec of bytes.
    ///
    /// # Example
    ///
    /// ```rust
    /// use bencoder::bencode::Bencode;
    ///
    /// // String
    /// let data = String::from("spam");
    /// let encoded = Bencode::encode(&data);
    ///
    /// assert_eq!(encoded, b"4:spam");
    ///
    /// // Integer
    /// let data = 123;
    /// let encoded = Bencode::encode(&data);
    ///
    /// assert_eq!(encoded, b"i123e");
    /// ```
    pub fn encode(bencode: &dyn ToBencode) -> Vec<u8> {
        let bencode = bencode.to_bencode();
        Bencode::do_encode(bencode)
    }

    fn do_encode(bencode: Bencode) -> Vec<u8> {
        match bencode {
            Bencode::BNumber(n) => Bencode::encode_number(n),
            Bencode::BString(s) => Bencode::encode_string(s),
            Bencode::BList(l) => Bencode::encode_list(l),
            Bencode::BDict(d) => Bencode::encode_dict(d),
        }
    }

    fn encode_number(n: i64) -> Vec<u8> {
        let mut encoded = vec![b'i'];
        encoded.extend(n.to_string().into_bytes());
        encoded.push(b'e');
        encoded
    }

    fn encode_string(s: Vec<u8>) -> Vec<u8> {
        let mut encoded = Vec::new();
        encoded.extend(s.len().to_string().into_bytes());
        encoded.push(b':');
        encoded.extend(s);
        encoded
    }

    fn encode_list(l: Vec<Bencode>) -> Vec<u8> {
        let mut encoded = vec![b'l'];
        for bencode in l {
            encoded.extend(Bencode::do_encode(bencode));
        }
        encoded.push(b'e');
        encoded
    }

    fn encode_dict(d: BTreeMap<Vec<u8>, Bencode>) -> Vec<u8> {
        let mut encoded = vec![b'd'];
        for (key, value) in d {
            encoded.extend(Bencode::do_encode(Bencode::BString(key)));
            encoded.extend(Bencode::do_encode(value));
        }
        encoded.push(b'e');
        encoded
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_empty_data() {
        let data: &[u8; 0] = &[];
        assert_eq!(Bencode::decode(data), Err(BencodeError::InvalidBencode));
    }

    #[test]
    fn test_decode_string() {
        let data = b"4:spam";

        assert_eq!(
            Bencode::decode(data).unwrap(),
            Bencode::BString(b"spam".to_vec())
        );
    }

    #[test]
    fn test_decode_empty_string() {
        let data = b"0:";
        assert_eq!(
            Bencode::decode(data).unwrap(),
            Bencode::BString(b"".to_vec())
        );
    }

    #[test]
    fn test_decode_positive_integer() {
        let data = b"i3e";
        assert_eq!(Bencode::decode(data).unwrap(), Bencode::BNumber(3));
    }

    #[test]
    fn test_decode_negative_integer() {
        let data = b"i-3e";
        assert_eq!(Bencode::decode(data).unwrap(), Bencode::BNumber(-3));
    }

    #[test]
    fn test_decode_list() {
        let data = b"l4:spam4:eggse";
        assert_eq!(
            Bencode::decode(data).unwrap(),
            Bencode::BList(vec![
                Bencode::BString(b"spam".to_vec()),
                Bencode::BString(b"eggs".to_vec()),
            ])
        );
    }

    #[test]
    fn test_decode_empty_list() {
        let data = b"le";
        assert_eq!(Bencode::decode(data).unwrap(), Bencode::BList(vec![]));
    }

    #[test]
    fn test_decode_nested_list() {
        let data = b"ll3:fooee";
        assert_eq!(
            Bencode::decode(data).unwrap(),
            Bencode::BList(vec![Bencode::BList(vec![Bencode::BString(
                b"foo".to_vec()
            )])])
        );
    }

    #[test]
    fn test_decode_nested_with_2_lists() {
        let data = b"ll3:fooel3:baree";

        assert_eq!(
            Bencode::decode(data).unwrap(),
            Bencode::BList(vec![
                Bencode::BList(vec![Bencode::BString(b"foo".to_vec())]),
                Bencode::BList(vec![Bencode::BString(b"bar".to_vec())])
            ])
        );
    }

    #[test]
    fn test_decode_dict() {
        let data = b"d3:cow3:moo4:spam4:eggse";
        let mut dict = BTreeMap::new();
        dict.insert(b"cow".to_vec(), Bencode::BString(b"moo".to_vec()));
        dict.insert(b"spam".to_vec(), Bencode::BString(b"eggs".to_vec()));

        assert_eq!(Bencode::decode(data).unwrap(), Bencode::BDict(dict));
    }

    #[test]
    fn test_decode_dict_with_list() {
        let data = b"d4:spaml1:a1:bee";
        let mut dict = BTreeMap::new();
        dict.insert(
            b"spam".to_vec(),
            Bencode::BList(vec![
                Bencode::BString(b"a".to_vec()),
                Bencode::BString(b"b".to_vec()),
            ]),
        );

        assert_eq!(Bencode::decode(data).unwrap(), Bencode::BDict(dict));
    }

    #[test]
    fn test_decode_longer_dict() {
        let data =
            b"d9:publisher3:bob17:publisher-webpage15:www.example.com18:publisher.location4:homee";
        let mut dict = BTreeMap::new();
        dict.insert(b"publisher".to_vec(), Bencode::BString(b"bob".to_vec()));
        dict.insert(
            b"publisher-webpage".to_vec(),
            Bencode::BString(b"www.example.com".to_vec()),
        );
        dict.insert(
            b"publisher.location".to_vec(),
            Bencode::BString(b"home".to_vec()),
        );

        assert_eq!(Bencode::decode(data).unwrap(), Bencode::BDict(dict));
    }

    #[test]
    fn test_decode_empty_dict() {
        let data = b"de";
        let dict = BTreeMap::new();

        assert_eq!(Bencode::decode(data).unwrap(), Bencode::BDict(dict));
    }

    #[test]
    fn test_decode_dict_with_number_and_string() {
        let data = b"d3:fooi42e3:bar5:thinge";
        let mut dict = BTreeMap::new();
        dict.insert(b"bar".to_vec(), Bencode::BString(b"thing".to_vec()));
        dict.insert(b"foo".to_vec(), Bencode::BNumber(42));

        assert_eq!(Bencode::decode(data).unwrap(), Bencode::BDict(dict));
    }

    #[test]
    fn test_encode_string() {
        let data = String::from("spam");
        assert_eq!(Bencode::encode(&data), b"4:spam");
    }

    #[test]
    fn test_encode_empty_string() {
        let data = String::from("");
        assert_eq!(Bencode::encode(&data), b"0:");
    }

    #[test]
    fn test_encode_positive_integer() {
        let data = 3;
        assert_eq!(Bencode::encode(&data), b"i3e");
    }

    #[test]
    fn test_encode_negative_integer() {
        let data = -3;
        assert_eq!(Bencode::encode(&data), b"i-3e");
    }

    #[test]
    fn test_encode_vec_of_bytes() {
        let data = b"spam".to_vec();
        assert_eq!(Bencode::encode(&data), b"4:spam");
    }

    #[test]
    fn test_encode_vec_of_strings() {
        let data = vec![String::from("spam"), String::from("eggs")];
        assert_eq!(Bencode::encode(&data), b"l4:spam4:eggse");
    }

    #[test]
    fn test_encode_vec_of_integers() {
        let data = vec![1, 2, 3];
        assert_eq!(Bencode::encode(&data), b"li1ei2ei3ee");
    }

    #[test]
    fn test_encode_nested_list() {
        let data = vec![vec![String::from("spam"), String::from("eggs")]];
        assert_eq!(Bencode::encode(&data), b"ll4:spam4:eggsee");
    }

    #[test]
    fn test_encode_empty_list() {
        let data: Vec<String> = vec![];
        assert_eq!(Bencode::encode(&data), b"le");
    }
}
