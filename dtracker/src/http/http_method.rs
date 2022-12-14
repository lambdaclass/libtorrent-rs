use std::str::FromStr;

#[derive(Debug, PartialEq)]
pub enum HttpMethod {
    Get,
}

impl FromStr for HttpMethod {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "GET" => Ok(HttpMethod::Get),
            _ => Err(()),
        }
    }
}
